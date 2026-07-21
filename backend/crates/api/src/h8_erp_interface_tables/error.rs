use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use wms_domain::ErrorResponse;

use crate::{audit::AuditError, auth::AuthError};

#[derive(Debug)]
pub(crate) enum H8InterfaceTableRepoError {
    ProbeCredentialNotConfigured,
    SecretNotResolved,
    ConnectorNotSupported,
    NotFound,
    Forbidden,
    Db(String),
}

#[derive(Debug)]
pub(crate) enum H8InterfaceTableHandlerError {
    Auth(AuthError),
    BadRequest(String),
    Audit,
    Repo(H8InterfaceTableRepoError),
}

impl From<AuthError> for H8InterfaceTableHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<H8InterfaceTableRepoError> for H8InterfaceTableHandlerError {
    fn from(value: H8InterfaceTableRepoError) -> Self {
        Self::Repo(value)
    }
}

impl From<AuditError> for H8InterfaceTableHandlerError {
    fn from(value: AuditError) -> Self {
        tracing::error!(target: "h8.interface_table", error = ?value, "query audit persistence failed");
        Self::Audit
    }
}

impl IntoResponse for H8InterfaceTableHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(err) => return err.into_response(),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "H8-400", message),
            Self::Audit => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H8-500",
                "query audit persistence failed".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::ProbeCredentialNotConfigured) => (
                StatusCode::CONFLICT,
                "H8_PROBE_CREDENTIAL_NOT_CONFIGURED",
                "interface probe credentials are not configured".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::SecretNotResolved) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H8-422",
                "interface probe secret cannot be resolved".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::ConnectorNotSupported) => (
                StatusCode::BAD_REQUEST,
                "H8-400",
                "connector does not expose an interface table channel".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::NotFound) => (
                StatusCode::NOT_FOUND,
                "H8-404",
                "interface row not found".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::Forbidden) => (
                StatusCode::FORBIDDEN,
                "H8-403",
                "interface row is outside the actor scope".into(),
            ),
            Self::Repo(H8InterfaceTableRepoError::Db(message)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "H8-500", message)
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
