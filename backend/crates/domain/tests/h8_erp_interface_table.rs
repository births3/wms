use chrono::{Duration, Utc};
use uuid::Uuid;
use wms_domain::{
    enforce_interface_table_scope, interface_table_spec, redacted_payload_summary,
    H8ErpInterfaceTableQuery, H8InterfaceTableScopeError,
};

#[test]
fn allowlist_and_table_specific_status_filters_are_enforced() {
    assert!(interface_table_spec("if_in_asn").is_some());
    assert!(interface_table_spec("h8_erp_connectors").is_none());

    let now = Utc::now();
    let valid = H8ErpInterfaceTableQuery {
        connector_id: Uuid::new_v4(),
        table_key: "if_in_asn".into(),
        updated_from: now - Duration::days(7),
        updated_to: now,
        sync_status: Some("failed".into()),
        warehouse_id: None,
        external_doc_no: None,
        source_outbox_id: None,
        event_type: None,
        external_ref: None,
        wms_resource_id: None,
        idempotency_key: None,
        page: 1,
        page_size: 50,
    };
    assert!(valid.validate().is_ok());

    let valid_multi_status = H8ErpInterfaceTableQuery {
        sync_status: Some("pending,failed".into()),
        ..valid.clone()
    };
    assert_eq!(
        valid_multi_status.sync_statuses(),
        vec!["pending", "failed"]
    );
    assert!(valid_multi_status.validate().is_ok());

    let invalid_status = H8ErpInterfaceTableQuery {
        sync_status: Some("pending,acked".into()),
        ..valid.clone()
    };
    assert!(invalid_status.validate().is_err());

    let invalid_external_ref = H8ErpInterfaceTableQuery {
        table_key: "if_in_outbound_order".into(),
        external_ref: Some("ERP-REF".into()),
        ..valid.clone()
    };
    assert!(invalid_external_ref.validate().is_err());

    let invalid_range = H8ErpInterfaceTableQuery {
        updated_from: now - Duration::days(32),
        ..valid
    };
    assert!(invalid_range.validate().is_err());
}

#[test]
fn scope_is_intersection_and_non_warehouse_tables_require_owner_wide_actor() {
    let owner = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let connector_whitelist = vec![warehouse];

    assert!(enforce_interface_table_scope(
        owner,
        Some(warehouse),
        owner,
        Some(warehouse),
        &connector_whitelist,
    )
    .is_ok());
    assert_eq!(
        enforce_interface_table_scope(
            owner,
            Some(Uuid::new_v4()),
            owner,
            Some(warehouse),
            &connector_whitelist,
        ),
        Err(H8InterfaceTableScopeError::WarehouseOutOfScope)
    );
    assert_eq!(
        enforce_interface_table_scope(owner, None, owner, Some(warehouse), &[]),
        Err(H8InterfaceTableScopeError::OwnerWideRequired)
    );
}

#[test]
fn payload_summary_redacts_secrets_and_is_bounded() {
    let summary = redacted_payload_summary(
        r#"{"ok":"yes","password":"do-not-show","nested":{"token":"hidden"}}"#,
    );
    assert!(summary.len() <= 4096);
    assert!(!summary.contains("do-not-show"));
    assert!(!summary.contains("hidden"));

    let huge = redacted_payload_summary(&format!(r#"{{"data":"{}"}}"#, "中".repeat(5000)));
    assert!(huge.len() <= 4096);

    let oversized = redacted_payload_summary(&"x".repeat(1024 * 1024 + 1));
    assert_eq!(oversized, "[payload omitted: too large]");
}
