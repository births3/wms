use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use wms_domain::ErrorResponse;

use crate::auth::AuthError;

#[derive(Debug)]
pub enum RoleError {
    Auth(AuthError),
    Database,
    Audit,
    Serialize,
    Validation,
    MissingIdempotency,
    IdempotencyConflict,
    DuplicateRole,
    RoleInUse,
    UnknownPermission,
    CrossOwner,
    Revocation,
    UserDuplicate,
    RoleNotFound,
    PasswordHash,
    UserValidation,
}

impl From<AuthError> for RoleError {
    fn from(error: AuthError) -> Self {
        Self::Auth(error)
    }
}

impl From<sqlx::Error> for RoleError {
    fn from(_: sqlx::Error) -> Self {
        Self::Database
    }
}

impl IntoResponse for RoleError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::MissingIdempotency => (
                StatusCode::BAD_REQUEST,
                "H1-IDEMPOTENCY-REQUIRED",
                "缺少 Idempotency-Key",
            ),
            Self::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "H1-IDEMPOTENCY-CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::DuplicateRole => (StatusCode::CONFLICT, "H1-ROLE-DUPLICATE", "角色编码已存在"),
            Self::RoleInUse => (
                StatusCode::CONFLICT,
                "H1-ROLE-IN-USE",
                "角色已绑定用户或子角色",
            ),
            Self::UnknownPermission => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1-PERMISSION-UNKNOWN",
                "包含未知权限码",
            ),
            Self::CrossOwner => (StatusCode::FORBIDDEN, "AUTH-004", "跨货主访问被拒绝"),
            Self::UserDuplicate => (StatusCode::CONFLICT, "M1_USER_DUPLICATE", "用户名已存在"),
            Self::RoleNotFound => (
                StatusCode::NOT_FOUND,
                "H1_ROLE_NOT_FOUND",
                "角色不存在或不属于当前货主",
            ),
            Self::PasswordHash => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1_PASSWORD_HASH_ERROR",
                "用户密码处理失败",
            ),
            Self::UserValidation => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M1_USER_INVALID",
                "用户资料、密码或角色参数非法",
            ),
            Self::Validation => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1-ROLE-INVALID",
                "角色参数非法",
            ),
            Self::Revocation => (
                StatusCode::SERVICE_UNAVAILABLE,
                "H1-REVOCATION-UNAVAILABLE",
                "权限撤销存储不可用",
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1-ROLE-DATABASE",
                "角色权限操作失败",
            ),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.into(),
                message: message.into(),
                severity: "error".into(),
                details: serde_json::json!({}),
                trace_id: "unavailable".into(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}
