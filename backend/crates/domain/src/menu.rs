use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

/// H1 菜单按钮权限点。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuButtonPermission {
    pub action_key: String,
    pub action_label: String,
    pub action_kind: String,
    pub enabled: bool,
    pub sort_order: i32,
}

/// H1 管理端菜单节点。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuNode {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub level: i32,
    pub code: String,
    pub path: String,
    pub title: String,
    pub view_id: Option<String>,
    pub icon_key: String,
    pub permission_key: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub button_permissions: Vec<AdminMenuButtonPermission>,
    pub children: Vec<AdminMenuNode>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// H1 菜单树响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuTreeResponse {
    pub data: Vec<AdminMenuNode>,
    pub version_no: Option<i64>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertAdminMenuButtonPermissionRequest {
    pub action_key: String,
    pub action_label: String,
    pub action_kind: String,
    pub enabled: bool,
    pub sort_order: i32,
}

/// 新增 H1 菜单节点请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateAdminMenuNodeRequest {
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub title: String,
    pub view_id: Option<String>,
    pub icon_key: String,
    pub permission_key: String,
    pub sort_order: i32,
    pub enabled: bool,
    pub button_permissions: Vec<UpsertAdminMenuButtonPermissionRequest>,
}

/// 更新 H1 菜单节点请求。
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct UpdateAdminMenuNodeRequest {
    pub parent_id: Option<Uuid>,
    pub title: Option<String>,
    pub view_id: Option<String>,
    pub icon_key: Option<String>,
    pub permission_key: Option<String>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
    pub button_permissions: Option<Vec<UpsertAdminMenuButtonPermissionRequest>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchEnableAdminMenuRequest {
    pub ids: Vec<Uuid>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PublishAdminMenuRequest {
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RollbackAdminMenuRequest {
    pub target_version_no: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AdminMenuVersion {
    pub id: Uuid,
    pub version_no: i64,
    pub note: Option<String>,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}
