use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    audit::{append_event, AuditDiff, AuditError, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
    sync::lock_recover,
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
) -> Result<(), AuditError> {
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
    persist_query_audit(state.audit_pool.as_ref(), &req).await?;
    lock_recover(&state.audit_log).append_event(req);
    Ok(())
}

async fn persist_query_audit(
    pool: Option<&sqlx::PgPool>,
    req: &AuditWriteRequest,
) -> Result<(), AuditError> {
    if let Some(pool) = pool {
        append_event(pool, req).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::persist_query_audit;
    use crate::{audit::AuditWriteRequest, operation_context::OperationContext as AuthContext};
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    async fn persistent_audit_failure_is_not_swallowed() {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://wms:wms@127.0.0.1:1/wms")
            .expect("lazy pool");
        let ctx = AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "audit-test".into(),
            permissions: vec![],
            jti: "audit-test-jti".into(),
            warehouse_scope: None,
        };
        let req = AuditWriteRequest::from_auth_context(
            &ctx,
            "h8_interface_table_list_query",
            "H8",
            "h8_erp_interface_table",
            "connector:if_in_asn",
            None,
        );

        assert!(persist_query_audit(Some(&pool), &req).await.is_err());
    }
}
