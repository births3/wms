use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::DeliveryNoteGroup;

use crate::{
    auth::AuthContext,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
};

const ACTOR: &str = "system-scheduler";

#[derive(Debug)]
pub enum PrintOrchestrationJobError {
    Database(sqlx::Error),
    Orchestration(PrintOrchestrationError),
}

pub async fn run_once(
    pool: &PgPool,
    now: DateTime<Utc>,
) -> Result<Vec<DeliveryNoteGroup>, PrintOrchestrationJobError> {
    let owner_ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT DISTINCT order_row.owner_id
          FROM outbound_orders order_row
          JOIN h9_outbound_route_snapshots snapshot
            ON snapshot.owner_id = order_row.owner_id
           AND snapshot.outbound_order_id = order_row.id
         WHERE order_row.status = 'confirmed'
           AND NOT EXISTS (
                SELECT 1
                  FROM h9_delivery_note_group_orders grouped
                 WHERE grouped.owner_id = order_row.owner_id
                   AND grouped.outbound_order_id = order_row.id
           )
         ORDER BY order_row.owner_id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(PrintOrchestrationJobError::Database)?;

    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let mut groups = Vec::new();
    for owner_id in owner_ids {
        let context = AuthContext {
            user_id: Uuid::nil(),
            owner_id,
            actor_name: ACTOR.to_string(),
            permissions: vec!["h9.print_orchestration.write".to_string()],
            jti: format!("{ACTOR}:h9-cutoff:{owner_id}:{}", now.timestamp()),
            warehouse_scope: None,
        };
        groups.extend(
            service
                .run_scheduled_cutoffs(&context, now)
                .await
                .map_err(PrintOrchestrationJobError::Orchestration)?,
        );
    }
    Ok(groups)
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(error) = run_once(&pool, Utc::now()).await {
                tracing::error!(?error, "H9 随货同行单自动截单调度失败");
            }
        }
    });
}
