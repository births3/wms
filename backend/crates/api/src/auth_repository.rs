//! Wave 1 H1 auth persistence.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::CurrentUser;

use crate::audit::{append_event, AuditWriteRequest};

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
        owner_code: &str,
        username: &str,
    ) -> Result<Option<LoginUser>, AuthRepositoryError> {
        sqlx::query_as::<_, LoginUser>(
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
        .bind(owner_code.trim())
        .bind(username.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthRepositoryError::Database)
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

    pub async fn append_login_success_audit(
        &self,
        user: &CurrentUser,
        jti: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), AuthRepositoryError> {
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
            ip: None,
            user_agent: None,
        };
        append_event(&self.pool, &request)
            .await
            .map_err(|_| AuthRepositoryError::Audit)?;
        Ok(())
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
            SELECT DISTINCT p.permission_code
              FROM auth_user_roles ur
              JOIN auth_role_permissions rp
                ON rp.role_id = ur.role_id
              JOIN auth_permissions p
                ON p.id = rp.permission_id
             WHERE ur.user_id = $1
               AND ur.owner_id = $2
             ORDER BY p.permission_code
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
