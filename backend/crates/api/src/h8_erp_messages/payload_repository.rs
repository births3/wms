//! US-H8-003 AC16：按连接短期加密保留完整报文。

use std::{collections::HashMap, sync::Mutex};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    resolve_payload_retention_days, H8DecryptedPayload, H8MessageError, H8PayloadRetentionPolicy,
    UpdateH8PayloadRetentionPolicyRequest,
};

use super::error::H8ErpMessageRepoError;

#[axum::async_trait]
pub trait H8PayloadRepository: Send + Sync {
    async fn list_policies(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8PayloadRetentionPolicy>, H8ErpMessageRepoError>;

    async fn update_policy(
        &self,
        owner_id: Uuid,
        request: &UpdateH8PayloadRetentionPolicyRequest,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8PayloadRetentionPolicy, H8ErpMessageRepoError>;

    async fn capture_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        connector_id: Uuid,
        payload: &str,
        master_key: Option<&str>,
        key_version: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, H8ErpMessageRepoError>;

    async fn payload_status(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(bool, Option<DateTime<Utc>>), H8ErpMessageRepoError>;

    async fn decrypt_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        master_keys: &HashMap<String, String>,
        now: DateTime<Utc>,
    ) -> Result<H8DecryptedPayload, H8ErpMessageRepoError>;

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, H8ErpMessageRepoError>;
}

fn validate_policy(
    request: &UpdateH8PayloadRetentionPolicyRequest,
) -> Result<i32, H8ErpMessageRepoError> {
    if !request.confirmed {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::FieldRequired("confirmed"),
        ));
    }
    resolve_payload_retention_days(request.retention_days).map_err(H8ErpMessageRepoError::Domain)
}

fn require_master_key(master_key: Option<&str>) -> Result<&str, H8ErpMessageRepoError> {
    master_key
        .filter(|value| value.len() >= 32)
        .ok_or(H8ErpMessageRepoError::Domain(
            H8MessageError::EncryptionKeyUnavailable,
        ))
}

fn require_key_version(key_version: &str) -> Result<&str, H8ErpMessageRepoError> {
    let key_version = key_version.trim();
    if key_version.is_empty() || key_version.len() > 64 {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::EncryptionKeyUnavailable,
        ));
    }
    Ok(key_version)
}

#[derive(Default)]
pub struct MemoryH8PayloadRepository {
    policies: Mutex<HashMap<(Uuid, Uuid), H8PayloadRetentionPolicy>>,
    payloads: Mutex<HashMap<(Uuid, Uuid), (Uuid, String, DateTime<Utc>, String)>>,
}

#[axum::async_trait]
impl H8PayloadRepository for MemoryH8PayloadRepository {
    async fn list_policies(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8PayloadRetentionPolicy>, H8ErpMessageRepoError> {
        Ok(self
            .policies
            .lock()
            .expect("payload policy lock")
            .iter()
            .filter(|((owner, _), _)| *owner == owner_id)
            .map(|(_, policy)| policy.clone())
            .collect())
    }

    async fn update_policy(
        &self,
        owner_id: Uuid,
        request: &UpdateH8PayloadRetentionPolicyRequest,
        _actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8PayloadRetentionPolicy, H8ErpMessageRepoError> {
        let retention_days = validate_policy(request)?;
        let policy = H8PayloadRetentionPolicy {
            connector_id: request.connector_id,
            enabled: request.enabled,
            retention_days,
            updated_at: now,
        };
        self.policies
            .lock()
            .expect("payload policy lock")
            .insert((owner_id, request.connector_id), policy.clone());
        if !request.enabled {
            self.payloads.lock().expect("payload lock").retain(
                |(owner, _), (connector, _, _, _)| {
                    *owner != owner_id || *connector != request.connector_id
                },
            );
        }
        Ok(policy)
    }

    async fn capture_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        connector_id: Uuid,
        payload: &str,
        master_key: Option<&str>,
        key_version: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, H8ErpMessageRepoError> {
        let policy = self
            .policies
            .lock()
            .expect("payload policy lock")
            .get(&(owner_id, connector_id))
            .cloned();
        let Some(policy) = policy.filter(|value| value.enabled) else {
            return Ok(false);
        };
        require_master_key(master_key)?;
        let key_version = require_key_version(key_version)?;
        self.payloads.lock().expect("payload lock").insert(
            (owner_id, message_id),
            (
                connector_id,
                payload.to_string(),
                now + chrono::Duration::days(i64::from(policy.retention_days)),
                key_version.to_string(),
            ),
        );
        Ok(true)
    }

    async fn payload_status(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(bool, Option<DateTime<Utc>>), H8ErpMessageRepoError> {
        let expires = self
            .payloads
            .lock()
            .expect("payload lock")
            .get(&(owner_id, message_id))
            .map(|(_, _, expires, _)| *expires);
        Ok((expires.is_some_and(|value| value > now), expires))
    }

    async fn decrypt_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        master_keys: &HashMap<String, String>,
        now: DateTime<Utc>,
    ) -> Result<H8DecryptedPayload, H8ErpMessageRepoError> {
        let value = self
            .payloads
            .lock()
            .expect("payload lock")
            .get(&(owner_id, message_id))
            .cloned()
            .ok_or(H8ErpMessageRepoError::Domain(
                H8MessageError::PayloadUnavailable,
            ))?;
        if value.2 <= now {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::PayloadExpired,
            ));
        }
        require_master_key(master_keys.get(&value.3).map(String::as_str))?;
        Ok(H8DecryptedPayload {
            message_id,
            payload: value.1,
            expires_at: value.2,
        })
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, H8ErpMessageRepoError> {
        let mut payloads = self.payloads.lock().expect("payload lock");
        let before = payloads.len();
        payloads.retain(|_, (_, _, expires, _)| *expires > now);
        Ok((before - payloads.len()) as u64)
    }
}

pub struct PgH8PayloadRepository {
    pool: PgPool,
}

#[derive(sqlx::FromRow)]
struct PolicyRow {
    connector_id: Uuid,
    enabled: bool,
    retention_days: i32,
    updated_at: DateTime<Utc>,
}

impl From<PolicyRow> for H8PayloadRetentionPolicy {
    fn from(value: PolicyRow) -> Self {
        Self {
            connector_id: value.connector_id,
            enabled: value.enabled,
            retention_days: value.retention_days,
            updated_at: value.updated_at,
        }
    }
}

impl PgH8PayloadRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db(error: sqlx::Error) -> H8ErpMessageRepoError {
    H8ErpMessageRepoError::Db(error.to_string())
}

#[axum::async_trait]
impl H8PayloadRepository for PgH8PayloadRepository {
    async fn list_policies(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<H8PayloadRetentionPolicy>, H8ErpMessageRepoError> {
        sqlx::query_as::<_, PolicyRow>(
            r#"SELECT connector_id, enabled, retention_days, updated_at
               FROM h8_erp_payload_retention_policies
               WHERE owner_id=$1 ORDER BY connector_id"#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(db)
    }

    async fn update_policy(
        &self,
        owner_id: Uuid,
        request: &UpdateH8PayloadRetentionPolicyRequest,
        actor: &str,
        now: DateTime<Utc>,
    ) -> Result<H8PayloadRetentionPolicy, H8ErpMessageRepoError> {
        let retention_days = validate_policy(request)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let policy = sqlx::query_as::<_, PolicyRow>(
            r#"INSERT INTO h8_erp_payload_retention_policies
               (owner_id, connector_id, enabled, retention_days, updated_by, updated_at)
               SELECT $1,$2,$3,$4,$5,$6 FROM h8_erp_connectors
               WHERE owner_id=$1 AND id=$2
               ON CONFLICT (owner_id, connector_id) DO UPDATE SET
                 enabled=EXCLUDED.enabled, retention_days=EXCLUDED.retention_days,
                 updated_by=EXCLUDED.updated_by, updated_at=EXCLUDED.updated_at
               RETURNING connector_id, enabled, retention_days, updated_at"#,
        )
        .bind(owner_id)
        .bind(request.connector_id)
        .bind(request.enabled)
        .bind(retention_days)
        .bind(actor)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(H8ErpMessageRepoError::NotFound)?
        .into();
        if !request.enabled {
            sqlx::query(
                r#"UPDATE h8_erp_messages
                   SET encrypted_payload=NULL, payload_key_version=NULL, payload_expires_at=NULL
                   WHERE owner_id=$1 AND connector_id=$2 AND encrypted_payload IS NOT NULL"#,
            )
            .bind(owner_id)
            .bind(request.connector_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(policy)
    }

    async fn capture_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        connector_id: Uuid,
        payload: &str,
        master_key: Option<&str>,
        key_version: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, H8ErpMessageRepoError> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        let policy: Option<(bool, i32)> = sqlx::query_as(
            r#"SELECT enabled, retention_days
               FROM h8_erp_payload_retention_policies
               WHERE owner_id=$1 AND connector_id=$2
               FOR SHARE"#,
        )
        .bind(owner_id)
        .bind(connector_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let enabled = policy.is_some_and(|value| value.0);
        if !enabled {
            let updated = sqlx::query(
                r#"UPDATE h8_erp_messages
                   SET payload_digest=encode(digest($3,'sha256'),'hex')
                   WHERE owner_id=$1 AND id=$2 AND connector_id=$4"#,
            )
            .bind(owner_id)
            .bind(message_id)
            .bind(payload)
            .bind(connector_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if updated.rows_affected() == 0 {
                return Err(H8ErpMessageRepoError::NotFound);
            }
            tx.commit().await.map_err(db)?;
            return Ok(false);
        }
        let master_key = require_master_key(master_key)?;
        let key_version = require_key_version(key_version)?;
        let retention_days = policy.expect("enabled policy").1;
        let expires_at = now + chrono::Duration::days(i64::from(retention_days));
        let updated = sqlx::query(
            r#"UPDATE h8_erp_messages
               SET payload_digest=encode(digest($4,'sha256'),'hex'),
                   encrypted_payload=pgp_sym_encrypt(
                     $4,
                     encode(hmac(
                       $3::text || ':' || $6,
                       encode(hmac($1::text, $5, 'sha256'),'hex'),
                       'sha256'
                     ),'hex'),
                     'cipher-algo=aes256,compress-algo=1'
                   ),
                   payload_key_version=$6,
                   payload_expires_at=$7
               WHERE owner_id=$1 AND id=$2 AND connector_id=$3"#,
        )
        .bind(owner_id)
        .bind(message_id)
        .bind(connector_id)
        .bind(payload)
        .bind(master_key)
        .bind(key_version)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if updated.rows_affected() == 0 {
            return Err(H8ErpMessageRepoError::NotFound);
        }
        tx.commit().await.map_err(db)?;
        Ok(true)
    }

    async fn payload_status(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(bool, Option<DateTime<Utc>>), H8ErpMessageRepoError> {
        let expires = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT payload_expires_at FROM h8_erp_messages WHERE owner_id=$1 AND id=$2",
        )
        .bind(owner_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        Ok((expires.is_some_and(|value| value > now), expires))
    }

    async fn decrypt_payload(
        &self,
        owner_id: Uuid,
        message_id: Uuid,
        master_keys: &HashMap<String, String>,
        now: DateTime<Utc>,
    ) -> Result<H8DecryptedPayload, H8ErpMessageRepoError> {
        let metadata = sqlx::query_as::<_, (Option<Uuid>, Option<String>, Option<DateTime<Utc>>)>(
            r#"SELECT connector_id, payload_key_version, payload_expires_at
               FROM h8_erp_messages WHERE owner_id=$1 AND id=$2"#,
        )
        .bind(owner_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or(H8ErpMessageRepoError::NotFound)?;
        metadata.0.ok_or(H8ErpMessageRepoError::Domain(
            H8MessageError::PayloadUnavailable,
        ))?;
        let key_version = metadata.1.ok_or(H8ErpMessageRepoError::Domain(
            H8MessageError::PayloadUnavailable,
        ))?;
        let master_key = require_master_key(master_keys.get(&key_version).map(String::as_str))?;
        let expires_at = metadata.2.ok_or(H8ErpMessageRepoError::Domain(
            H8MessageError::PayloadUnavailable,
        ))?;
        if expires_at <= now {
            return Err(H8ErpMessageRepoError::Domain(
                H8MessageError::PayloadExpired,
            ));
        }
        let payload = sqlx::query_scalar::<_, String>(
            r#"SELECT pgp_sym_decrypt(
                 encrypted_payload,
                 encode(hmac(
                   connector_id::text || ':' || $4,
                   encode(hmac(owner_id::text, $3, 'sha256'),'hex'),
                   'sha256'
                 ),'hex')
               )
               FROM h8_erp_messages
               WHERE owner_id=$1 AND id=$2 AND payload_key_version=$4"#,
        )
        .bind(owner_id)
        .bind(message_id)
        .bind(master_key)
        .bind(&key_version)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            tracing::error!(?error, message_id = %message_id, "H8 完整报文解密失败");
            H8ErpMessageRepoError::Domain(H8MessageError::EncryptionKeyUnavailable)
        })?;
        Ok(H8DecryptedPayload {
            message_id,
            payload,
            expires_at,
        })
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, H8ErpMessageRepoError> {
        sqlx::query(
            r#"UPDATE h8_erp_messages
               SET encrypted_payload=NULL, payload_key_version=NULL, payload_expires_at=NULL
               WHERE encrypted_payload IS NOT NULL AND payload_expires_at <= $1"#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(db)
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn payload_is_encrypted_decrypted_and_purged_without_deleting_message(pool: PgPool) {
        let owner_id = Uuid::new_v4();
        let connector_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
            .bind(owner_id)
            .bind(format!("OWNER-{owner_id}"))
            .bind("H8 payload test")
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
        .bind(vec!["inbound".to_string()])
        .bind(vec!["asn".to_string()])
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO h8_erp_messages
               (id, owner_id, connector_id, connector_code, config_version, direction,
                message_type, schema_version, channel, external_ref, idempotency_key,
                correlation_id, payload_digest)
               VALUES ($1,$2,$3,'SELF-ERP',1,'inbound','asn','1','interface_table',
                       'ERP-ASN-1','idem-1','corr-1','initial')"#,
        )
        .bind(message_id)
        .bind(owner_id)
        .bind(connector_id)
        .execute(&pool)
        .await
        .unwrap();

        let repository = PgH8PayloadRepository::new(pool.clone());
        let now = Utc::now();
        let payload = r#"{"patient":"张三","qty":1}"#;
        assert!(!repository
            .capture_payload(owner_id, message_id, connector_id, payload, None, "v1", now)
            .await
            .unwrap());
        let encrypted: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT encrypted_payload FROM h8_erp_messages WHERE id=$1")
                .bind(message_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(encrypted.is_none());

        repository
            .update_policy(
                owner_id,
                &UpdateH8PayloadRetentionPolicyRequest {
                    connector_id,
                    enabled: true,
                    retention_days: None,
                    confirmed: true,
                },
                "admin",
                now,
            )
            .await
            .unwrap();
        let key = "k".repeat(32);
        assert!(repository
            .capture_payload(
                owner_id,
                message_id,
                connector_id,
                payload,
                Some(&key),
                "v1",
                now,
            )
            .await
            .unwrap());
        let encrypted: Vec<u8> =
            sqlx::query_scalar("SELECT encrypted_payload FROM h8_erp_messages WHERE id=$1")
                .bind(message_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_ne!(encrypted, payload.as_bytes());
        let wrong_key = repository
            .decrypt_payload(
                owner_id,
                message_id,
                &HashMap::from([("v1".into(), "x".repeat(32))]),
                now,
            )
            .await
            .unwrap_err();
        assert!(matches!(
            wrong_key,
            H8ErpMessageRepoError::Domain(H8MessageError::EncryptionKeyUnavailable)
        ));
        let decrypted = repository
            .decrypt_payload(
                owner_id,
                message_id,
                &HashMap::from([("v1".into(), key.clone()), ("v2".into(), "a".repeat(32))]),
                now,
            )
            .await
            .unwrap();
        assert_eq!(decrypted.payload, payload);
        assert!((decrypted.expires_at - (now + chrono::Duration::days(7)))
            .num_microseconds()
            .is_some_and(|delta| delta.abs() <= 1));

        repository
            .update_policy(
                owner_id,
                &UpdateH8PayloadRetentionPolicyRequest {
                    connector_id,
                    enabled: false,
                    retention_days: None,
                    confirmed: true,
                },
                "admin",
                now,
            )
            .await
            .unwrap();
        assert!(
            !repository
                .payload_status(owner_id, message_id, now)
                .await
                .unwrap()
                .0
        );
        repository
            .update_policy(
                owner_id,
                &UpdateH8PayloadRetentionPolicyRequest {
                    connector_id,
                    enabled: true,
                    retention_days: None,
                    confirmed: true,
                },
                "admin",
                now,
            )
            .await
            .unwrap();
        repository
            .capture_payload(
                owner_id,
                message_id,
                connector_id,
                payload,
                Some(&key),
                "v1",
                now,
            )
            .await
            .unwrap();

        assert_eq!(
            repository
                .purge_expired(now + chrono::Duration::days(8))
                .await
                .unwrap(),
            1
        );
        let (retained, _) = repository
            .payload_status(owner_id, message_id, now + chrono::Duration::days(8))
            .await
            .unwrap();
        assert!(!retained);
        let message_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM h8_erp_messages WHERE id=$1)")
                .bind(message_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(message_exists);
    }
}
