use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::{error::WorkerError, outbound::OutboxRow};

#[derive(Clone, Copy)]
pub struct OutboxSource {
    pub table: &'static str,
    pub reference_column: &'static str,
    pub message_type: &'static str,
    pub bounded_retry: bool,
}

const SOURCES: [OutboxSource; 7] = [
    OutboxSource {
        table: "receiving_putaway_erp_feedback_outbox",
        reference_column: "receiving_order_id",
        message_type: "putaway_complete",
        bounded_retry: false,
    },
    OutboxSource {
        table: "inventory_status_erp_feedback_outbox",
        reference_column: "batch_id",
        message_type: "inventory_status",
        bounded_retry: false,
    },
    OutboxSource {
        table: "stock_adjustment_erp_feedback_outbox",
        reference_column: "order_id",
        message_type: "stock_adjustment",
        bounded_retry: false,
    },
    OutboxSource {
        table: "archive_revision_erp_feedback_outbox",
        reference_column: "liaison_id",
        message_type: "archive_revision",
        bounded_retry: true,
    },
    OutboxSource {
        table: "reconciliation_erp_feedback_outbox",
        reference_column: "recon_doc_no",
        message_type: "reconciliation_diff",
        bounded_retry: true,
    },
    OutboxSource {
        table: "shipment_confirm_erp_feedback_outbox",
        reference_column: "shipment_id",
        message_type: "shipment_confirm",
        bounded_retry: false,
    },
    OutboxSource {
        table: "inventory_snapshot_erp_feedback_outbox",
        reference_column: "snapshot_no",
        message_type: "inventory_snapshot",
        bounded_retry: false,
    },
];

pub fn outbox_sources() -> &'static [OutboxSource] {
    &SOURCES
}

pub fn claim_sql(source: &OutboxSource) -> String {
    let barrier_guard = if source.table == "receiving_putaway_erp_feedback_outbox" {
        "AND (source.event_type <> 'order_status' OR source.payload ->> 'feedback_type' <> '2' OR NOT EXISTS (SELECT 1 FROM receiving_putaway_erp_feedback_outbox detail WHERE detail.owner_id = source.owner_id AND detail.event_type = 'inbound_putaway_completed' AND detail.payload ->> 'erp_bill_code' = source.payload ->> 'erp_bill_code' AND detail.payload ->> 'revision' = source.payload ->> 'revision' AND detail.status <> 'succeeded'))"
    } else {
        ""
    };
    let bounded_guard = if source.bounded_retry {
        "AND source.attempt_count < COALESCE((to_jsonb(source) ->> 'max_attempts')::int, 5) AND COALESCE((to_jsonb(source) ->> 'deadline_at')::timestamptz, 'infinity'::timestamptz) > now()"
    } else {
        ""
    };
    format!(
        r#"WITH picked AS (
  SELECT source.id
    FROM {table} source
   WHERE source.owner_id = $2
     AND source.status IN ('pending', 'failed')
     AND source.next_attempt_at <= now()
     {bounded_guard}
     {barrier_guard}
   ORDER BY source.next_attempt_at, source.created_at, source.id
   LIMIT $1
   FOR UPDATE SKIP LOCKED
)
UPDATE {table} source
   SET attempt_count = source.attempt_count + 1,
       next_attempt_at = now() + interval '5 minutes',
       updated_at = now()
  FROM picked
 WHERE source.id = picked.id
RETURNING source.id, source.owner_id, source.event_type, source.payload,
          COALESCE(to_jsonb(source) ->> '{reference}', '') AS external_ref,
          source.attempt_count,
          COALESCE((to_jsonb(source) ->> 'max_attempts')::int, 5) AS max_attempts,
          source.created_at"#,
        table = source.table,
        reference = source.reference_column,
    )
}

#[derive(Clone)]
pub struct PgOutboxRepository {
    pool: PgPool,
}

impl PgOutboxRepository {
    pub async fn connect(database_url: &str) -> Result<Self, WorkerError> {
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(database_url)
            .await
            .map_err(pg_error)?;
        Ok(Self { pool })
    }

    pub async fn healthcheck(&self) -> Result<(), WorkerError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(pg_error)?;
        Ok(())
    }

    pub async fn claim(
        &self,
        source: &'static OutboxSource,
        batch_size: u32,
        owner_id: Uuid,
    ) -> Result<Vec<OutboxRow>, WorkerError> {
        let rows = sqlx::query(&claim_sql(source))
            .bind(i64::from(batch_size))
            .bind(owner_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_error)?;
        rows.into_iter()
            .map(|row| {
                let created_at: DateTime<Utc> = row.try_get("created_at").map_err(pg_error)?;
                Ok(OutboxRow {
                    table: source.table,
                    id: row.try_get("id").map_err(pg_error)?,
                    owner_id: row.try_get("owner_id").map_err(pg_error)?,
                    event_type: row.try_get("event_type").map_err(pg_error)?,
                    payload: row.try_get::<Value, _>("payload").map_err(pg_error)?,
                    external_ref: row.try_get("external_ref").map_err(pg_error)?,
                    attempt_count: row.try_get("attempt_count").map_err(pg_error)?,
                    max_attempts: row.try_get("max_attempts").map_err(pg_error)?,
                    created_at: created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
                })
            })
            .collect()
    }

    pub async fn mark(
        &self,
        source: &OutboxSource,
        row: &OutboxRow,
        error: Option<&WorkerError>,
    ) -> Result<(), WorkerError> {
        let exhausted = source.bounded_retry && row.attempt_count >= row.max_attempts;
        let status = if error.is_none() {
            "succeeded"
        } else if exhausted {
            "dead"
        } else {
            "failed"
        };
        let summary = error.map(|value| {
            format!("{}: {}", value.code(), value.message())
                .chars()
                .take(900)
                .collect::<String>()
        });
        let sql = format!(
            "UPDATE {} SET status=$1, last_error=$2, next_attempt_at=CASE WHEN $1='failed' THEN now()+interval '5 minutes' ELSE next_attempt_at END, updated_at=now() WHERE id=$3 AND owner_id=$4 AND attempt_count=$5",
            source.table
        );
        sqlx::query(&sql)
            .bind(status)
            .bind(summary)
            .bind(row.id)
            .bind(row.owner_id)
            .bind(row.attempt_count)
            .execute(&self.pool)
            .await
            .map_err(pg_error)?;
        Ok(())
    }

    pub async fn requeue(&self, table: &str, id: Uuid, owner_id: Uuid) -> Result<(), WorkerError> {
        let source = SOURCES
            .iter()
            .find(|source| source.table == table)
            .ok_or_else(|| WorkerError::new("INVALID_DATA", "unregistered outbox table"))?;
        let sql = format!(
            "UPDATE {} SET status='failed', last_error='business receipt timeout', next_attempt_at=now(), updated_at=now() WHERE id=$1 AND owner_id=$2 AND status='succeeded'",
            source.table
        );
        sqlx::query(&sql)
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(pg_error)?;
        Ok(())
    }
}

fn pg_error(error: impl std::fmt::Display) -> WorkerError {
    WorkerError::new("H8_WORKER_POSTGRES_FAILED", error.to_string())
}
