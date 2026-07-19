//! H8 消息 API 错误。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use wms_domain::{ErrorResponse, H8MessageError};

use crate::auth::AuthError;

#[derive(Debug)]
pub enum H8ErpMessageRepoError {
    Domain(H8MessageError),
    NotFound,
    Db(String),
}

#[derive(Debug)]
pub(crate) enum H8ErpMessageHandlerError {
    Auth(AuthError),
    Repo(H8ErpMessageRepoError),
    BadRequest(&'static str),
}

impl From<AuthError> for H8ErpMessageHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<H8ErpMessageRepoError> for H8ErpMessageHandlerError {
    fn from(value: H8ErpMessageRepoError) -> Self {
        Self::Repo(value)
    }
}

impl IntoResponse for H8ErpMessageHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(err) => return err.into_response(),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "H8-400", msg.to_string()),
            Self::Repo(H8ErpMessageRepoError::NotFound) => {
                (StatusCode::NOT_FOUND, "H8-404", "message not found".into())
            }
            Self::Repo(H8ErpMessageRepoError::Domain(err)) => domain_error(err),
            Self::Repo(H8ErpMessageRepoError::Db(msg)) => {
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

fn domain_error(err: H8MessageError) -> (StatusCode, &'static str, String) {
    match err {
        H8MessageError::ReplayNotAllowed => (
            StatusCode::CONFLICT,
            "H8-409",
            "only failed/dead messages may be replayed".into(),
        ),
        H8MessageError::LeaseConflict => (
            StatusCode::CONFLICT,
            "H8-409",
            "message lease conflict".into(),
        ),
        H8MessageError::IllegalTransition => (
            StatusCode::CONFLICT,
            "H8-409",
            "illegal message status transition".into(),
        ),
        H8MessageError::FieldRequired(f) => {
            (StatusCode::BAD_REQUEST, "H8-400", format!("{f} required"))
        }
        other => (
            StatusCode::BAD_REQUEST,
            "H8-400",
            format!("h8 message error: {other:?}"),
        ),
    }
}
