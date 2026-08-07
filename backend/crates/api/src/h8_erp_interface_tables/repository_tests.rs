use super::{
    handelflag_status, is_loopback_probe_host, projection_for, safe_packaging_levels,
    v19_table_contract,
};

#[test]
fn loopback_probe_uses_legacy_sql_server_transport() {
    for host in ["localhost", "127.0.0.1", "::1"] {
        assert!(is_loopback_probe_host(host));
    }
    assert!(!is_loopback_probe_host("10.12.98.254"));
}

#[test]
fn projections_only_reference_columns_present_in_each_interface_table() {
    let outbound =
        projection_for("x_wmsinter_OutboundOrder", false).expect("v1.9 outbound order projection");
    assert!(outbound.contains("CONVERT(nvarchar(128), ERPBillCode) AS business_key"));
    assert!(outbound.contains("CONVERT(nvarchar(64), OrderID) AS row_id"));
    assert!(!outbound.contains("external_ref"));

    let command =
        v19_table_contract("x_wmsinter_OrderCommand").expect("v1.9 order command contract");
    assert_eq!(command.primary_key, "CommandID");
    assert!(v19_table_contract("if_in_asn").is_none());
}

#[test]
fn product_master_projection_separates_list_summary_from_detail_fields() {
    let list = projection_for("x_wmsinter_GoodsInfo", false).expect("v1.9 product list projection");
    assert!(list.contains("product_code"));
    assert!(list.contains("product_name"));
    assert!(list.contains("spec"));
    assert!(!list.contains("packaging_json"));

    let detail =
        projection_for("x_wmsinter_GoodsInfo", true).expect("v1.9 product detail projection");
    assert!(detail.contains("approval_no"));
    assert!(detail.contains("storage_condition"));
    assert!(detail.contains("packaging_json"));
    assert!(detail.contains("SchemaVersion AS schema_version"));
    assert!(!detail.contains("dosage_form"));
    assert!(!detail.contains("udi_code"));
}

#[test]
fn v19_handelflag_maps_without_collapsing_technical_and_business_completion() {
    assert_eq!(handelflag_status(Some(0)), "pending");
    assert_eq!(handelflag_status(Some(1)), "awaiting_receipt");
    assert_eq!(handelflag_status(Some(2)), "processing");
    assert_eq!(handelflag_status(Some(3)), "failed");
    assert_eq!(handelflag_status(Some(4)), "dead");
    assert_eq!(handelflag_status(Some(5)), "acked");
    assert_eq!(handelflag_status(None), "readonly");
}

#[test]
fn packaging_summary_keeps_only_whitelisted_fields() {
    let summary = safe_packaging_levels(Some(
        r#"[{"unit":"片","ratio_to_base":1,"is_base":true,"is_default":false,"sort_order":1,"password":"do-not-show"}]"#,
    ))
    .expect("valid packaging summary");
    assert!(summary.contains(r#""unit":"片""#));
    assert!(!summary.contains("password"));
    assert!(!summary.contains("do-not-show"));

    assert_eq!(
        safe_packaging_levels(Some(r#"{"unit":"片"}"#)).as_deref(),
        Some("[包装数据格式无效]")
    );
}

#[test]
fn packaging_summary_rejects_oversized_safe_output() {
    let levels = (0..100)
        .map(|sort_order| {
            serde_json::json!({
                "unit": "超长包装单位名称".repeat(8),
                "ratio_to_base": sort_order + 1,
                "is_base": sort_order == 0,
                "is_default": sort_order == 1,
                "sort_order": sort_order,
            })
        })
        .collect::<Vec<_>>();
    let raw = serde_json::to_string(&levels).expect("serialize oversized packaging levels");

    assert_eq!(
        safe_packaging_levels(Some(&raw)).as_deref(),
        Some("[包装数据过大，已省略]")
    );
}
