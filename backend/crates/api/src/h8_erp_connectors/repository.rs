//! H8 ERP 连接仓储。

use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;
use wms_domain::{
    inflight_status_after_activate, inflight_status_after_disable, H8ErpConnector,
    H8ErpConnectorError, H8_INFLIGHT_PAUSED, H8_INFLIGHT_RUNNING,
};

use super::error::H8ErpConnectorRepoError;
use super::row::H8ErpConnectorRow;

#[axum::async_trait]
pub trait H8ErpConnectorRepository: Send + Sync {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError>;
    async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    /// AC15：`observed_version` 为加载时的 config_version，用于乐观锁。
    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError>;
    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError>;
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
    /// connector_id -> (status, count of messages)
    inflight: Mutex<HashMap<Uuid, Vec<String>>>,
    api_key_scopes: Mutex<HashMap<Uuid, Vec<String>>>,
}

#[axum::async_trait]
impl H8ErpConnectorRepository for MemoryH8ErpConnectorRepository {
    async fn list(&self, owner_id: Uuid) -> Result<Vec<H8ErpConnector>, H8ErpConnectorRepoError> {
        let guard = self.inner.lock().expect("lock");
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

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        if guard.iter().any(|c| {
            c.owner_id == connector.owner_id && c.connector_code == connector.connector_code
        }) {
            return Err(H8ErpConnectorRepoError::DuplicateCode);
        }
        guard.push(connector.clone());
        Ok(connector.clone())
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
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
        *slot = connector.clone();
        Ok(connector.clone())
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        let mut guard = self.inner.lock().expect("lock");
        let before = guard.len();
        guard.retain(|c| !(c.owner_id == owner_id && c.id == id));
        if guard.len() == before {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        Ok(())
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
        let guard = self.inflight.lock().expect("lock");
        Ok(guard
            .get(&connector_id)
            .is_some_and(|rows| !rows.is_empty()))
    }

    async fn pause_inflight(
        &self,
        _owner_id: Uuid,
        connector_id: Uuid,
    ) -> Result<u64, H8ErpConnectorRepoError> {
        let mut guard = self.inflight.lock().expect("lock");
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
        let mut guard = self.inflight.lock().expect("lock");
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
        _owner_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<Option<Vec<String>>, H8ErpConnectorRepoError> {
        let guard = self.api_key_scopes.lock().expect("lock");
        Ok(guard.get(&api_key_id).cloned())
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
        let mut guard = self.inflight.lock().expect("lock");
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

    async fn insert(
        &self,
        connector: &H8ErpConnector,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_connectors (
                id, owner_id, connector_code, connector_name, warehouse_ids, directions,
                message_types, channel_mode, api_base_url, interface_db_host, interface_db_port,
                interface_db_name, interface_db_username, api_key_id, bearer_secret_alias,
                interface_db_password_alias, status, config_version, first_activated_at,
                last_tested_version, last_tested_at, last_tested_succeeded,
                last_tested_error_summary, created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25
            )
            "#,
        )
        .bind(connector.id)
        .bind(connector.owner_id)
        .bind(&connector.connector_code)
        .bind(&connector.connector_name)
        .bind(&connector.warehouse_ids)
        .bind(&connector.directions)
        .bind(&connector.message_types)
        .bind(&connector.channel_mode)
        .bind(&connector.api_base_url)
        .bind(&connector.interface_db_host)
        .bind(connector.interface_db_port)
        .bind(&connector.interface_db_name)
        .bind(&connector.interface_db_username)
        .bind(connector.api_key_id)
        .bind(&connector.bearer_secret_alias)
        .bind(&connector.interface_db_password_alias)
        .bind(&connector.status)
        .bind(connector.config_version)
        .bind(connector.first_activated_at)
        .bind(connector.last_tested_version)
        .bind(connector.last_tested_at)
        .bind(connector.last_tested_succeeded)
        .bind(&connector.last_tested_error_summary)
        .bind(connector.created_at)
        .bind(connector.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("uq_h8_erp_connectors_owner_code") {
                H8ErpConnectorRepoError::DuplicateCode
            } else {
                H8ErpConnectorRepoError::Db(msg)
            }
        })?;
        Ok(connector.clone())
    }

    async fn save(
        &self,
        connector: &H8ErpConnector,
        observed_version: i64,
    ) -> Result<H8ErpConnector, H8ErpConnectorRepoError> {
        let result = sqlx::query(
            r#"
            UPDATE h8_erp_connectors SET
                connector_name = $3,
                warehouse_ids = $4,
                directions = $5,
                message_types = $6,
                channel_mode = $7,
                api_base_url = $8,
                interface_db_host = $9,
                interface_db_port = $10,
                interface_db_name = $11,
                interface_db_username = $12,
                api_key_id = $13,
                bearer_secret_alias = $14,
                interface_db_password_alias = $15,
                status = $16,
                config_version = $17,
                first_activated_at = $18,
                last_tested_version = $19,
                last_tested_at = $20,
                last_tested_succeeded = $21,
                last_tested_error_summary = $22,
                updated_at = $23
             WHERE owner_id = $1 AND id = $2 AND config_version = $24
            "#,
        )
        .bind(connector.owner_id)
        .bind(connector.id)
        .bind(&connector.connector_name)
        .bind(&connector.warehouse_ids)
        .bind(&connector.directions)
        .bind(&connector.message_types)
        .bind(&connector.channel_mode)
        .bind(&connector.api_base_url)
        .bind(&connector.interface_db_host)
        .bind(connector.interface_db_port)
        .bind(&connector.interface_db_name)
        .bind(&connector.interface_db_username)
        .bind(connector.api_key_id)
        .bind(&connector.bearer_secret_alias)
        .bind(&connector.interface_db_password_alias)
        .bind(&connector.status)
        .bind(connector.config_version)
        .bind(connector.first_activated_at)
        .bind(connector.last_tested_version)
        .bind(connector.last_tested_at)
        .bind(connector.last_tested_succeeded)
        .bind(&connector.last_tested_error_summary)
        .bind(connector.updated_at)
        .bind(observed_version)
        .execute(&self.pool)
        .await
        .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        if result.rows_affected() == 0 {
            // 区分「不存在」与「版本冲突」
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM h8_erp_connectors WHERE owner_id = $1 AND id = $2)",
            )
            .bind(connector.owner_id)
            .bind(connector.id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
            return Err(H8ErpConnectorRepoError::Domain(if exists {
                H8ErpConnectorError::VersionConflict
            } else {
                H8ErpConnectorError::NotFound
            }));
        }
        Ok(connector.clone())
    }

    async fn delete(&self, owner_id: Uuid, id: Uuid) -> Result<(), H8ErpConnectorRepoError> {
        let result = sqlx::query("DELETE FROM h8_erp_connectors WHERE owner_id = $1 AND id = $2")
            .bind(owner_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| H8ErpConnectorRepoError::Db(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(H8ErpConnectorRepoError::Domain(
                H8ErpConnectorError::NotFound,
            ));
        }
        Ok(())
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
             WHERE owner_id = $1 AND id = $2 AND status = 'active'
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
