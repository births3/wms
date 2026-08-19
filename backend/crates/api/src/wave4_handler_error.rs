use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use wms_domain::ErrorResponse;

use crate::{auth::AuthError, wave4_repository::Wave4RepositoryError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave4HandlerError {
    Auth(AuthError),
    InvalidIdempotencyKey,
    Repository(Wave4RepositoryError),
    ReplenishmentGap,
}

impl From<AuthError> for Wave4HandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<Wave4RepositoryError> for Wave4HandlerError {
    fn from(value: Wave4RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for Wave4HandlerError {
    fn into_response(self) -> Response {
        if let Wave4HandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            Wave4HandlerError::InvalidIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "W4-400",
                "缺少或非法 Idempotency-Key",
            ),
            Wave4HandlerError::ReplenishmentGap
            | Wave4HandlerError::Repository(Wave4RepositoryError::ReplenishmentGap) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M3_REPLENISH_WAVE_GAP",
                "波次补货缺口生成失败",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::NotFound) => {
                (StatusCode::NOT_FOUND, "W4-404", "资源不存在")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::IdempotencyConflict) => {
                (StatusCode::CONFLICT, "W4-409", "幂等键已用于不同请求")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::DuplicateCode) => {
                (StatusCode::CONFLICT, "W4-409", "业务单号重复")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::OrderAlreadyInWave) => {
                (StatusCode::CONFLICT, "W4-409", "订单已加入其他波次")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::PendingErpCancel) => (
                StatusCode::CONFLICT,
                "W4-409",
                "订单存在待处理的 ERP 取消命令",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::MissingSecondReviewer) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_DUAL_PERSON_REQUIRED",
                "M-VR 策略要求第二复核员",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::UnqualifiedSecondReviewer) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_SECOND_REVIEWER_UNAUTHORIZED",
                "第二复核员不是当前货主的有效保管员",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::DualPersonApprovalRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M4_DUAL_PERSON_APPROVAL_REQUIRED",
                "M-VR 策略要求先完成主管审批",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::MissingRejectReason) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "W4-422", "驳回原因必填")
            }
            Wave4HandlerError::Repository(Wave4RepositoryError::MissingRequiredField(_)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W4-422",
                "必填字段不能为空",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::ErpGoodsMappingIncomplete) => (
                StatusCode::CONFLICT,
                "M4_ERP_GOODS_MAPPING_INCOMPLETE",
                "部分商品未完成 ERP 商品映射，请先同步主数据后再发货",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::RouteBindingUnavailable) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_ROUTE_BINDING_UNAVAILABLE",
                "送货地址没有唯一有效的线路绑定",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::EmptySelection)
            | Wave4HandlerError::Repository(Wave4RepositoryError::BatchNotAffected(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidQuantity)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDocumentType)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDeliveryAddress)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidTraceabilityEvent)
            | Wave4HandlerError::Repository(Wave4RepositoryError::ShortPickNotReplenished)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidDriver)
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidSignatureAttachment)
            | Wave4HandlerError::Repository(Wave4RepositoryError::ReviewValidation(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::ShipmentValidation(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidStatus { .. })
            | Wave4HandlerError::Repository(Wave4RepositoryError::InvalidStateTransition {
                ..
            }) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "W4-422",
                "业务规则校验失败",
            ),
            Wave4HandlerError::Repository(Wave4RepositoryError::Audit(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::DocumentNumbering(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::Database(_))
            | Wave4HandlerError::Repository(Wave4RepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "W4-500",
                "持久化或审计写入失败",
            ),
            Wave4HandlerError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}
