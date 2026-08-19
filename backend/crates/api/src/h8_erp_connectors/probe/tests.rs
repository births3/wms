use super::{
    interface::{
        is_loopback_interface_host, required_interface_contracts, requires_control_column_updates,
    },
    rest::{rest_curl_command, rest_url_probe, validate_rest_probe_endpoint},
    run_connection_probe,
};

#[test]
fn interface_transport_allows_legacy_plaintext_only_over_loopback_tunnel() {
    for host in ["localhost", "127.0.0.1", "::1"] {
        assert!(is_loopback_interface_host(host));
    }
    for host in ["10.12.98.254", "sql.example.test"] {
        assert!(!is_loopback_interface_host(host));
    }
}

#[test]
fn interface_update_scope_is_limited_to_inbound_main_records() {
    for table in [
        "x_wmsinter_GoodsInfo",
        "x_wmsinter_InboundOrder",
        "x_wmsinter_OrderCommand",
        "x_wmsinter_InventoryPushHeader",
    ] {
        assert!(requires_control_column_updates(table));
    }
    for table in [
        "x_wmsinter_InboundOrderItems",
        "x_wmsinter_OrderFeedback",
        "x_wmsinter_InventoryReceiveHeader",
    ] {
        assert!(!requires_control_column_updates(table));
    }
}
use chrono::Utc;
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    thread,
    time::Duration,
};
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_domain::{H8ErpConnector, H8_INTERFACE_TABLE_REQUIRED_OBJECTS};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

struct EnvGuard {
    name: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var(name).ok();
        std::env::set_var(name, value);
        Self { name, previous }
    }

    fn remove(name: &'static str) -> Self {
        let previous = std::env::var(name).ok();
        std::env::remove_var(name);
        Self { name, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var(self.name, previous);
        } else {
            std::env::remove_var(self.name);
        }
    }
}

fn connector(channel_mode: &str) -> H8ErpConnector {
    let now = Utc::now();
    H8ErpConnector {
        id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        connector_code: "probe-test".into(),
        connector_name: "Probe Test".into(),
        warehouse_ids: vec![],
        directions: vec!["inbound".into()],
        message_types: vec!["asn".into()],
        channel_mode: channel_mode.into(),
        api_base_url: None,
        interface_db_host: None,
        interface_db_port: None,
        interface_db_name: None,
        interface_db_username: None,
        api_key_id: Some(Uuid::new_v4()),
        bearer_secret_alias: None,
        interface_db_password_alias: None,
        interface_probe_db_username: None,
        interface_probe_db_password_alias: None,
        interface_probe_db_password_alias_set: false,
        interface_probe_config_version: 1,
        status: "testing".into(),
        config_version: 1,
        first_activated_at: None,
        last_tested_version: None,
        last_tested_at: None,
        last_tested_succeeded: None,
        last_tested_error_summary: None,
        created_at: now,
        updated_at: now,
    }
}

fn http_server(status: u16) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("test server address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let mut request = [0_u8; 4096];
        let size = stream.read(&mut request).unwrap_or_default();
        let response =
            format!("HTTP/1.1 {status} Probe\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = stream.write_all(response.as_bytes());
        String::from_utf8_lossy(&request[..size]).to_string()
    });
    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn rest_probe_rejects_non_success_health_response() {
    let _env = ENV_LOCK.lock().await;
    let _local_http = EnvGuard::set("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP", "true");
    let (base_url, server) = http_server(500);
    let _allowlist = EnvGuard::set(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        base_url.trim_start_matches("http://"),
    );
    let mut connector = connector("rest");
    connector.api_base_url = Some(base_url);

    let (succeeded, error) = run_connection_probe(&connector).await;

    let _request = server.join().expect("server join");
    assert!(!succeeded, "500 health response must fail the probe");
    assert!(
        error.as_deref().is_some_and(|value| value.contains("500")),
        "probe should expose the sanitized HTTP status: {error:?}"
    );
}

#[tokio::test]
async fn interface_probe_rejects_unreachable_worker_transport() {
    let _env = ENV_LOCK.lock().await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve interface port");
    let port = listener.local_addr().expect("interface address").port();
    drop(listener);
    std::env::set_var(
        "WMS_H8_SECRET_ALIASES",
        r#"{"vault://h8/worker-db":"worker-password"}"#,
    );
    let mut connector = connector("interface_table");
    connector.interface_db_host = Some("127.0.0.1".into());
    connector.interface_db_port = Some(i32::from(port));
    connector.interface_db_name = Some("wms_erp_if".into());
    connector.interface_db_username = Some("h8_worker".into());
    connector.interface_db_password_alias = Some("vault://h8/worker-db".into());

    let (succeeded, error) = run_connection_probe(&connector).await;

    std::env::remove_var("WMS_H8_SECRET_ALIASES");
    assert!(!succeeded, "closed database port must fail the probe");
    assert_ne!(
        error.as_deref(),
        Some("H8_PROBE_CREDENTIAL_NOT_CONFIGURED"),
        "a resolved Worker transport secret must reach the actual database login path"
    );
}

#[tokio::test]
async fn outbound_rest_probe_sends_resolved_bearer_to_health_endpoint() {
    let _env = ENV_LOCK.lock().await;
    let _local_http = EnvGuard::set("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP", "true");
    std::env::set_var(
        "WMS_H8_SECRET_ALIASES",
        r#"{"vault://h8/erp-bearer":"probe-token"}"#,
    );
    let (base_url, server) = http_server(200);
    let _allowlist = EnvGuard::set(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        base_url.trim_start_matches("http://"),
    );
    let mut connector = connector("rest");
    connector.directions = vec!["outbound".into()];
    connector.api_key_id = None;
    connector.api_base_url = Some(base_url);
    connector.bearer_secret_alias = Some("vault://h8/erp-bearer".into());

    let (succeeded, error) = run_connection_probe(&connector).await;

    let request = server.join().expect("server join");
    std::env::remove_var("WMS_H8_SECRET_ALIASES");
    assert!(succeeded, "Bearer health probe should pass: {error:?}");
    assert!(request.starts_with("GET /healthz HTTP/1.1"));
    assert!(request.contains("Authorization: Bearer probe-token"));
}

#[tokio::test]
async fn fallback_requires_rest_and_interface_probe_to_succeed() {
    let _env = ENV_LOCK.lock().await;
    let _local_http = EnvGuard::set("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP", "true");
    let (base_url, server) = http_server(200);
    let _allowlist = EnvGuard::set(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        base_url.trim_start_matches("http://"),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve interface port");
    let port = listener.local_addr().expect("interface address").port();
    drop(listener);
    std::env::set_var(
        "WMS_H8_SECRET_ALIASES",
        r#"{"vault://h8/worker-db":"worker-password"}"#,
    );
    let mut connector = connector("rest_primary_table_fallback");
    connector.api_base_url = Some(base_url);
    connector.interface_db_host = Some("127.0.0.1".into());
    connector.interface_db_port = Some(i32::from(port));
    connector.interface_db_name = Some("wms_erp_if".into());
    connector.interface_db_username = Some("h8_worker".into());
    connector.interface_db_password_alias = Some("vault://h8/worker-db".into());

    let (succeeded, error) = run_connection_probe(&connector).await;

    let request = server.join().expect("server join");
    std::env::remove_var("WMS_H8_SECRET_ALIASES");
    assert!(request.starts_with("GET /healthz HTTP/1.1"));
    assert!(!succeeded, "unreachable fallback database must fail");
    assert!(
        error
            .as_deref()
            .is_some_and(|value| value.contains("fallback interface probe")),
        "fallback database failure must be identified: {error:?}"
    );
}

#[test]
fn interface_contract_catalog_covers_declared_worker_tables() {
    let mut connector = connector("interface_table");
    connector.directions = vec!["inbound".into(), "outbound".into()];
    connector.message_types = vec![
        "asn".into(),
        "outbound_order".into(),
        "product_master".into(),
        "customer_master".into(),
        "supplier_master".into(),
        "inventory_seed_snapshot".into(),
        "order_cancel".into(),
        "order_status".into(),
        "putaway_complete".into(),
        "inventory_status".into(),
        "stock_adjustment".into(),
        "archive_revision".into(),
        "reconciliation_diff".into(),
        "shipment_confirm".into(),
        "inventory_snapshot".into(),
    ];

    let contracts = required_interface_contracts(&connector).expect("contracts");

    let mut actual = contracts
        .iter()
        .map(|contract| contract.table)
        .collect::<Vec<_>>();
    let mut declared = H8_INTERFACE_TABLE_REQUIRED_OBJECTS.to_vec();
    actual.sort_unstable();
    declared.sort_unstable();

    assert_eq!(actual, declared);
    assert!(contracts.iter().all(|contract| {
        !contract.columns.is_empty()
            && !contract.permissions.is_empty()
            && contract
                .permissions
                .iter()
                .all(|permission| matches!(*permission, "SELECT" | "INSERT" | "UPDATE"))
    }));
}

#[test]
fn interface_contracts_require_v19_frozen_business_columns() {
    let mut connector = connector("interface_table");
    connector.directions = vec!["inbound".into(), "outbound".into()];
    connector.message_types = vec![
        "product_master".into(),
        "outbound_order".into(),
        "inventory_seed_snapshot".into(),
        "inventory_snapshot".into(),
    ];

    let contracts = required_interface_contracts(&connector).expect("contracts");
    let columns = |table| {
        contracts
            .iter()
            .find(|contract| contract.table == table)
            .expect("table contract")
            .columns
    };

    for column in ["IsImport", "IsTCM", "SpecialCategory"] {
        assert!(columns("x_wmsinter_GoodsInfo").contains(&column));
    }
    for column in ["RequiredShipAt", "ERPAddressID", "AddressCode"] {
        assert!(columns("x_wmsinter_OutboundOrder").contains(&column));
    }
    for column in ["StallCode", "GoodsStatus"] {
        assert!(columns("x_wmsinter_InventoryPushItems").contains(&column));
    }
    assert!(columns("x_wmsinter_InventoryReceiveItems").contains(&"GoodsStatus"));
}

#[test]
fn interface_contracts_reject_message_not_matching_any_selected_direction() {
    let mut connector = connector("interface_table");
    connector.directions = vec!["inbound".into()];
    connector.message_types = vec!["asn".into(), "shipment_confirm".into()];

    let result = required_interface_contracts(&connector);

    assert!(
        result.is_err(),
        "outbound message must not be silently ignored on an inbound-only connector"
    );
}

#[test]
fn interface_contracts_require_each_selected_direction_to_have_a_message() {
    let mut connector = connector("interface_table");
    connector.directions = vec!["inbound".into(), "outbound".into()];
    connector.message_types = vec!["asn".into()];

    let result = required_interface_contracts(&connector);

    assert!(
        result.is_err(),
        "outbound direction without an outbound message must be rejected"
    );
}

#[tokio::test]
async fn rest_probe_endpoint_policy_is_fail_closed_and_rejects_ip_bypasses() {
    let _env = ENV_LOCK.lock().await;
    let _allowlist = EnvGuard::remove("WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS");
    let _local_http = EnvGuard::remove("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP");

    assert!(
        validate_rest_probe_endpoint("http://localhost:18091").is_err(),
        "local HTTP must fail closed unless development explicitly enables it"
    );
    assert!(validate_rest_probe_endpoint("http://127.0.0.1:18091").is_err());
    std::env::set_var("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP", "true");
    std::env::set_var(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        "localhost:18091,127.0.0.1:18091",
    );
    assert!(validate_rest_probe_endpoint("http://localhost:18091").is_ok());
    assert!(validate_rest_probe_endpoint("http://127.0.0.1:18091").is_ok());
    assert!(validate_rest_probe_endpoint("http://localhost:18092").is_err());
    std::env::remove_var("WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS");
    assert!(
        validate_rest_probe_endpoint("https://erp.example.test").is_err(),
        "HTTPS must be denied when deployment endpoint allowlist is absent"
    );

    std::env::set_var(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        " ERP.EXAMPLE.TEST.:443 ,8.8.8.8:443,10.0.0.1:8443,[fd00::1]:443",
    );
    assert!(validate_rest_probe_endpoint("https://erp.example.test/base").is_ok());
    assert!(validate_rest_probe_endpoint("https://8.8.8.8").is_ok());
    assert!(
        validate_rest_probe_endpoint("https://10.0.0.1:8443").is_ok(),
        "an explicitly authorized on-prem private address must be allowed"
    );
    assert!(
        validate_rest_probe_endpoint("https://[fd00::1]").is_ok(),
        "an explicitly authorized on-prem unique-local address must be allowed"
    );
    assert!(
        validate_rest_probe_endpoint("https://10.0.0.1").is_err(),
        "same host on an unlisted port must be denied"
    );
    assert!(
        validate_rest_probe_endpoint("https://erp.example.test:8443").is_err(),
        "credentials must not be sent to another port on the same host"
    );

    std::env::set_var("WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS", "erp.example.test");
    assert!(
        validate_rest_probe_endpoint("https://erp.example.test").is_err(),
        "host-only allowlist entries must be rejected"
    );
    std::env::set_var("WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS", "*.example.test:443");
    assert!(
        validate_rest_probe_endpoint("https://erp.example.test").is_err(),
        "wildcard allowlist entries must be rejected"
    );
    std::env::set_var(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        "erp.example.test:443",
    );

    for unsafe_url in [
        "https://localhost",
        "https://127.0.0.1",
        "https://[::1]",
        "https://[::ffff:127.0.0.1]",
        "https://[::ffff:169.254.169.254]",
        "https://0.0.0.0",
        "https://224.0.0.1",
        "https://255.255.255.255",
        "https://169.254.169.254",
        "https://[::]",
        "https://[fe80::1]",
        "https://[ff02::1]",
        "https://2130706433",
        "https://0x7f000001",
        "https://0x7f.0.0.1",
        "https://127.0.0.0x1",
        "https://127.1",
        "https://user@erp.example.test",
    ] {
        let error = validate_rest_probe_endpoint(unsafe_url)
            .expect_err("unsafe endpoint must fail before Bearer is sent");
        assert!(
            !error.contains("erp.example.test") && !error.contains("user"),
            "endpoint error must not echo sensitive URL parts: {error}"
        );
    }
}

#[test]
fn rest_curl_disables_config_before_all_other_options() {
    let command = rest_curl_command("https://erp.example.test/healthz", true);
    let args = command
        .get_args()
        .map(|argument| argument.to_string_lossy())
        .collect::<Vec<_>>();

    assert_eq!(args.first().map(AsRef::as_ref), Some("--disable"));
    assert!(
        args.windows(2).any(|pair| pair == ["--max-redirs", "0"]),
        "redirects must be explicitly disabled"
    );
    assert!(!args.iter().any(|argument| {
        matches!(
            argument.as_ref(),
            "--insecure" | "-k" | "--location" | "-L" | "--location-trusted"
        )
    }));
}

#[tokio::test]
async fn rest_probe_ignores_curlrc_redirect_and_insecure_options() {
    let _env = ENV_LOCK.lock().await;
    let _local_http = EnvGuard::set("WMS_H8_REST_PROBE_ALLOW_LOCAL_HTTP", "true");
    let curl_home = std::env::temp_dir().join(format!("wms-h8-curlrc-{}", Uuid::new_v4()));
    fs::create_dir_all(&curl_home).expect("create curl home");
    fs::write(
        curl_home.join(".curlrc"),
        "location\nlocation-trusted\ninsecure\n",
    )
    .expect("write hostile curl config");
    let curl_home_string = curl_home.to_string_lossy().into_owned();
    let _curl_home = EnvGuard::set("CURL_HOME", &curl_home_string);
    let _home = EnvGuard::set("HOME", &curl_home_string);

    let redirect_target = TcpListener::bind("127.0.0.1:0").expect("bind redirect target");
    let target_port = redirect_target
        .local_addr()
        .expect("redirect target address")
        .port();
    let (base_url, server) = redirect_server(&format!(
        "http://localhost:{target_port}/bearer-must-not-arrive"
    ));
    let _allowlist = EnvGuard::set(
        "WMS_H8_REST_PROBE_ALLOWED_ENDPOINTS",
        base_url.trim_start_matches("http://"),
    );

    let result = rest_url_probe(&base_url, Some("redirect-secret".into())).await;

    let first_request = server.join().expect("redirect server join");
    assert!(
        matches!(result.as_ref(), Err(error) if error.contains("HTTP 302")),
        "probe must stop at the redirect when a hostile curlrc enables location: {result:?}"
    );
    assert!(first_request.contains("Authorization: Bearer redirect-secret"));
    redirect_target
        .set_nonblocking(true)
        .expect("set redirect target nonblocking");
    match redirect_target.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Ok((mut stream, _)) => {
            let request = read_request(&mut stream);
            panic!("curl followed a cross-host redirect from .curlrc: {request}");
        }
        Err(error) => panic!("inspect redirect target: {error}"),
    }
    remove_test_directory(&curl_home);
}

fn redirect_server(location: &str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect server");
    let address = listener.local_addr().expect("redirect server address");
    let location = location.to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept redirect probe");
        let request = read_request(&mut stream);
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .expect("write redirect response");
        request
    });
    (format!("http://{address}"), handle)
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let mut request = [0_u8; 4096];
    let size = stream.read(&mut request).unwrap_or_default();
    String::from_utf8_lossy(&request[..size]).to_string()
}

fn remove_test_directory(path: &Path) {
    fs::remove_dir_all(path).expect("remove curl home");
}
