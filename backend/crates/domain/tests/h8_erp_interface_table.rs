use chrono::{Duration, Utc};
use uuid::Uuid;
use wms_domain::{
    enforce_interface_table_scope, interface_table_spec, redacted_payload_summary,
    H8ErpInterfaceTableField, H8ErpInterfaceTableQuery, H8ErpInterfaceTableRow,
    H8InterfaceTableScopeError,
};

#[test]
fn allowlist_and_table_specific_status_filters_are_enforced() {
    assert!(interface_table_spec("x_wmsinter_GoodsInfo").is_some());
    assert!(interface_table_spec("x_wmsinter_InboundOrderItems").is_some());
    assert!(interface_table_spec("if_in_asn").is_none());
    assert!(interface_table_spec("h8_erp_connectors").is_none());

    let now = Utc::now();
    let valid = H8ErpInterfaceTableQuery {
        connector_id: Uuid::new_v4(),
        table_key: "x_wmsinter_InboundOrder".into(),
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
        sync_status: Some("pending,awaiting_receipt,failed,acked".into()),
        ..valid.clone()
    };
    assert_eq!(
        valid_multi_status.sync_statuses(),
        vec!["pending", "awaiting_receipt", "failed", "acked"]
    );
    assert!(valid_multi_status.validate().is_ok());

    let invalid_status = H8ErpInterfaceTableQuery {
        sync_status: Some("success".into()),
        ..valid.clone()
    };
    assert!(invalid_status.validate().is_err());

    let invalid_external_ref = H8ErpInterfaceTableQuery {
        table_key: "x_wmsinter_OutboundOrder".into(),
        external_ref: Some("ERP-REF".into()),
        ..valid.clone()
    };
    assert!(invalid_external_ref.validate().is_err());

    let child_with_status = H8ErpInterfaceTableQuery {
        table_key: "x_wmsinter_InboundOrderItems".into(),
        sync_status: Some("pending".into()),
        ..valid.clone()
    };
    assert!(child_with_status.validate().is_err());

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
    assert_eq!(oversized, "[报文已省略：内容过大]");
    assert_eq!(redacted_payload_summary(""), "[报文已省略：格式无效]");
}

#[test]
fn product_master_row_serializes_safe_business_fields() {
    let now = Utc::now();
    let row = H8ErpInterfaceTableRow {
        row_id: Uuid::new_v4().to_string(),
        connector_id: Uuid::new_v4(),
        table_key: "x_wmsinter_GoodsInfo".into(),
        owner_id: Uuid::new_v4(),
        warehouse_id: None,
        business_key: Some("DEMO-PM-001".into()),
        business_fields: vec![
            H8ErpInterfaceTableField {
                key: "product_code".into(),
                value: Some("DEMO-P-001".into()),
            },
            H8ErpInterfaceTableField {
                key: "product_name".into(),
                value: Some("演示商品-对乙酰氨基酚片".into()),
            },
            H8ErpInterfaceTableField {
                key: "spec".into(),
                value: Some("0.5g*24片".into()),
            },
        ],
        event_type: None,
        external_ref: None,
        wms_resource_id: None,
        sync_status: "pending".into(),
        retry_count: 0,
        last_error: None,
        idempotency_key: Some("h8-demo-pm-001".into()),
        created_at: now,
        updated_at: now,
        payload_summary: "{}".into(),
    };

    let value = serde_json::to_value(row).expect("serialize product master interface row");
    assert_eq!(
        value["business_fields"][1]["value"],
        "演示商品-对乙酰氨基酚片"
    );
    assert!(value.get("payload_json").is_none());
}
