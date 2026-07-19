//! H8 ERP 连接错误类型。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use wms_domain::{ErrorResponse, H8ErpConnectorError};

use crate::auth::AuthError;

#[derive(Debug)]
pub enum H8ErpConnectorRepoError {
    Domain(H8ErpConnectorError),
    DuplicateCode,
    Db(String),
}

#[derive(Debug)]
pub(crate) enum H8ErpConnectorHandlerError {
    Auth(AuthError),
    MissingIdempotencyKey,
    Repo(H8ErpConnectorRepoError),
}

impl From<AuthError> for H8ErpConnectorHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<H8ErpConnectorRepoError> for H8ErpConnectorHandlerError {
    fn from(value: H8ErpConnectorRepoError) -> Self {
        Self::Repo(value)
    }
}

impl IntoResponse for H8ErpConnectorHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(err) => return err.into_response(),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H8-400",
                "Idempotency-Key required".to_string(),
            ),
            Self::Repo(H8ErpConnectorRepoError::DuplicateCode) => (
                StatusCode::CONFLICT,
                "H8-409",
                "connector_code already exists".to_string(),
            ),
            Self::Repo(H8ErpConnectorRepoError::Domain(err)) => domain_error(err),
            Self::Repo(H8ErpConnectorRepoError::Db(msg)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "H8-500", msg)
            }
        };
        (
            status,
            Json(ErrorResponse {
                code: code.into(),
                message,
                severity: "error".into(),
                details: serde_json::json!({}),
                trace_id: "unavailable".into(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

fn domain_error(err: H8ErpConnectorError) -> (StatusCode, &'static str, String) {
    match err {
        H8ErpConnectorError::NotFound => (StatusCode::NOT_FOUND, "H8-404", "not found".into()),
        H8ErpConnectorError::VersionConflict => (
            StatusCode::CONFLICT,
            "H8-409",
            "config_version conflict".into(),
        ),
        H8ErpConnectorError::RouteOverlap => (
            StatusCode::CONFLICT,
            "H8-409",
            "route overlap with active connector".into(),
        ),
        H8ErpConnectorError::TestRequired => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "current version must pass test".into(),
        ),
        H8ErpConnectorError::DeleteNotAllowed => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "delete not allowed".into(),
        ),
        H8ErpConnectorError::IllegalTransition => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "illegal status transition".into(),
        ),
        H8ErpConnectorError::InsufficientApiKeyScope => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H8-422",
            "api key scopes do not cover message types".into(),
        ),
        H8ErpConnectorError::IdempotencyConflict => (
            StatusCode::CONFLICT,
            "H8-409",
            "idempotency key reuse with different payload".into(),
        ),
        other => (StatusCode::BAD_REQUEST, "H8-400", format!("{other:?}")),
    }
}
