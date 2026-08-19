use h8_erp_worker::mssql::{
    claim_statement, insert_statement, mark_statement, retry_delay_seconds, table_contract,
    MarkStatus,
};

#[test]
fn claim_statement_uses_atomic_lease_and_stable_order() {
    let contract = table_contract("x_wmsinter_InboundOrder").expect("入库头表必须登记 Worker 契约");
    let sql = claim_statement(contract);

    assert!(sql.contains("FROM dbo.x_wmsinter_InboundOrder WITH (UPDLOCK, READPAST, ROWLOCK)"));
    assert!(sql.contains("OwnerCode = @P4"));
    assert!(sql.contains("handelflag = 0"));
    assert!(sql.contains("handelflag = 3 AND next_retry_at <= SYSUTCDATETIME()"));
    assert!(sql.contains("handelflag = 2 AND lease_until < SYSUTCDATETIME()"));
    assert!(sql.contains("ORDER BY inserttime, OrderID"));
    assert!(sql.contains("SET handelflag = 2"));
    assert!(sql.contains("OUTPUT INSERTED.OrderID INTO @claimed"));
    assert!(sql.contains("ORDER BY source.inserttime, source.OrderID"));
}

#[test]
fn outbound_insert_statements_are_whitelisted_and_parameterized() {
    let sql = insert_statement("x_wmsinter_OrderFeedback").expect("feedback insert SQL");
    assert!(sql.contains("INSERT INTO dbo.x_wmsinter_OrderFeedback"));
    assert!(sql.contains("@P1"));
    assert!(!sql.contains("OUTBOUND_SHORTAGE"));
    assert!(insert_statement("x_wmsinter_unknown").is_none());

    let child =
        insert_statement("x_wmsinter_InventoryReceiveItems").expect("snapshot child insert SQL");
    assert!(!child.contains("PayloadDigest"));
    assert!(!child.contains("handelflag"));
}

#[test]
fn mark_statement_only_updates_control_columns() {
    let contract =
        table_contract("x_wmsinter_OutboundOrder").expect("出库头表必须登记 Worker 契约");
    let sql = mark_statement(contract, MarkStatus::Retry);

    for control in [
        "handelflag",
        "handelmsg",
        "error_code",
        "retry_count",
        "next_retry_at",
        "lease_until",
        "processtime",
    ] {
        assert!(sql.contains(control), "missing control column {control}");
    }
    for payload in ["ERPBillCode", "Revision", "LineCount", "PayloadDigest"] {
        assert!(!sql.contains(&format!("SET {payload}")));
        assert!(!sql.contains(&format!(", {payload}")));
    }
    assert!(sql.contains("WHERE OwnerCode = @P5 AND OrderID = @P6"));
}

#[test]
fn only_claimable_main_tables_are_registered() {
    assert!(table_contract("x_wmsinter_GoodsInfo").is_some());
    assert!(table_contract("x_wmsinter_OrderCommand").is_some());
    assert!(table_contract("x_wmsinter_InventoryPushHeader").is_some());
    assert!(table_contract("x_wmsinter_InboundOrderItems").is_none());
    assert!(table_contract("x_wmsinter_InventoryPushItems").is_none());
}

#[test]
fn retry_backoff_matches_v19_cap() {
    assert_eq!(retry_delay_seconds(1), 1);
    assert_eq!(retry_delay_seconds(5), 16);
    assert_eq!(retry_delay_seconds(30), 60);
}
