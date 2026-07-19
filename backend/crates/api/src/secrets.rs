//! ADR-0013 分层 secrets 解析（连接测试用）。
//!
//! 解析顺序：
//! 1. `WMS_H8_SECRET_ALIASES` / `WMS_SECRETS_MAP` JSON 对象（local / e2e 文件式 vault）
//! 2. `WMS_VAULT_ADDR` + `WMS_VAULT_TOKEN` 的 Vault KV v2 HTTP（`vault://path` → `/v1/secret/data/{path}`）
//! 3. 若配置了 `WMS_SECRETS_REQUIRE_RESOLVE=1`（或任一本地方案已启用）则失败；否则仅校验 alias 形态

use std::time::Duration;

/// 解析 secret alias，成功返回明文值（调用方不得记录/回显）。
pub fn resolve_secret_alias(alias: &str) -> Result<String, String> {
    let alias = alias.trim();
    validate_alias_shape(alias)?;

    if let Some(value) = resolve_from_env_map(alias)? {
        return Ok(value);
    }

    if alias.starts_with("vault://") {
        if let Some(value) = resolve_from_vault_http(alias)? {
            return Ok(value);
        }
    }

    if secrets_backend_configured() || secrets_require_resolve() {
        return Err(format!("secret alias not resolvable: {alias}"));
    }

    // 未配置任何 secrets 后端时：形态校验通过即视为可进入后续字段探测（开发默认）。
    Ok(String::new())
}

/// 连接测试专用：要求 secret 可解析；配置了后端时值必须非空。
pub fn resolve_secret_alias_for_probe(alias: Option<&str>) -> Result<(), String> {
    let alias = alias
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "secret alias missing".to_string())?;
    let value = resolve_secret_alias(alias)?;
    if secrets_backend_configured() && value.is_empty() {
        return Err(format!("secret alias resolved empty: {alias}"));
    }
    if !secrets_backend_configured() && secrets_require_resolve() {
        return Err(format!("secret alias not resolvable: {alias}"));
    }
    // 未配置后端时 shape 通过即可
    let _ = value;
    Ok(())
}

fn validate_alias_shape(alias: &str) -> Result<(), String> {
    if alias.len() < 3 || alias.contains(' ') || alias.contains('\n') {
        return Err("invalid secret alias shape".into());
    }
    if alias.starts_with("sk-") || alias.len() > 256 {
        return Err("invalid secret alias shape".into());
    }
    Ok(())
}

fn secrets_require_resolve() -> bool {
    matches!(
        std::env::var("WMS_SECRETS_REQUIRE_RESOLVE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

fn secrets_backend_configured() -> bool {
    env_map_raw().is_some() || vault_http_configured()
}

fn env_map_raw() -> Option<String> {
    std::env::var("WMS_H8_SECRET_ALIASES")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("WMS_SECRETS_MAP")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
}

fn resolve_from_env_map(alias: &str) -> Result<Option<String>, String> {
    let Some(raw) = env_map_raw() else {
        return Ok(None);
    };
    let map: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| "WMS secrets map invalid JSON".to_string())?;
    let value = map
        .as_object()
        .and_then(|o| o.get(alias))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(value)
}

fn vault_http_configured() -> bool {
    std::env::var("WMS_VAULT_ADDR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
        && std::env::var("WMS_VAULT_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .is_some()
}

fn resolve_from_vault_http(alias: &str) -> Result<Option<String>, String> {
    if !vault_http_configured() {
        return Ok(None);
    }
    let addr = std::env::var("WMS_VAULT_ADDR").unwrap_or_default();
    let token = std::env::var("WMS_VAULT_TOKEN").unwrap_or_default();
    let path = alias
        .strip_prefix("vault://")
        .ok_or_else(|| "invalid vault alias".to_string())?
        .trim_matches('/');
    if path.is_empty() {
        return Err("invalid vault alias path".into());
    }
    // KV v2: GET {addr}/v1/secret/data/{path}
    let mount = std::env::var("WMS_VAULT_KV_MOUNT").unwrap_or_else(|_| "secret".into());
    let url = format!(
        "{}/v1/{}/data/{}",
        addr.trim_end_matches('/'),
        mount.trim_matches('/'),
        path
    );
    let body = vault_http_get(&url, &token)?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| "vault response invalid JSON".to_string())?;
    // data.data.value 或 data.data 中第一个字符串字段
    if let Some(data) = json.pointer("/data/data") {
        if let Some(v) = data.get("value").and_then(|x| x.as_str()) {
            let t = v.trim();
            if !t.is_empty() {
                return Ok(Some(t.to_string()));
            }
        }
        if let Some(obj) = data.as_object() {
            for (_k, v) in obj {
                if let Some(s) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    return Ok(Some(s.to_string()));
                }
            }
        }
    }
    Err(format!("vault secret empty for {alias}"))
}

fn vault_http_get(url: &str, token: &str) -> Result<String, String> {
    vault_http_get_impl(url, token)
}

fn vault_http_get_impl(url: &str, token: &str) -> Result<String, String> {
    // 轻量实现：blocking via std Tcp is painful for HTTPS.
    // Prefer optional reqwest if available through workspace.
    use std::io::{Read, Write};
    use std::net::TcpStream;

    if url.starts_with("https://") {
        // 生产应使用正式 Vault client；此处 HTTPS 要求环境有 curl
        return vault_http_get_via_curl(url, token);
    }
    if !url.starts_with("http://") {
        return Err("unsupported vault URL scheme".into());
    }
    let without = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = without
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap_or((without, "/".into()));
    let mut stream = TcpStream::connect(host_port).map_err(|e| format!("vault connect: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nX-Vault-Token: {token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("vault write: {e}"))?;
    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .map_err(|e| format!("vault read: {e}"))?;
    let body = buf
        .split("\r\n\r\n")
        .nth(1)
        .ok_or_else(|| "vault empty body".to_string())?;
    if !buf.contains("200") {
        return Err(format!(
            "vault HTTP error: {}",
            buf.lines().next().unwrap_or("")
        ));
    }
    Ok(body.to_string())
}

fn vault_http_get_via_curl(url: &str, token: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-f",
            "-H",
            &format!("X-Vault-Token: {token}"),
            "--max-time",
            "5",
            url,
        ])
        .output()
        .map_err(|e| format!("vault curl: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "vault curl failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| "vault body not utf8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn rejects_bad_shape() {
        let _g = ENV_LOCK.lock().expect("env lock");
        assert!(validate_alias_shape("ab").is_err());
        assert!(validate_alias_shape("sk-plaintext").is_err());
    }

    #[test]
    fn env_map_resolves_vault_alias() {
        let _g = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("WMS_SECRETS_MAP");
        std::env::remove_var("WMS_VAULT_ADDR");
        std::env::remove_var("WMS_VAULT_TOKEN");
        std::env::set_var(
            "WMS_H8_SECRET_ALIASES",
            r#"{"vault://wms/e2e/h8/bearer":"token-value"}"#,
        );
        let v = resolve_secret_alias("vault://wms/e2e/h8/bearer").expect("resolve");
        assert_eq!(v, "token-value");
        assert!(resolve_secret_alias("vault://missing").is_err());
        std::env::remove_var("WMS_H8_SECRET_ALIASES");
    }

    #[test]
    fn probe_requires_alias() {
        let _g = ENV_LOCK.lock().expect("env lock");
        assert!(resolve_secret_alias_for_probe(None).is_err());
        std::env::remove_var("WMS_H8_SECRET_ALIASES");
        std::env::remove_var("WMS_SECRETS_MAP");
        std::env::remove_var("WMS_VAULT_ADDR");
        std::env::remove_var("WMS_VAULT_TOKEN");
        std::env::remove_var("WMS_SECRETS_REQUIRE_RESOLVE");
        assert!(resolve_secret_alias_for_probe(Some("vault://wms/e2e/x")).is_ok());
    }
}
