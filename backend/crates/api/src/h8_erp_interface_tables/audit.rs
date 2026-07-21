use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::state::H8ErpInterfaceTableAppState;

pub(crate) async fn write_query_audit(
    state: &H8ErpInterfaceTableAppState,
    ctx: &AuthContext,
    action: &str,
    connector_id: Uuid,
    table_key: &str,
    filter_summary: serde_json::Value,
    result_count: u64,
) {
    let after = json!({
        "connector_id": connector_id,
        "table_key": table_key,
        "filters": filter_summary,
        "result_count": result_count,
    });
    let mut req = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H8",
        "h8_erp_interface_table",
        format!("{connector_id}:{table_key}"),
        Some(AuditDiff::compute(serde_json::Value::Null, after)),
    );
    req.occurred_at = Utc::now();
    {
        let mut log = state.audit_log.lock().expect("audit log");
        log.append_event(req.clone());
    }
    if let Some(pool) = &state.audit_pool {
        let _ = append_event(pool, &req).await;
    }
}
