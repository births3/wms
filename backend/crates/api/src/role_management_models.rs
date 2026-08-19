use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::PageMeta;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateRoleRequest {
    pub role_code: String,
    pub role_name: String,
    pub data_scope: String,
    pub parent_role_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub role_name: String,
    pub data_scope: String,
    pub parent_role_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplaceRolePermissionsRequest {
    pub permission_codes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchAssignRolesRequest {
    pub user_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub phone: String,
    pub password: String,
    pub role_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: Uuid,
    pub role_code: String,
    pub role_name: String,
    pub data_scope: String,
    pub parent_role_id: Option<Uuid>,
    pub permission_codes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RoleListResponse {
    pub items: Vec<RoleResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchAssignRolesResponse {
    pub user_ids: Vec<Uuid>,
    pub role_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PermissionResponse {
    pub id: Uuid,
    pub permission_code: String,
    pub permission_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PermissionListResponse {
    pub items: Vec<PermissionResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RoleUserResponse {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub role_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RoleUserListResponse {
    pub data: Vec<RoleUserResponse>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DeleteRoleResponse {
    pub id: Uuid,
}
