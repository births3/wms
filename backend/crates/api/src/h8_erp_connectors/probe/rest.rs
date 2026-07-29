use std::{
    collections::HashSet,
    io::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    process::{Command, Stdio},
};

use wms_domain::sanitize_error_summary;

const REST_PROBE_ALLOWED_ENDPOINTS_ENV: &str = "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS";
const REST_PROBE_ALLOW_LOCAL_HTTP_ENV: &str = "WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP";

pub(super) async fn rest_url_probe(url: &str, bearer_token: Option<String>) -> Result<(), String> {
    let health_url = validate_rest_probe_endpoint(url)?;
    tokio::task::spawn_blocking(move || {
        rest_health_probe_blocking(&health_url, bearer_token.as_deref())
    })
    .await
    .map_err(|_| "REST health probe task failed".to_string())?
}

pub(super) fn validate_rest_probe_endpoint(url: &str) -> Result<String, String> {
    let uri = url
        .trim()
        .parse::<axum::http::Uri>()
        .map_err(|_| "invalid REST base URL".to_string())?;
    let scheme = uri
        .scheme_str()
        .ok_or_else(|| "REST URL scheme missing".to_string())?;
    let authority = uri
        .authority()
        .ok_or_else(|| "REST URL host missing".to_string())?;
    if authority.as_str().contains('@') {
        return Err("REST URL credentials are not allowed".into());
    }
    if uri
        .path_and_query()
        .and_then(|path_and_query| path_and_query.query())
        .is_some()
    {
        return Err("REST URL query is not allowed".into());
    }
    let host = normalize_host(authority.host()).map_err(|_| "REST probe host is unsafe")?;
    let port = authority.port_u16().unwrap_or(match scheme {
        "https" => 443,
        "http" => 80,
        _ => return Err("unsupported REST URL scheme".into()),
    });
    match scheme {
        "https" => {
            validate_nonlocal_https_host(&host)
                .map_err(|_| "REST probe host is unsafe".to_string())?;
        }
        "http"
            if matches!(host.as_str(), "127.0.0.1" | "localhost") && local_http_probe_allowed() => {
        }
        "http" => return Err("untrusted non-https base for probe".into()),
        _ => return Err("unsupported REST URL scheme".into()),
    }
    let endpoint = normalize_endpoint(&host, port);
    if !rest_probe_allowed_endpoints()?.contains(&endpoint) {
        return Err("REST probe endpoint not allowed".into());
    }
    Ok(format!("{}/healthz", uri.to_string().trim_end_matches('/')))
}

fn local_http_probe_allowed() -> bool {
    matches!(
        std::env::var(REST_PROBE_ALLOW_LOCAL_HTTP_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

fn normalize_endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn rest_probe_allowed_endpoints() -> Result<HashSet<String>, String> {
    let raw = std::env::var(REST_PROBE_ALLOWED_ENDPOINTS_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "REST probe endpoint allowlist missing".to_string())?;
    let mut endpoints = HashSet::new();
    for entry in raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let authority = entry
            .parse::<axum::http::uri::Authority>()
            .map_err(|_| "REST probe endpoint allowlist invalid".to_string())?;
        if authority.as_str().contains('@') {
            return Err("REST probe endpoint allowlist invalid".into());
        }
        let port = authority
            .port_u16()
            .ok_or_else(|| "REST probe endpoint allowlist invalid".to_string())?;
        let host = normalize_host(authority.host())
            .map_err(|_| "REST probe endpoint allowlist invalid".to_string())?;
        if !matches!(host.as_str(), "localhost" | "127.0.0.1") {
            validate_nonlocal_https_host(&host)
                .map_err(|_| "REST probe endpoint allowlist invalid".to_string())?;
        }
        endpoints.insert(normalize_endpoint(&host, port));
    }
    if endpoints.is_empty() {
        return Err("REST probe endpoint allowlist missing".into());
    }
    Ok(endpoints)
}

fn normalize_host(raw: &str) -> Result<String, ()> {
    let host = raw
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty()
        || host.contains(['/', '\\', '@', '%'])
        || host.chars().any(char::is_whitespace)
    {
        return Err(());
    }
    Ok(host)
}

fn validate_nonlocal_https_host(host: &str) -> Result<(), ()> {
    if host == "localhost" || host.ends_with(".localhost") {
        return Err(());
    }
    if let Ok(address) = host.parse::<IpAddr>() {
        return match address {
            IpAddr::V4(address) => validate_explicit_ipv4(address),
            IpAddr::V6(address) => validate_explicit_ipv6(address),
        };
    }
    if looks_like_noncanonical_ip(host) || !is_valid_dns_host(host) {
        return Err(());
    }
    Ok(())
}

fn validate_explicit_ipv4(address: Ipv4Addr) -> Result<(), ()> {
    if address.is_unspecified()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || address.octets() == [255, 255, 255, 255]
    {
        return Err(());
    }
    Ok(())
}

fn validate_explicit_ipv6(address: Ipv6Addr) -> Result<(), ()> {
    if address.to_ipv4_mapped().is_some()
        || address.is_unspecified()
        || address.is_loopback()
        || address.is_unicast_link_local()
        || address.is_multicast()
    {
        return Err(());
    }
    Ok(())
}

fn looks_like_noncanonical_ip(host: &str) -> bool {
    host.chars()
        .all(|character| character.is_ascii_digit() || character == '.')
        || host.strip_prefix("0x").is_some_and(is_nonempty_ascii_hex)
        || (host.contains('.')
            && host.split('.').all(|label| {
                label.chars().all(|character| character.is_ascii_digit())
                    || label.strip_prefix("0x").is_some_and(is_nonempty_ascii_hex)
            }))
}

fn is_nonempty_ascii_hex(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_valid_dns_host(host: &str) -> bool {
    host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && label
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
                && label
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
        })
}

fn rest_health_probe_blocking(url: &str, bearer_token: Option<&str>) -> Result<(), String> {
    if bearer_token.is_some_and(|token| token.contains(['\r', '\n'])) {
        return Err("invalid bearer secret".into());
    }
    let mut child = rest_curl_command(url, bearer_token.is_some())
        .spawn()
        .map_err(|error| format!("REST curl unavailable: {error}"))?;
    if let Some(token) = bearer_token {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err("REST probe stdin unavailable".into());
        };
        stdin
            .write_all(format!("Authorization: Bearer {token}\n").as_bytes())
            .map_err(|error| format!("REST probe auth write failed: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("REST health probe failed: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(sanitize_error_summary(&format!(
            "REST health probe transport failed: {detail}"
        )));
    }
    let status = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u16>()
        .map_err(|_| "REST health probe returned invalid status".to_string())?;
    if !(200..300).contains(&status) {
        return Err(format!("REST health probe HTTP {status}"));
    }
    Ok(())
}

pub(super) fn rest_curl_command(url: &str, bearer_token_present: bool) -> Command {
    let mut command = Command::new("curl");
    command.args([
        "--disable",
        "--silent",
        "--show-error",
        "--output",
        "/dev/null",
        "--write-out",
        "%{http_code}",
        "--connect-timeout",
        "2",
        "--max-time",
        "5",
        "--max-redirs",
        "0",
        "--request",
        "GET",
        "--proto",
        "=http,https",
        url,
    ]);
    if bearer_token_present {
        command.args(["--header", "@-"]).stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command
}
