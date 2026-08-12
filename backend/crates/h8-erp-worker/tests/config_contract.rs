use std::collections::HashMap;

use h8_erp_worker::config::{BootstrapSettings, RuntimeSettings};
use serde_json::json;

fn base_env() -> HashMap<String, String> {
    HashMap::from([
        (
            "H8_CONNECTOR_ID".to_owned(),
            "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5".to_owned(),
        ),
        (
            "WMS_H8_WORKER_API_KEY".to_owned(),
            "worker-api-key".to_owned(),
        ),
    ])
}

#[test]
fn bootstrap_uses_frozen_worker_defaults() {
    let settings = BootstrapSettings::from_map(&base_env()).expect("合法的最小启动配置应被接受");

    assert_eq!(settings.poll_interval_seconds, 5);
    assert_eq!(settings.max_retry, 5);
    assert_eq!(settings.batch_size, 10);
    assert_eq!(settings.lease_minutes, 5);
    assert_eq!(settings.heartbeat_ttl_seconds, 15);
    assert_eq!(settings.owner_code, "ZBPF7");
    assert_eq!(settings.api_key.as_deref(), Some("worker-api-key"));
}

#[test]
fn runtime_transport_only_comes_from_connector_snapshot() {
    let bootstrap = BootstrapSettings::from_map(&base_env()).expect("合法的最小启动配置应被接受");
    let snapshot = json!({
        "id": "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5",
        "owner_id": "00000000-0000-0000-0000-000000000001",
        "config_version": 7,
        "channel_mode": "interface_table",
        "interface_db_host": "10.12.98.254",
        "interface_db_port": 9631,
        "interface_db_name": "zbpf7_test",
        "interface_db_username": "wms_worker_test",
        "interface_db_password_alias": "vault://h8/zbpf7-test"
    });
    let secrets = HashMap::from([(
        "vault://h8/zbpf7-test".to_owned(),
        "resolved-password".to_owned(),
    )]);

    let runtime = RuntimeSettings::from_snapshot(bootstrap, 7, &snapshot, &secrets)
        .expect("匹配的不可变快照应被接受");

    assert_eq!(runtime.mssql.host, "10.12.98.254");
    assert_eq!(runtime.mssql.port, 9631);
    assert_eq!(runtime.mssql.database, "zbpf7_test");
    assert_eq!(runtime.mssql.username, "wms_worker_test");
    assert_eq!(runtime.mssql.password, "resolved-password");
    assert_eq!(runtime.connector_config_version, 7);
}

#[test]
fn runtime_rejects_snapshot_identity_change() {
    let bootstrap = BootstrapSettings::from_map(&base_env()).expect("合法的最小启动配置应被接受");
    let snapshot = json!({
        "id": "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5",
        "config_version": 8,
        "channel_mode": "interface_table"
    });

    let error = RuntimeSettings::from_snapshot(bootstrap, 7, &snapshot, &HashMap::new())
        .expect_err("快照版本变化必须被拒绝");

    assert_eq!(error.code(), "H8_WORKER_SNAPSHOT_IDENTITY_CHANGED");
}
