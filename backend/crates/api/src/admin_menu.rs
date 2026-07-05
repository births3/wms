//! US-H1-007 admin menu draft, publish, version, and button permission service.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AdminMenuNode, AdminMenuVersion, BatchEnableAdminMenuRequest, CreateAdminMenuNodeRequest,
    PublishAdminMenuRequest, RollbackAdminMenuRequest, UpdateAdminMenuNodeRequest,
};

use crate::{
    admin_menu_idempotency::{
        finish_mutation, json_request_hash, lock_idempotency_key, replay_idempotency,
    },
    admin_menu_model::{
        build_tree, filter_visible_tree, map_db_error, validate_buttons, validate_node,
        MenuNodeRow, VersionRow,
    },
    admin_menu_repository::{
        buttons_for_nodes, draft_buttons, latest_version, load_draft_node_tree, load_version_by_no,
        next_level_and_path, next_version_no, previous_version, refresh_child_levels_and_paths,
        replace_buttons, restore_draft_from_version, snapshot_draft, validate_draft_for_publish,
        version_buttons, would_exceed_three_levels,
    },
    auth::AuthContext,
};

pub use crate::admin_menu_model::AdminMenuError;

pub const ADMIN_MENU_READ_PERMISSION: &str = "h1.menu.read";
pub const ADMIN_MENU_WRITE_PERMISSION: &str = "h1.menu.write";
pub const ADMIN_MENU_PUBLISH_PERMISSION: &str = "h1.menu.publish";

#[derive(Clone, Debug)]
pub struct PgAdminMenuService;

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

impl PgAdminMenuService {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_published_tree(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
    ) -> Result<(Vec<AdminMenuNode>, Option<i64>), AdminMenuError> {
        let Some(version) = latest_version(pool).await? else {
            return Ok((Vec::new(), None));
        };
        let nodes = sqlx::query_as::<_, MenuNodeRow>(
            r#"
            SELECT source_node_id AS id, parent_source_id AS parent_id, level, code, path,
                   title, view_id, icon_key, permission_key, sort_order, enabled,
                   created_at, updated_at
              FROM admin_menu_version_nodes
             WHERE version_id = $1 AND enabled = TRUE
             ORDER BY level, sort_order, title
            "#,
        )
        .bind(version.id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        let buttons = version_buttons(pool, version.id).await?;
        let tree = build_tree(nodes, buttons);
        let visible = if ctx.has_permission(ADMIN_MENU_READ_PERMISSION) {
            tree
        } else {
            filter_visible_tree(tree, ctx)
        };
        Ok((visible, Some(version.version_no)))
    }

    pub async fn list_draft_tree(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<AdminMenuNode>, AdminMenuError> {
        let nodes = sqlx::query_as::<_, MenuNodeRow>(
            r#"
            SELECT id, parent_id, level, code, path, title, view_id, icon_key,
                   permission_key, sort_order, enabled, created_at, updated_at
              FROM admin_menu_draft_nodes
             ORDER BY level, sort_order, title
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        let buttons = draft_buttons(pool).await?;
        Ok(build_tree(nodes, buttons))
    }

    pub async fn create_node(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: CreateAdminMenuNodeRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AdminMenuNode>, AdminMenuError> {
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let (level, path) = next_level_and_path(&mut tx, req.parent_id, &req.code).await?;
        validate_node(
            level,
            req.view_id.as_deref(),
            &req.icon_key,
            &req.permission_key,
        )?;
        validate_buttons(&req.button_permissions)?;
        let row = sqlx::query_as::<_, MenuNodeRow>(
            r#"
            INSERT INTO admin_menu_draft_nodes (
                id, parent_id, level, code, path, title, view_id, icon_key,
                permission_key, sort_order, enabled, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
            RETURNING id, parent_id, level, code, path, title, view_id, icon_key,
                      permission_key, sort_order, enabled, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(req.parent_id)
        .bind(level)
        .bind(&req.code)
        .bind(path)
        .bind(&req.title)
        .bind(req.view_id)
        .bind(&req.icon_key)
        .bind(&req.permission_key)
        .bind(req.sort_order)
        .bind(req.enabled)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        replace_buttons(&mut tx, row.id, &req.button_permissions, now).await?;
        let node = load_draft_node_tree(&mut tx, row.id).await?;
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/admin/menus/draft/nodes",
            &node,
            "admin_menu.create_node",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: node,
            replayed: false,
        })
    }

    pub async fn update_node(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        node_id: Uuid,
        req: UpdateAdminMenuNodeRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AdminMenuNode>, AdminMenuError> {
        let request_hash =
            json_request_hash(&serde_json::json!({ "node_id": node_id, "request": &req }))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let current =
            crate::admin_menu_repository::load_draft_node_for_update(&mut tx, node_id).await?;
        let parent_id = req.parent_id.or(current.parent_id);
        let (level, path) = next_level_and_path(&mut tx, parent_id, &current.code).await?;
        if would_exceed_three_levels(&mut tx, node_id, level).await? {
            return Err(AdminMenuError::InvalidTree);
        }
        let view_id = req.view_id.or(current.view_id);
        let icon_key = req.icon_key.unwrap_or(current.icon_key);
        let permission_key = req.permission_key.unwrap_or(current.permission_key);
        validate_node(level, view_id.as_deref(), &icon_key, &permission_key)?;
        if let Some(buttons) = &req.button_permissions {
            validate_buttons(buttons)?;
        }
        sqlx::query(
            r#"
            UPDATE admin_menu_draft_nodes
               SET parent_id = $1,
                   level = $2,
                   path = $3,
                   title = COALESCE($4, title),
                   view_id = $5,
                   icon_key = $6,
                   permission_key = $7,
                   sort_order = COALESCE($8, sort_order),
                   enabled = COALESCE($9, enabled),
                   updated_at = $10,
                   version = version + 1
             WHERE id = $11
            "#,
        )
        .bind(parent_id)
        .bind(level)
        .bind(path)
        .bind(req.title)
        .bind(view_id)
        .bind(icon_key)
        .bind(permission_key)
        .bind(req.sort_order)
        .bind(req.enabled)
        .bind(now)
        .bind(node_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        refresh_child_levels_and_paths(&mut tx, node_id, now).await?;
        if let Some(buttons) = &req.button_permissions {
            replace_buttons(&mut tx, node_id, buttons, now).await?;
        }
        let node = load_draft_node_tree(&mut tx, node_id).await?;
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/admin/menus/draft/nodes/{node_id}"),
            &node,
            "admin_menu.update_node",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: node,
            replayed: false,
        })
    }

    pub async fn batch_enable(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: BatchEnableAdminMenuRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<Vec<AdminMenuNode>>, AdminMenuError> {
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let rows = sqlx::query_as::<_, MenuNodeRow>(
            r#"
            UPDATE admin_menu_draft_nodes
               SET enabled = $1, updated_at = $2, version = version + 1
             WHERE id = ANY($3)
             RETURNING id, parent_id, level, code, path, title, view_id, icon_key,
                       permission_key, sort_order, enabled, created_at, updated_at
            "#,
        )
        .bind(req.enabled)
        .bind(now)
        .bind(&req.ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if rows.len() != req.ids.len() {
            return Err(AdminMenuError::NodeNotFound);
        }
        let buttons = buttons_for_nodes(&mut tx, &req.ids).await?;
        let value = rows
            .into_iter()
            .map(|row| {
                let id = row.id;
                row.into_node(buttons.get(&id).cloned().unwrap_or_default(), Vec::new())
            })
            .collect::<Vec<_>>();
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/admin/menus/draft/batch-enable",
            &value,
            "admin_menu.batch_enable",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn publish(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PublishAdminMenuRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AdminMenuVersion>, AdminMenuError> {
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        validate_draft_for_publish(&mut tx).await?;
        let version_no = next_version_no(&mut tx).await?;
        let version_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, VersionRow>(
            r#"
            INSERT INTO admin_menu_versions (id, version_no, note, published_by, published_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, version_no, note, published_by, published_at
            "#,
        )
        .bind(version_id)
        .bind(version_no)
        .bind(req.note)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        snapshot_draft(&mut tx, version_id).await?;
        let version = AdminMenuVersion::from(row);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/admin/menus/publish",
            &version,
            "admin_menu.publish",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: version,
            replayed: false,
        })
    }

    pub async fn rollback(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: RollbackAdminMenuRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AdminMenuVersion>, AdminMenuError> {
        let request_hash = json_request_hash(&req)?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let target = if let Some(version_no) = req.target_version_no {
            load_version_by_no(&mut tx, version_no).await?
        } else {
            previous_version(&mut tx).await?
        };
        restore_draft_from_version(&mut tx, target.id).await?;
        validate_draft_for_publish(&mut tx).await?;
        let version_no = next_version_no(&mut tx).await?;
        let version_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, VersionRow>(
            r#"
            INSERT INTO admin_menu_versions (id, version_no, note, published_by, published_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, version_no, note, published_by, published_at
            "#,
        )
        .bind(version_id)
        .bind(version_no)
        .bind(format!("回滚到版本 {}", target.version_no))
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        snapshot_draft(&mut tx, version_id).await?;
        let version = AdminMenuVersion::from(row);
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/admin/menus/rollback",
            &version,
            "admin_menu.rollback",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: version,
            replayed: false,
        })
    }
}
