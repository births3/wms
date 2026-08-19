//! H8 ERP 连接仓储。

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    inflight_status_after_activate, inflight_status_after_disable, H8ErpConnector,
    H8ErpConnectorError, H8ErpConnectorRuntimeConfig, H8ErpConnectorTestResult, H8_INFLIGHT_PAUSED,
    H8_INFLIGHT_RUNNING,
};

use super::error::H8ErpConnectorRepoError;
use super::idempotency::H8IdempotencyWrite;
use super::persistence;
use super::row::H8ErpConnectorRow;
use crate::audit::AuditWriteRequest;
use crate::sync::lock_recover;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum H8ConnectorStatusTransition {
    Activate,
    Disable,
}

impl H8ConnectorStatusTransition {
    pub(super) fn inflight_statuses(self) -> (&'static str, &'static str) {
        match self {
            Self::Activate => (H8_INFLIGHT_PAUSED, H8_INFLIGHT_RUNNING),
            Self::Disable => (H8_INFLIGHT_RUNNING, H8_INFLIGHT_PAUSED),
        }
    }
}

#[axum::async_trait]
pub trait H8ErpConnectorRepository: Send + Sync {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError>;
    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn get_version(
        &self,
        owner_id: Uuid,
        id: Uuid,
        config_version: i64,
    ) -> Result<H8ErpConnectorRuntimeConfig, H8ErpConnectorRepoError>;
    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn commit_create(
        &self,
        connector: &H8ErpConnector,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    /// AC15：transport 与 probe 版本都必须匹配加载时版本，用于乐观锁。
    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn commit_update(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    #[allow(clippy::too_many_arguments)]
    async fn commit_test(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        result: &H8ErpConnectorTestResult,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnectorTestResult, H8ErpConnectorRepoError>;
    /// 连接状态、在途状态、审计与幂等响应必须在同一事务中提交。
    #[allow(clippy::too_many_arguments)]
    async fn commit_status_transition(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        transition: H8ConnectorStatusTransition,
        audit_request: &AuditWriteRequest,
        inflight_audit_request: Option<&AuditWriteRequest>,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(H8ErpConnector, u64), H8ErpConnectorRepoError>;
    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError>;
    async fn commit_delete(
        &self,
        owner_id: Uuid,
        id: Uuid,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(), H8ErpConnectorRepoError>;
    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError>;
    async fn has_inflight_refs(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError>;
    async fn pause_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError>;
    /// AC12：再启用后将 paused 在途恢复为 running。
    async fn resume_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError>;
    /// AC9：读取入站 API Key 的 scopes（无记录时返回 None）。
    async fn load_api_key_scopes(
        &self,
        owner_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<Option<Vec<String>>, H8ErpConnectorRepoError>;
    /// AC12：绑定在途消息（Worker/运行时调用；测试可写入）。
    async fn bind_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        idempotency_key: &str,
        direction: &str,
        message_type: &str,
        channel_stage: &str,
        status: &str,
    ) -> Result<(), H8ErpConnectorRepoError>;
}

#[derive(Default)]
pub(crate) struct MemoryH8ErpConnectorRepository {
    inner: Mutex<Vec<H8ErpConnector>>,
    versions: Mutex<HashMap<(Uuid, Uuid, i64), H8ErpConnectorRuntimeConfig>>,
    /// connector_id -> (status, count of messages)
    inflight: Mutex<HashMap<Uuid, Vec<String>>>,
    api_key_scopes: Mutex<HashMap<(Uuid, Uuid), Vec<String>>>,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for MemoryH8ErpConnectorRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let guard = lock_recover(&self.inner);
        Ok(guard
            .iter()
            .filter(|c| c.owner_id == owner_id)
            .cloned()
            .collect())
    }

    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.list(owner_id)
            .await?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ))
    }

    async fn get_version(
        &self,
        owner_id: Uuid,
        id: Uuid,
        config_version: i64,
    ) -> Result<H8ErpConnectorRuntimeConfig, H8ErpConnectorRepoError> {
        lock_recover(&self.versions)
            .get(&(owner_id, id, config_version))
            .cloned()
            .ok_or(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ))
    }

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inner);
        if guard.iter().any(|c| {
            c.owner_id == connector.owner_id && c.connector_code == connector.connector_code
        }) {
            return Err(H8ErpConnectorRepoError::DuplicateCode);
        }
        guard.push(connector.clone());
        lock_recover(&self.versions).insert(
            (connector.owner_id, connector.id, connector.config_version),
            connector.into(),
        );
        Ok(connector.clone())
    }

    async fn commit_create(
        &self,
        connector: &H8ErpConnector,
        _audit_request: &AuditWriteRequest,
        _idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.insert(connector).await
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inner);
        let Some(slot) = guard
            .iter_mut()
            .find(|c| c.id == connector.id && c.owner_id == connector.owner_id)
        else {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        };
        if slot.config_version != observed_version {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::VersionConflict,
            ));
        }
        if slot.interface_probe_config_version != observed_probe_version {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::ProbeVersionConflict,
            ));
        }
        *slot = connector.clone();
        lock_recover(&self.versions)
            .entry((connector.owner_id, connector.id, connector.config_version))
            .or_insert_with(|| connector.into());
        Ok(connector.clone())
    }

    async fn commit_update(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        _audit_request: &AuditWriteRequest,
        _idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        self.save(connector, observed_version, observed_probe_version)
            .await
    }

    async fn commit_test(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        result: &H8ErpConnectorTestResult,
        _audit_request: &AuditWriteRequest,
        _idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnectorTestResult, H8ErpConnectorRepoError> {
        self.save(connector, observed_version, observed_probe_version)
            .await?;
        Ok(result.clone())
    }

    async fn commit_status_transition(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        transition: H8ConnectorStatusTransition,
        _audit_request: &AuditWriteRequest,
        _inflight_audit_request: Option<&AuditWriteRequest>,
        _idempotency: &H8IdempotencyWrite,
    ) -> Result<(H8ErpConnector, u64), H8ErpConnectorRepoError> {
        let saved = self
            .save(connector, observed_version, observed_probe_version)
            .await?;
        let affected = match transition {
            H8ConnectorStatusTransition::Activate => {
                self.resume_inflight(connector.owner_id, connector.id)
                    .await?
            }
            H8ConnectorStatusTransition::Disable => {
                self.pause_inflight(connector.owner_id, connector.id)
                    .await?
            }
        };
        Ok((saved, affected))
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inner);
        let before = guard.len();
        guard.retain(|c| !(c.owner_id == owner_id && c.id == id));
        if guard.len() == before {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        lock_recover(&self.versions).retain(|(snapshot_owner, connector_id, _), _| {
            *snapshot_owner != owner_id || *connector_id != id
        });
        Ok(())
    }

    async fn commit_delete(
        &self,
        owner_id: Uuid,
        id: Uuid,
        _audit_request: &AuditWriteRequest,
        _idempotency: &H8IdempotencyWrite,
    ) -> Result<(), H8ErpConnectorRepoError> {
        self.delete(owner_id, id).await
    }

    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        Ok(self
            .list(owner_id)
            .await?
            .into_iter()
            .filter(|c| c.status == "active")
            .collect())
    }

    async fn has_inflight_refs(
        &self,
        _owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError> {
        let guard = lock_recover(&self.inflight);
        Ok(guard
            .get(&connector_id)
            .is_some_and(|rows| !rows.is_empty()))
    }

    async fn pause_inflight(
        &self,
        _owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inflight);
        let Some(rows) = guard.get_mut(&connector_id) else {
            return Ok(0);
        };
        let mut n = 0u64;
        for status in rows.iter_mut() {
            if let Some(next) = inflight_status_after_disable(status) {
                *status = next.to_string();
                n += 1;
            }
        }
        Ok(n)
    }

    async fn resume_inflight(
        &self,
        _owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inflight);
        let Some(rows) = guard.get_mut(&connector_id) else {
            return Ok(0);
        };
        let mut n = 0u64;
        for status in rows.iter_mut() {
            if let Some(next) = inflight_status_after_activate(status) {
                *status = next.to_string();
                n += 1;
            }
        }
        Ok(n)
    }

    async fn load_api_key_scopes(
        &self,
        owner_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<Option<Vec<String>>, H8ErpConnectorRepoError> {
        let guard = lock_recover(&self.api_key_scopes);
        Ok(guard.get(&(owner_id, api_key_id)).cloned())
    }

    async fn bind_inflight(
        &self,
        _owner_id: Uuid,
        connector_id: Uuid,
        idempotency_key: &str,
        _direction: &str,
        _message_type: &str,
        _channel_stage: &str,
        status: &str,
    ) -> Result<(), H8ErpConnectorRepoError> {
        let mut guard = lock_recover(&self.inflight);
        let rows = guard.entry(connector_id).or_default();
        // 以幂等键为索引时内存实现简化为追加 status 行
        let _ = idempotency_key;
        rows.push(status.to_string());
        Ok(())
    }
}

pub(crate) struct PgH8ErpConnectorRepository {
    pub(crate) pool: PgPool,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for PgH8ErpConnectorRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let rows = sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1
             ORDER BY updated_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?
        .map(Into::into)
        .ok_or(H8ErpConnectorRepoError::Domain(
            H8ErpConnectorError::NotFound,
        ))
    }

    async fn get_version(
        &self,
        owner_id: Uuid,
        id: Uuid,
        config_version: i64,
    ) -> Result<H8ErpConnectorRuntimeConfig, H8ErpConnectorRepoError> {
        let value: Option<serde_json::Value> = sqlx::query_scalar(
            r#"
            SELECT runtime_config
              FROM h8_erp_connector_versions
             WHERE owner_id = $1 AND connector_id = $2 AND config_version = $3
            "#,
        )
        .bind(owner_id)
        .bind(id)
        .bind(config_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        value
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?
            .ok_or(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ))
    }

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        persistence::insert(&self.pool, connector).await
    }

    async fn commit_create(
        &self,
        connector: &H8ErpConnector,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        persistence::commit_create(&self.pool, connector, audit_request, idempotency).await
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        persistence::save(
            &self.pool,
            connector,
            observed_version,
            observed_probe_version,
        )
        .await
    }

    async fn commit_update(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        persistence::commit_update(
            &self.pool,
            connector,
            observed_version,
            observed_probe_version,
            audit_request,
            idempotency,
        )
        .await
    }

    async fn commit_test(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        result: &H8ErpConnectorTestResult,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<H8ErpConnectorTestResult, H8ErpConnectorRepoError> {
        persistence::commit_test(
            &self.pool,
            connector,
            observed_version,
            observed_probe_version,
            result,
            audit_request,
            idempotency,
        )
        .await
    }

    async fn commit_status_transition(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
        observed_probe_version: i64,
        transition: H8ConnectorStatusTransition,
        audit_request: &AuditWriteRequest,
        inflight_audit_request: Option<&AuditWriteRequest>,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(H8ErpConnector, u64), H8ErpConnectorRepoError> {
        persistence::commit_status_transition(
            &self.pool,
            connector,
            observed_version,
            observed_probe_version,
            transition,
            audit_request,
            inflight_audit_request,
            idempotency,
        )
        .await
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        persistence::delete(&self.pool, owner_id, id).await
    }

    async fn commit_delete(
        &self,
        owner_id: Uuid,
        id: Uuid,
        audit_request: &AuditWriteRequest,
        idempotency: &H8IdempotencyWrite,
    ) -> Result<(), H8ErpConnectorRepoError> {
        persistence::commit_delete(&self.pool, owner_id, id, audit_request, idempotency).await
    }

    async fn list_active(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let rows = sqlx::query_as::<_, H8ErpConnectorRow>(
            r#"
            SELECT * FROM h8_erp_connectors
             WHERE owner_id = $1 AND status = 'active'
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn has_inflight_refs(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<bool, H8ErpConnectorRepoError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM h8_erp_in_flight_messages
             WHERE owner_id = $1 AND connector_id = $2
            "#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(count > 0)
    }

    async fn pause_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let result = sqlx::query(
            r#"
            UPDATE h8_erp_in_flight_messages
               SET status = $3, updated_at = now()
             WHERE owner_id = $1 AND connector_id = $2 AND status = $4
            "#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .bind(H8_INFLIGHT_PAUSED)
        .bind(H8_INFLIGHT_RUNNING)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn resume_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let result = sqlx::query(
            r#"
            UPDATE h8_erp_in_flight_messages
               SET status = $3, updated_at = now()
             WHERE owner_id = $1 AND connector_id = $2 AND status = $4
            "#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .bind(H8_INFLIGHT_RUNNING)
        .bind(H8_INFLIGHT_PAUSED)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn load_api_key_scopes(
        &self,
        owner_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<Option<Vec<String>>, H8ErpConnectorRepoError> {
        let scopes: Option<Vec<String>> = sqlx::query_scalar(
            r#"
            SELECT scopes FROM auth_api_keys
             WHERE owner_id = $1
               AND id = $2
               AND status = 'active'
               AND expires_at > now()
               AND (grace_expires_at IS NULL OR grace_expires_at > now())
            "#,
        )
        .bind(owner_id)
        .bind(api_key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(scopes)
    }

    async fn bind_inflight(
        &self,
        owner_id: Uuid,
        connector_id: Uuid,
        idempotency_key: &str,
        direction: &str,
        message_type: &str,
        channel_stage: &str,
        status: &str,
    ) -> Result<(), H8ErpConnectorRepoError> {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_in_flight_messages (
                id, owner_id, connector_id, idempotency_key, direction, message_type,
                channel_stage, status, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now(), now())
            ON CONFLICT (owner_id, idempotency_key) DO UPDATE SET
                status = EXCLUDED.status,
                channel_stage = EXCLUDED.channel_stage,
                updated_at = now()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(connector_id)
        .bind(idempotency_key)
        .bind(direction)
        .bind(message_type)
        .bind(channel_stage)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        Ok(())
    }
}
