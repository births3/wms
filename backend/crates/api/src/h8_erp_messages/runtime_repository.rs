//! US-H8-003 AC9/15：Worker 心跳与按连接、方向暂停认领。

use std::{collections::HashMap, sync::Mutex};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    derive_worker_health, is_worker_claim_paused, validate_direction, H8MessageError,
    H8WorkerClaimControl, H8WorkerClaimDecision, H8WorkerHeartbeatRequest, H8WorkerRuntimeResponse,
    H8WorkerStatus, SetH8WorkerClaimControlRequest,
};

use super::error::H8ErpMessageRepoError;
use crate::sync::lock_recover;

#[axum::async_trait]
pub trait H8WorkerRuntimeRepository: Send + Sync {
    async fn record_heartbeat(
        &self,
        owner_id: Uuid,
        request: &H8WorkerHeartbeatRequest,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerStatus, H8ErpMessageRepoError>;

    async fn list_runtime(
        &self,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerRuntimeResponse, H8ErpMessageRepoError>;

    async fn set_claim_control(
        &self,
        owner_id: Uuid,
        request: &SetH8WorkerClaimControlRequest,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimControl, H8ErpMessageRepoError>;

    async fn claim_decision(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        direction: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimDecision, H8ErpMessageRepoError>;
}

#[derive(Default)]
pub struct MemoryH8WorkerRuntimeRepository {
    heartbeats: Mutex<HashMap<(Uuid, String), H8WorkerStatus>>,
    controls: Mutex<HashMap<(Uuid, Uuid, String), H8WorkerClaimControl>>,
}

fn validate_heartbeat(request: &H8WorkerHeartbeatRequest) -> Result<(), H8ErpMessageRepoError> {
    if request.worker_id.trim().is_empty() || request.worker_id.len() > 128 {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::FieldRequired("worker_id"),
        ));
    }
    if request.worker_version.trim().is_empty() || request.worker_version.len() > 64 {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::FieldRequired("worker_version"),
        ));
    }
    if request.directions.is_empty()
        || request.directions.len() > 2
        || (request.directions.len() == 2 && request.directions[0] == request.directions[1])
        || request.current_claims < 0
    {
        return Err(H8ErpMessageRepoError::Domain(H8MessageError::InvalidStatus));
    }
    for direction in &request.directions {
        validate_direction(direction).map_err(H8ErpMessageRepoError::Domain)?;
    }
    if !(1..=3600).contains(&request.heartbeat_ttl_seconds) {
        return Err(H8ErpMessageRepoError::Domain(H8MessageError::InvalidStatus));
    }
    Ok(())
}

fn validate_control(
    request: &SetH8WorkerClaimControlRequest,
    now: DateTime<Utc>,
) -> Result<(), H8ErpMessageRepoError> {
    validate_direction(request.direction.trim()).map_err(H8ErpMessageRepoError::Domain)?;
    if !request.confirmed {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::FieldRequired("confirmed"),
        ));
    }
    if request.reason.trim().is_empty() || request.reason.len() > 500 {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::FieldRequired("reason"),
        ));
    }
    if request.paused && request.paused_until.is_some_and(|until| until <= now) {
        return Err(H8ErpMessageRepoError::Domain(H8MessageError::InvalidStatus));
    }
    Ok(())
}

fn heartbeat_status(request: &H8WorkerHeartbeatRequest, now: DateTime<Utc>) -> H8WorkerStatus {
    let expires_at = now + chrono::Duration::seconds(request.heartbeat_ttl_seconds);
    H8WorkerStatus {
        worker_id: request.worker_id.trim().to_string(),
        worker_version: request.worker_version.trim().to_string(),
        connector_id: request.connector_id,
        directions: request.directions.clone(),
        current_claims: request.current_claims,
        created_at: now,
        last_heartbeat_at: now,
        heartbeat_expires_at: expires_at,
        health: derive_worker_health(expires_at, now).into(),
    }
}

fn claim_decision(
    control: Option<H8WorkerClaimControl>,
    now: DateTime<Utc>,
) -> H8WorkerClaimDecision {
    match control.filter(|c| is_worker_claim_paused(c.paused, c.paused_until, now)) {
        Some(control) => H8WorkerClaimDecision {
            allowed: false,
            reason: Some(control.reason),
            paused_until: control.paused_until,
        },
        None => H8WorkerClaimDecision {
            allowed: true,
            reason: None,
            paused_until: None,
        },
    }
}

#[axum::async_trait]
impl H8WorkerRuntimeRepository for MemoryH8WorkerRuntimeRepository {
    async fn record_heartbeat(
        &self,
        owner_id: Uuid,
        request: &H8WorkerHeartbeatRequest,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerStatus, H8ErpMessageRepoError> {
        validate_heartbeat(request)?;
        let mut status = heartbeat_status(request, now);
        let mut heartbeats = lock_recover(&self.heartbeats);
        if let Some(existing) = heartbeats.get(&(owner_id, status.worker_id.clone())) {
            status.created_at = existing.created_at;
        }
        heartbeats.insert((owner_id, status.worker_id.clone()), status.clone());
        Ok(status)
    }

    async fn list_runtime(
        &self,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerRuntimeResponse, H8ErpMessageRepoError> {
        let mut workers: Vec<_> = lock_recover(&self.heartbeats)
            .iter()
            .filter(|((owner, _), _)| *owner == owner_id)
            .map(|(_, worker)| {
                let mut worker = worker.clone();
                worker.health = derive_worker_health(worker.heartbeat_expires_at, now).into();
                worker
            })
            .collect();
        workers.sort_by(|a, b| a.worker_id.cmp(&b.worker_id));
        let mut controls: Vec<_> = lock_recover(&self.controls)
            .iter()
            .filter(|((owner, _, _), _)| *owner == owner_id)
            .map(|(_, control)| {
                let mut control = control.clone();
                control.paused = is_worker_claim_paused(control.paused, control.paused_until, now);
                control
            })
            .collect();
        controls
            .sort_by(|a, b| (a.connector_id, &a.direction).cmp(&(b.connector_id, &b.direction)));
        Ok(H8WorkerRuntimeResponse { workers, controls })
    }

    async fn set_claim_control(
        &self,
        owner_id: Uuid,
        request: &SetH8WorkerClaimControlRequest,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimControl, H8ErpMessageRepoError> {
        validate_control(request, now)?;
        let control = H8WorkerClaimControl {
            connector_id: request.connector_id,
            direction: request.direction.trim().to_string(),
            paused: request.paused,
            reason: request.reason.trim().to_string(),
            paused_until: if request.paused {
                request.paused_until
            } else {
                None
            },
            updated_by: actor.to_string(),
            updated_at: now,
        };
        lock_recover(&self.controls).insert(
            (owner_id, request.connector_id, control.direction.clone()),
            control.clone(),
        );
        Ok(control)
    }

    async fn claim_decision(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        direction: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimDecision, H8ErpMessageRepoError> {
        validate_direction(direction).map_err(H8ErpMessageRepoError::Domain)?;
        let control = lock_recover(&self.controls)
            .get(&(owner_id, connector_id, direction.to_string()))
            .cloned();
        Ok(claim_decision(control, now))
    }
}

pub struct PgH8WorkerRuntimeRepository {
    pool: PgPool,
}

impl PgH8WorkerRuntimeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WorkerRow {
    worker_id: String,
    worker_version: String,
    connector_id: Uuid,
    directions: Vec<String>,
    current_claims: i32,
    created_at: DateTime<Utc>,
    last_heartbeat_at: DateTime<Utc>,
    heartbeat_expires_at: DateTime<Utc>,
}

impl WorkerRow {
    fn into_status(self, now: DateTime<Utc>) -> H8WorkerStatus {
        H8WorkerStatus {
            worker_id: self.worker_id,
            worker_version: self.worker_version,
            connector_id: self.connector_id,
            directions: self.directions,
            current_claims: self.current_claims,
            created_at: self.created_at,
            last_heartbeat_at: self.last_heartbeat_at,
            heartbeat_expires_at: self.heartbeat_expires_at,
            health: derive_worker_health(self.heartbeat_expires_at, now).into(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ControlRow {
    connector_id: Uuid,
    direction: String,
    paused: bool,
    reason: String,
    paused_until: Option<DateTime<Utc>>,
    updated_by: String,
    updated_at: DateTime<Utc>,
}

impl ControlRow {
    fn into_control(self, now: DateTime<Utc>) -> H8WorkerClaimControl {
        H8WorkerClaimControl {
            connector_id: self.connector_id,
            direction: self.direction,
            paused: is_worker_claim_paused(self.paused, self.paused_until, now),
            reason: self.reason,
            paused_until: self.paused_until,
            updated_by: self.updated_by,
            updated_at: self.updated_at,
        }
    }
}

fn db(error: sqlx::Error) -> H8ErpMessageRepoError {
    H8ErpMessageRepoError::Db(error.to_string())
}

#[axum::async_trait]
impl H8WorkerRuntimeRepository for PgH8WorkerRuntimeRepository {
    async fn record_heartbeat(
        &self,
        owner_id: Uuid,
        request: &H8WorkerHeartbeatRequest,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerStatus, H8ErpMessageRepoError> {
        validate_heartbeat(request)?;
        let expires_at = now + chrono::Duration::seconds(request.heartbeat_ttl_seconds);
        let row = sqlx::query_as::<_, WorkerRow>(
            r#"INSERT INTO h8_erp_worker_heartbeats
               (owner_id, worker_id, worker_version, connector_id, directions,
                current_claims, created_at, last_heartbeat_at, heartbeat_expires_at)
               SELECT $1,$2,$3,$4,$5,$6,$7,$7,$8
               FROM h8_erp_connectors WHERE owner_id=$1 AND id=$4
               ON CONFLICT (owner_id, worker_id) DO UPDATE SET
                 worker_version=EXCLUDED.worker_version,
                 connector_id=EXCLUDED.connector_id,
                 directions=EXCLUDED.directions,
                 current_claims=EXCLUDED.current_claims,
                 last_heartbeat_at=EXCLUDED.last_heartbeat_at,
                 heartbeat_expires_at=EXCLUDED.heartbeat_expires_at
               RETURNING worker_id, worker_version, connector_id, directions,
                         current_claims, created_at, last_heartbeat_at, heartbeat_expires_at"#,
        )
        .bind(owner_id)
        .bind(request.worker_id.trim())
        .bind(request.worker_version.trim())
        .bind(request.connector_id)
        .bind(&request.directions)
        .bind(request.current_claims)
        .bind(now)
        .bind(expires_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        Ok(row.into_status(now))
    }

    async fn list_runtime(
        &self,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerRuntimeResponse, H8ErpMessageRepoError> {
        let workers = sqlx::query_as::<_, WorkerRow>(
            r#"SELECT worker_id, worker_version, connector_id, directions,
                      current_claims, created_at, last_heartbeat_at, heartbeat_expires_at
               FROM h8_erp_worker_heartbeats WHERE owner_id=$1 ORDER BY worker_id"#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?
        .into_iter()
        .map(|row| row.into_status(now))
        .collect();
        let controls = sqlx::query_as::<_, ControlRow>(
            r#"SELECT connector_id, direction, paused, reason, paused_until,
                      updated_by, updated_at
               FROM h8_erp_worker_claim_controls
               WHERE owner_id=$1 ORDER BY connector_id, direction"#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?
        .into_iter()
        .map(|row| row.into_control(now))
        .collect();
        Ok(H8WorkerRuntimeResponse { workers, controls })
    }

    async fn set_claim_control(
        &self,
        owner_id: Uuid,
        request: &SetH8WorkerClaimControlRequest,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimControl, H8ErpMessageRepoError> {
        validate_control(request, now)?;
        let connector_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM h8_erp_connectors WHERE owner_id=$1 AND id=$2)",
        )
        .bind(owner_id)
        .bind(request.connector_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        if !connector_exists {
            return Err(H8ErpMessageRepoError::NotFound);
        }
        let row = sqlx::query_as::<_, ControlRow>(
            r#"INSERT INTO h8_erp_worker_claim_controls
               (owner_id, connector_id, direction, paused, reason, paused_until,
                updated_by, updated_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
               ON CONFLICT (owner_id, connector_id, direction) DO UPDATE SET
                 paused=EXCLUDED.paused, reason=EXCLUDED.reason,
                 paused_until=EXCLUDED.paused_until, updated_by=EXCLUDED.updated_by,
                 updated_at=EXCLUDED.updated_at
               RETURNING connector_id, direction, paused, reason, paused_until,
                         updated_by, updated_at"#,
        )
        .bind(owner_id)
        .bind(request.connector_id)
        .bind(request.direction.trim())
        .bind(request.paused)
        .bind(request.reason.trim())
        .bind(if request.paused {
            request.paused_until
        } else {
            None
        })
        .bind(actor)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        Ok(row.into_control(now))
    }

    async fn claim_decision(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        direction: &str,
        now: DateTime<Utc>,
    ) -> Result<H8WorkerClaimDecision, H8ErpMessageRepoError> {
        validate_direction(direction).map_err(H8ErpMessageRepoError::Domain)?;
        let row = sqlx::query_as::<_, ControlRow>(
            r#"SELECT connector_id, direction, paused, reason, paused_until,
                      updated_by, updated_at
               FROM h8_erp_worker_claim_controls
               WHERE owner_id=$1 AND connector_id=$2 AND direction=$3"#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .bind(direction)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        Ok(claim_decision(
            row.map(|value| value.into_control(now)),
            now,
        ))
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn heartbeat_and_pause_survive_repository_restart(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let connector_id = Uuid::new_v4();
        sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
            .bind(owner_id)
            .bind(format!("OWNER-{owner_id}"))
            .bind("H8 test owner")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            r#"INSERT INTO h8_erp_connectors
               (id, owner_id, connector_code, connector_name, directions,
                message_types, channel_mode)
               VALUES ($1,$2,'SELF-ERP','Self ERP',$3,$4,'interface_table')"#,
        )
        .bind(connector_id)
        .bind(owner_id)
        .bind(vec!["inbound".to_string(), "outbound".to_string()])
        .bind(vec!["asn".to_string()])
        .execute(&pool)
        .await
        .unwrap();

        let now = Utc::now();
        let repository = PgH8WorkerRuntimeRepository::new(pool.clone());
        let heartbeat = H8WorkerHeartbeatRequest {
            worker_id: "worker-01".into(),
            worker_version: "1.0.0".into(),
            connector_id,
            directions: vec!["inbound".into(), "outbound".into()],
            current_claims: 2,
            heartbeat_ttl_seconds: 15,
        };
        repository
            .record_heartbeat(owner_id, &heartbeat, now)
            .await
            .unwrap();
        let first_created_at = repository
            .list_runtime(owner_id, now)
            .await
            .unwrap()
            .workers[0]
            .created_at;
        let refreshed = repository
            .record_heartbeat(owner_id, &heartbeat, now + chrono::Duration::seconds(1))
            .await
            .unwrap();
        assert!(matches!(
            repository
                .record_heartbeat(Uuid::new_v4(), &heartbeat, now)
                .await,
            Err(H8ErpMessageRepoError::NotFound)
        ));
        repository
            .set_claim_control(
                owner_id,
                &SetH8WorkerClaimControlRequest {
                    connector_id,
                    direction: "inbound".into(),
                    paused: true,
                    reason: "ERP maintenance".into(),
                    paused_until: Some(now + chrono::Duration::minutes(10)),
                    confirmed: true,
                },
                "admin",
                now,
            )
            .await
            .unwrap();

        let restarted = PgH8WorkerRuntimeRepository::new(pool);
        let runtime = restarted.list_runtime(owner_id, now).await.unwrap();
        assert_eq!(runtime.workers[0].current_claims, 2);
        assert_eq!(runtime.workers[0].created_at, first_created_at);
        assert_eq!(
            runtime.workers[0].last_heartbeat_at,
            refreshed.last_heartbeat_at
        );
        assert_eq!(runtime.workers[0].health, "healthy");
        assert!(runtime.controls[0].paused);
        assert!(
            !restarted
                .claim_decision(owner_id, connector_id, "inbound", now)
                .await
                .unwrap()
                .allowed
        );
        assert_eq!(
            restarted
                .list_runtime(owner_id, now + chrono::Duration::seconds(16))
                .await
                .unwrap()
                .workers[0]
                .health,
            "stale"
        );
        assert!(
            restarted
                .claim_decision(
                    owner_id,
                    connector_id,
                    "inbound",
                    now + chrono::Duration::minutes(11),
                )
                .await
                .unwrap()
                .allowed
        );
    }
}
