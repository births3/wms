//! Wave 1 H1 auth persistence.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{AuthSession, CurrentUser};

use crate::{
    audit::{append_event, append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext,
};

#[derive(Clone)]
pub struct AuthRepository {
    pool: PgPool,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_login_user(
        &self,
        owner_code: Option<&str>,
        username: &str,
    ) -> Result<Option<LoginUser>, AuthRepositoryError> {
        let trimmed_code = owner_code.map(str::trim).filter(|c| !c.is_empty());
        match trimmed_code {
            Some(code) => sqlx::query_as::<_, LoginUser>(
                r#"
                SELECT
                    u.id AS user_id,
                    o.id AS owner_id,
                    o.owner_code,
                    u.username,
                    u.display_name,
                    u.password_hash,
                    u.status,
                    u.locked_until
                  FROM auth_users u
                  JOIN auth_user_owner_bindings b
                    ON b.user_id = u.id
                   AND b.is_active = true
                  JOIN auth_owners o
                    ON o.id = b.owner_id
                 WHERE lower(o.owner_code) = lower($1)
                   AND lower(u.username) = lower($2)
                 LIMIT 1
                "#,
            )
            .bind(code)
            .bind(username.trim())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| AuthRepositoryError::Database),
            None => sqlx::query_as::<_, LoginUser>(
                r#"
                SELECT
                    u.id AS user_id,
                    o.id AS owner_id,
                    o.owner_code,
                    u.username,
                    u.display_name,
                    u.password_hash,
                    u.status,
                    u.locked_until
                  FROM auth_users u
                  JOIN auth_user_owner_bindings b
                    ON b.user_id = u.id
                   AND b.is_active = true
                  JOIN auth_owners o
                    ON o.id = b.owner_id
                 WHERE lower(u.username) = lower($1)
                 ORDER BY b.is_primary DESC, b.created_at ASC
                 LIMIT 1
                "#,
            )
            .bind(username.trim())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| AuthRepositoryError::Database),
        }
    }

    pub async fn current_user(
        &self,
        user_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<CurrentUser>, AuthRepositoryError> {
        let Some(row) = sqlx::query_as::<_, LoginUser>(
            r#"
            SELECT
                u.id AS user_id,
                o.id AS owner_id,
                o.owner_code,
                u.username,
                u.display_name,
                u.password_hash,
                u.status,
                u.locked_until
              FROM auth_users u
              JOIN auth_user_owner_bindings b
                ON b.user_id = u.id
               AND b.owner_id = $2
               AND b.is_active = true
              JOIN auth_owners o
                ON o.id = b.owner_id
             WHERE u.id = $1
             LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)?
        else {
            return Ok(None);
        };

        let roles = self.role_codes(user_id, owner_id).await?;
        let permissions = self.permission_codes(user_id, owner_id).await?;

        Ok(Some(CurrentUser {
            user_id: row.user_id,
            owner_id: row.owner_id,
            owner_code: row.owner_code,
            username: row.username,
            display_name: row.display_name,
            roles,
            permissions,
        }))
    }

    pub async fn record_failed_login(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AuthRepositoryError> {
        let locked_until = now + chrono::Duration::minutes(15);
        sqlx::query(
            r#"
            UPDATE auth_users
               SET failed_login_count = failed_login_count + 1,
                   status = CASE WHEN failed_login_count + 1 >= 5 THEN 'locked' ELSE status END,
                   locked_until = CASE WHEN failed_login_count + 1 >= 5 THEN $2 ELSE locked_until END,
                   updated_at = $3
             WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(locked_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        Ok(())
    }

    pub async fn reset_login_failures(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AuthRepositoryError> {
        sqlx::query(
            r#"
            UPDATE auth_users
               SET failed_login_count = 0,
                   status = 'active',
                   locked_until = NULL,
                   updated_at = $2
             WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        Ok(())
    }

    pub async fn password_hash(
        &self,
        actor: &OperationContext,
    ) -> Result<Option<String>, AuthRepositoryError> {
        sqlx::query_scalar(
            "SELECT u.password_hash FROM auth_users u JOIN auth_user_owner_bindings b ON b.user_id=u.id AND b.owner_id=$1 AND b.is_active WHERE u.id=$2",
        )
        .bind(actor.owner_id)
        .bind(actor.user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }

    pub async fn change_password(
        &self,
        actor: &OperationContext,
        new_password_hash: &str,
        changed_at: DateTime<Utc>,
    ) -> Result<bool, AuthRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        let old_password = sqlx::query_scalar::<_, String>(
            "SELECT u.password_hash FROM auth_users u JOIN auth_user_owner_bindings b ON b.user_id=u.id AND b.owner_id=$1 AND b.is_active WHERE u.id=$2 FOR UPDATE",
        )
        .bind(actor.owner_id)
        .bind(actor.user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        if old_password.is_none() {
            tx.commit()
                .await
                .map_err(|_| AuthRepositoryError::Database)?;
            return Ok(false);
        }
        sqlx::query(
            "UPDATE auth_users SET password_hash=$3, permissions_changed_at=$4, updated_at=$4 WHERE id=$2 AND EXISTS (SELECT 1 FROM auth_user_owner_bindings WHERE owner_id=$1 AND user_id=auth_users.id AND is_active)",
        )
        .bind(actor.owner_id)
        .bind(actor.user_id)
        .bind(new_password_hash)
        .bind(changed_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                actor,
                "auth.password.changed",
                "H1",
                "auth_user",
                actor.user_id.to_string(),
                Some(AuditDiff::compute(
                    serde_json::json!({"credential": "present"}),
                    serde_json::json!({"credential": "changed", "reason": "用户修改密码"}),
                )),
            ),
        )
        .await
        .map_err(|_| AuthRepositoryError::Audit)?;
        tx.commit()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        Ok(true)
    }

    pub async fn change_user_status(
        &self,
        actor: &OperationContext,
        user_id: Uuid,
        status: &str,
        changed_at: DateTime<Utc>,
    ) -> Result<bool, AuthRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        let old_status = sqlx::query_scalar::<_, String>(
            "SELECT u.status FROM auth_users u JOIN auth_user_owner_bindings b ON b.user_id=u.id AND b.owner_id=$1 AND b.is_active WHERE u.id=$2 FOR UPDATE",
        )
        .bind(actor.owner_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        let Some(old_status) = old_status else {
            tx.commit()
                .await
                .map_err(|_| AuthRepositoryError::Database)?;
            return Ok(false);
        };
        sqlx::query(
            "UPDATE auth_users SET status=$3, permissions_changed_at=$4, updated_at=$4 WHERE id=$2 AND EXISTS (SELECT 1 FROM auth_user_owner_bindings WHERE owner_id=$1 AND user_id=auth_users.id AND is_active)",
        )
        .bind(actor.owner_id)
        .bind(user_id)
        .bind(status)
        .bind(changed_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                actor,
                "auth.user.status_changed",
                "H1",
                "auth_user",
                user_id.to_string(),
                Some(AuditDiff::compute(
                    serde_json::json!({"status": old_status}),
                    serde_json::json!({"status": status}),
                )),
            ),
        )
        .await
        .map_err(|_| AuthRepositoryError::Audit)?;
        tx.commit()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        Ok(true)
    }

    pub async fn record_login_session(
        &self,
        user: &CurrentUser,
        jti: &str,
        expires_at: DateTime<Utc>,
        occurred_at: DateTime<Utc>,
        device_name: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), AuthRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        sqlx::query(
            r#"
            INSERT INTO auth_sessions
                (session_id, owner_id, user_id, device_name, ip, logged_in_at, expires_at)
            VALUES ($1, $2, $3, $4, $5::inet, $6, $7)
            ON CONFLICT (session_id) DO NOTHING
            "#,
        )
        .bind(jti)
        .bind(user.owner_id)
        .bind(user.user_id)
        .bind(device_name)
        .bind(ip)
        .bind(occurred_at)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        let request = AuditWriteRequest {
            occurred_at,
            actor_id: user.user_id,
            actor_name: user.display_name.clone(),
            owner_id: user.owner_id,
            jti: jti.to_string(),
            action: "auth.login.success".to_string(),
            module: "H1".to_string(),
            resource_type: "auth_session".to_string(),
            resource_id: user.user_id.to_string(),
            diff: None,
            request_id: None,
            ip: ip.map(str::to_string),
            user_agent: user_agent.map(str::to_string),
        };
        append_event_in_tx(&mut tx, &request)
            .await
            .map_err(|_| AuthRepositoryError::Audit)?;
        tx.commit()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        Ok(())
    }

    pub async fn active_sessions(
        &self,
        owner_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<AuthSessionRow>, AuthRepositoryError> {
        sqlx::query_as::<_, AuthSessionRow>(
            r#"
            SELECT session_id, user_id, device_name, host(ip) AS ip,
                   logged_in_at, expires_at, revoked_at
              FROM auth_sessions
             WHERE owner_id = $1
               AND user_id = $2
               AND revoked_at IS NULL
               AND expires_at > $3
             ORDER BY logged_in_at DESC, session_id
            "#,
        )
        .bind(owner_id)
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }

    pub async fn user_belongs_to_owner(
        &self,
        owner_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AuthRepositoryError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM auth_user_owner_bindings WHERE owner_id=$1 AND user_id=$2 AND is_active)",
        )
        .bind(owner_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }

    pub async fn revoke_session(
        &self,
        actor: &OperationContext,
        user_id: Uuid,
        session_id: &str,
        reason: &str,
        action: &str,
    ) -> Result<SessionRevokeState, AuthRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        let row = sqlx::query_as::<_, AuthSessionRow>(
            "SELECT session_id,user_id,device_name,host(ip) AS ip,logged_in_at,expires_at,revoked_at FROM auth_sessions WHERE owner_id=$1 AND user_id=$2 AND session_id=$3 FOR UPDATE",
        )
        .bind(actor.owner_id)
        .bind(user_id)
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        let Some(row) = row else {
            tx.commit()
                .await
                .map_err(|_| AuthRepositoryError::Database)?;
            return Ok(SessionRevokeState::NotFound);
        };
        if row.revoked_at.is_some() {
            tx.commit()
                .await
                .map_err(|_| AuthRepositoryError::Database)?;
            return Ok(SessionRevokeState::AlreadyRevoked {
                expires_at: row.expires_at,
            });
        }
        sqlx::query(
            "UPDATE auth_sessions SET revoked_at=now(), revoke_reason=$4, revoked_by=$5 WHERE owner_id=$1 AND user_id=$2 AND session_id=$3",
        )
        .bind(actor.owner_id)
        .bind(user_id)
        .bind(session_id)
        .bind(reason)
        .bind(actor.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                actor,
                action,
                "H1",
                "auth_session",
                session_id,
                Some(AuditDiff::compute(
                    serde_json::json!({"status": "active"}),
                    serde_json::json!({"status": "revoked", "reason": reason}),
                )),
            ),
        )
        .await
        .map_err(|_| AuthRepositoryError::Audit)?;
        tx.commit()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        Ok(SessionRevokeState::Revoked {
            expires_at: row.expires_at,
        })
    }

    pub async fn revoke_active_sessions(
        &self,
        actor: &OperationContext,
        user_id: Uuid,
        except_session_id: Option<&str>,
        reason: &str,
        action: &str,
    ) -> Result<Vec<AuthSessionRow>, AuthRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        let rows = sqlx::query_as::<_, AuthSessionRow>(
            "SELECT session_id,user_id,device_name,host(ip) AS ip,logged_in_at,expires_at,revoked_at FROM auth_sessions WHERE owner_id=$1 AND user_id=$2 AND revoked_at IS NULL AND expires_at > now() AND ($3::text IS NULL OR session_id <> $3) FOR UPDATE",
        )
        .bind(actor.owner_id)
        .bind(user_id)
        .bind(except_session_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| AuthRepositoryError::Database)?;
        if !rows.is_empty() {
            sqlx::query(
                "UPDATE auth_sessions SET revoked_at=now(), revoke_reason=$3, revoked_by=$4 WHERE owner_id=$1 AND user_id=$2 AND revoked_at IS NULL AND expires_at > now() AND ($5::text IS NULL OR session_id <> $5)",
            )
            .bind(actor.owner_id)
            .bind(user_id)
            .bind(reason)
            .bind(actor.user_id)
            .bind(except_session_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        }
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                actor,
                action,
                "H1",
                "auth_user",
                user_id.to_string(),
                Some(AuditDiff::compute(
                    serde_json::json!({"active_sessions": rows.len()}),
                    serde_json::json!({"revoked_sessions": rows.len(), "reason": reason}),
                )),
            ),
        )
        .await
        .map_err(|_| AuthRepositoryError::Audit)?;
        tx.commit()
            .await
            .map_err(|_| AuthRepositoryError::Database)?;
        Ok(rows)
    }

    pub async fn append_auth_event(
        &self,
        actor: &OperationContext,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        diff: Option<AuditDiff>,
    ) -> Result<(), AuthRepositoryError> {
        append_event(
            &self.pool,
            &AuditWriteRequest::from_auth_context(
                actor,
                action,
                "H1",
                resource_type,
                resource_id,
                diff,
            ),
        )
        .await
        .map_err(|_| AuthRepositoryError::Audit)?;
        Ok(())
    }

    pub async fn logout_audit_exists(
        &self,
        owner_id: Uuid,
        jti: &str,
    ) -> Result<bool, AuthRepositoryError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM audit_event WHERE owner_id=$1 AND jti=$2 AND action='auth.logout' AND resource_id=$2)",
        )
        .bind(owner_id)
        .bind(jti)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }

    async fn role_codes(
        &self,
        user_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Vec<String>, AuthRepositoryError> {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT r.role_code
              FROM auth_user_roles ur
              JOIN auth_roles r
                ON r.id = ur.role_id
             WHERE ur.user_id = $1
               AND ur.owner_id = $2
             ORDER BY r.role_code
            "#,
        )
        .bind(user_id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }

    async fn permission_codes(
        &self,
        user_id: Uuid,
        owner_id: Uuid,
    ) -> Result<Vec<String>, AuthRepositoryError> {
        sqlx::query_scalar::<_, String>(
            r#"
            WITH RECURSIVE hierarchy AS (
                SELECT ur.role_id AS root_role_id,
                       r.id,
                       r.parent_role_id,
                       0 AS depth
                  FROM auth_user_roles ur
                  JOIN auth_roles r ON r.id = ur.role_id
                 WHERE ur.user_id = $1
                   AND ur.owner_id = $2
                UNION ALL
                SELECT hierarchy.root_role_id,
                       parent.id,
                       parent.parent_role_id,
                       hierarchy.depth + 1
                  FROM auth_roles parent
                  JOIN hierarchy ON hierarchy.parent_role_id = parent.id
            ), decisions AS (
                SELECT hierarchy.root_role_id,
                       grant_row.permission_id,
                       hierarchy.depth,
                       TRUE AS allowed
                  FROM hierarchy
                  JOIN auth_role_permissions grant_row
                    ON grant_row.role_id = hierarchy.id
                UNION ALL
                SELECT hierarchy.root_role_id,
                       exclusion.permission_id,
                       hierarchy.depth,
                       FALSE AS allowed
                  FROM hierarchy
                  JOIN auth_role_permission_exclusions exclusion
                    ON exclusion.role_id = hierarchy.id
            ), nearest AS (
                SELECT DISTINCT ON (root_role_id, permission_id)
                       root_role_id,
                       permission_id,
                       allowed
                  FROM decisions
                 ORDER BY root_role_id, permission_id, depth, allowed
            )
            SELECT DISTINCT permission.permission_code
              FROM nearest
              JOIN auth_permissions permission
                ON permission.id = nearest.permission_id
             WHERE nearest.allowed
             ORDER BY permission.permission_code
            "#,
        )
        .bind(user_id)
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
    }
}

#[derive(Debug, FromRow)]
pub struct AuthSessionRow {
    pub session_id: String,
    pub user_id: Uuid,
    pub device_name: String,
    pub ip: Option<String>,
    pub logged_in_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl AuthSessionRow {
    pub fn into_active_session(self, current_jti: &str) -> AuthSession {
        AuthSession {
            is_current: self.session_id == current_jti,
            session_id: self.session_id,
            user_id: self.user_id,
            device_name: self.device_name,
            ip: self.ip,
            logged_in_at: self.logged_in_at,
            expires_at: self.expires_at,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRevokeState {
    NotFound,
    AlreadyRevoked { expires_at: DateTime<Utc> },
    Revoked { expires_at: DateTime<Utc> },
}

#[derive(Debug, FromRow)]
pub struct LoginUser {
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub owner_code: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub status: String,
    pub locked_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthRepositoryError {
    Database,
    Audit,
}
