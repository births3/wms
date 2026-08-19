//! Admin menu persistence helpers.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    AdminMenuButtonPermission, AdminMenuNode, UpsertAdminMenuButtonPermissionRequest,
};

use crate::admin_menu_model::{
    code_segment, map_db_error, validate_node, AdminMenuError, ButtonRow, MenuNodeRow, VersionRow,
};

pub(crate) async fn latest_version(pool: &PgPool) -> Result<Option<VersionRow>, AdminMenuError> {
    sqlx::query_as::<_, VersionRow>(
        r#"
        SELECT id, version_no, note, published_by, published_at
          FROM admin_menu_versions
         ORDER BY version_no DESC
         LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn draft_buttons(pool: &PgPool) -> Result<Vec<ButtonRow>, AdminMenuError> {
    sqlx::query_as::<_, ButtonRow>(
        r#"
        SELECT menu_node_id, action_key, action_label, action_kind, enabled, sort_order
          FROM admin_menu_draft_button_permissions
         ORDER BY menu_node_id, sort_order, action_key
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn version_buttons(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<Vec<ButtonRow>, AdminMenuError> {
    sqlx::query_as::<_, ButtonRow>(
        r#"
        SELECT menu_source_node_id AS menu_node_id, action_key, action_label, action_kind,
               enabled, sort_order
          FROM admin_menu_version_button_permissions
         WHERE version_id = $1
         ORDER BY menu_source_node_id, sort_order, action_key
        "#,
    )
    .bind(version_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn next_level_and_path(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: Option<Uuid>,
    code: &str,
) -> Result<(i32, String), AdminMenuError> {
    let segment = code_segment(code)?;
    if let Some(parent_id) = parent_id {
        let parent = load_draft_node_for_update(tx, parent_id).await?;
        if parent.level >= 3 {
            return Err(AdminMenuError::InvalidTree);
        }
        Ok((parent.level + 1, format!("{}/{}", parent.path, segment)))
    } else {
        Ok((1, segment))
    }
}

pub(crate) async fn load_draft_node_for_update(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
) -> Result<MenuNodeRow, AdminMenuError> {
    sqlx::query_as::<_, MenuNodeRow>(
        r#"
        SELECT id, parent_id, level, code, path, title, view_id, icon_key,
               permission_key, sort_order, enabled, created_at, updated_at
          FROM admin_menu_draft_nodes
         WHERE id = $1
         FOR UPDATE
        "#,
    )
    .bind(node_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AdminMenuError::NodeNotFound)
}

pub(crate) async fn load_draft_node_tree(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
) -> Result<AdminMenuNode, AdminMenuError> {
    let row = sqlx::query_as::<_, MenuNodeRow>(
        r#"
        SELECT id, parent_id, level, code, path, title, view_id, icon_key,
               permission_key, sort_order, enabled, created_at, updated_at
          FROM admin_menu_draft_nodes
         WHERE id = $1
        "#,
    )
    .bind(node_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AdminMenuError::NodeNotFound)?;
    let buttons = buttons_for_nodes(tx, &[node_id]).await?;
    Ok(row.into_node(
        buttons.get(&node_id).cloned().unwrap_or_default(),
        Vec::new(),
    ))
}

pub(crate) async fn buttons_for_nodes(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<AdminMenuButtonPermission>>, AdminMenuError> {
    let rows = sqlx::query_as::<_, ButtonRow>(
        r#"
        SELECT menu_node_id, action_key, action_label, action_kind, enabled, sort_order
          FROM admin_menu_draft_button_permissions
         WHERE menu_node_id = ANY($1)
         ORDER BY menu_node_id, sort_order, action_key
        "#,
    )
    .bind(ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut buttons = HashMap::new();
    for row in rows {
        buttons
            .entry(row.menu_node_id)
            .or_insert_with(Vec::new)
            .push(row.into());
    }
    Ok(buttons)
}

pub(crate) async fn replace_buttons(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
    buttons: &[UpsertAdminMenuButtonPermissionRequest],
    now: DateTime<Utc>,
) -> Result<(), AdminMenuError> {
    sqlx::query("DELETE FROM admin_menu_draft_button_permissions WHERE menu_node_id = $1")
        .bind(node_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    for button in buttons {
        sqlx::query(
            r#"
            INSERT INTO admin_menu_draft_button_permissions (
                id, menu_node_id, action_key, action_label, action_kind,
                enabled, sort_order, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(node_id)
        .bind(&button.action_key)
        .bind(&button.action_label)
        .bind(&button.action_kind)
        .bind(button.enabled)
        .bind(button.sort_order)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

pub(crate) async fn would_exceed_three_levels(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
    level: i32,
) -> Result<bool, AdminMenuError> {
    let max_child_depth: Option<i32> = sqlx::query_scalar(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id, parent_id, 0 AS depth
              FROM admin_menu_draft_nodes
             WHERE id = $1
            UNION ALL
            SELECT child.id, child.parent_id, descendants.depth + 1
              FROM admin_menu_draft_nodes child
              JOIN descendants ON child.parent_id = descendants.id
        )
        SELECT max(depth) FROM descendants
        "#,
    )
    .bind(node_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(level + max_child_depth.unwrap_or(0) > 3)
}

pub(crate) async fn refresh_child_levels_and_paths(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), AdminMenuError> {
    let mut stack = vec![node_id];
    while let Some(parent_id) = stack.pop() {
        let parent = load_draft_node_for_update(tx, parent_id).await?;
        let children = sqlx::query_as::<_, MenuNodeRow>(
            r#"
            SELECT id, parent_id, level, code, path, title, view_id, icon_key,
                   permission_key, sort_order, enabled, created_at, updated_at
              FROM admin_menu_draft_nodes
             WHERE parent_id = $1
             FOR UPDATE
            "#,
        )
        .bind(parent_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
        for child in children {
            let child_level = parent.level + 1;
            let child_path = format!("{}/{}", parent.path, code_segment(&child.code)?);
            validate_node(
                child_level,
                child.view_id.as_deref(),
                &child.icon_key,
                &child.permission_key,
            )?;
            sqlx::query(
                r#"
                UPDATE admin_menu_draft_nodes
                   SET level = $1, path = $2, updated_at = $3, version = version + 1
                 WHERE id = $4
                "#,
            )
            .bind(child_level)
            .bind(child_path)
            .bind(now)
            .bind(child.id)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
            stack.push(child.id);
        }
    }
    Ok(())
}

pub(crate) async fn validate_draft_for_publish(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), AdminMenuError> {
    let rows = sqlx::query_as::<_, MenuNodeRow>(
        r#"
        SELECT id, parent_id, level, code, path, title, view_id, icon_key,
               permission_key, sort_order, enabled, created_at, updated_at
          FROM admin_menu_draft_nodes
        "#,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for row in &rows {
        validate_node(
            row.level,
            row.view_id.as_deref(),
            &row.icon_key,
            &row.permission_key,
        )?;
    }
    let ids = rows.iter().map(|row| row.id).collect::<HashSet<_>>();
    for row in &rows {
        if row.level == 1 && row.parent_id.is_some() {
            return Err(AdminMenuError::InvalidTree);
        }
        if row.level > 1 && !row.parent_id.is_some_and(|id| ids.contains(&id)) {
            return Err(AdminMenuError::InvalidTree);
        }
    }
    Ok(())
}

pub(crate) async fn next_version_no(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<i64, AdminMenuError> {
    let current: Option<i64> =
        sqlx::query_scalar("SELECT max(version_no) FROM admin_menu_versions")
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    Ok(current.unwrap_or(0) + 1)
}

pub(crate) async fn snapshot_draft(
    tx: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
) -> Result<(), AdminMenuError> {
    sqlx::query(
        r#"
        INSERT INTO admin_menu_version_nodes (
            id, version_id, source_node_id, parent_source_id, level, code, path,
            title, view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
        )
        SELECT md5($1::text || ':' || id::text)::uuid, $1, id, parent_id, level, code, path, title,
               view_id, icon_key, permission_key, sort_order, enabled, created_at, updated_at
          FROM admin_menu_draft_nodes
        "#,
    )
    .bind(version_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        r#"
        INSERT INTO admin_menu_version_button_permissions (
            id, version_id, menu_source_node_id, action_key, action_label,
            action_kind, enabled, sort_order
        )
        SELECT md5($1::text || ':' || id::text)::uuid, $1, menu_node_id, action_key, action_label,
               action_kind, enabled, sort_order
          FROM admin_menu_draft_button_permissions
        "#,
    )
    .bind(version_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn load_version_by_no(
    tx: &mut Transaction<'_, Postgres>,
    version_no: i64,
) -> Result<VersionRow, AdminMenuError> {
    sqlx::query_as::<_, VersionRow>(
        r#"
        SELECT id, version_no, note, published_by, published_at
          FROM admin_menu_versions
         WHERE version_no = $1
        "#,
    )
    .bind(version_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AdminMenuError::VersionNotFound)
}

pub(crate) async fn previous_version(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<VersionRow, AdminMenuError> {
    sqlx::query_as::<_, VersionRow>(
        r#"
        SELECT id, version_no, note, published_by, published_at
          FROM admin_menu_versions
         ORDER BY version_no DESC
         OFFSET 1
         LIMIT 1
        "#,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AdminMenuError::VersionNotFound)
}

pub(crate) async fn restore_draft_from_version(
    tx: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
) -> Result<(), AdminMenuError> {
    sqlx::query("DELETE FROM admin_menu_draft_button_permissions")
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("DELETE FROM admin_menu_draft_nodes")
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query(
        r#"
        INSERT INTO admin_menu_draft_nodes (
            id, parent_id, level, code, path, title, view_id, icon_key,
            permission_key, sort_order, enabled, created_at, updated_at
        )
        SELECT source_node_id, parent_source_id, level, code, path, title, view_id,
               icon_key, permission_key, sort_order, enabled, created_at, updated_at
          FROM admin_menu_version_nodes
         WHERE version_id = $1
         ORDER BY level, sort_order
        "#,
    )
    .bind(version_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        r#"
        INSERT INTO admin_menu_draft_button_permissions (
            id, menu_node_id, action_key, action_label, action_kind,
            enabled, sort_order, created_at, updated_at
        )
        SELECT md5(menu_source_node_id::text || ':' || action_key)::uuid, menu_source_node_id, action_key, action_label,
               action_kind, enabled, sort_order, now(), now()
          FROM admin_menu_version_button_permissions
         WHERE version_id = $1
        "#,
    )
    .bind(version_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
