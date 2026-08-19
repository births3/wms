#[allow(unused_imports)]
use crate::{
    reconciliation::{ReconciliationItem, ReconciliationRun},
    reconciliation_handlers::{
        ResolveReconciliationRequest, SetIsolationRequest, SubmitReconciliationRunRequest,
    },
    reconciliation_query::{
        ClaimReconciliationRequest, FailReconciliationClaimRequest, ReconciliationClaimMutation,
        ReconciliationClaimResponse, ReconciliationItemListResponse, ReconciliationItemQuery,
        ReconciliationRule, RenewReconciliationClaimRequest, UpsertReconciliationRuleRequest,
    },
};
#[allow(unused_imports)]
use uuid::Uuid;
#[allow(unused_imports)]
use wms_domain::ErrorResponse;

#[utoipa::path(
    get,
    path = "/api/v1/reconciliation/rule",
    tag = "reconciliation",
    responses(
        (status = 200, body = ReconciliationRule),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn get_reconciliation_rule() {}

#[utoipa::path(
    put,
    path = "/api/v1/reconciliation/rule",
    tag = "reconciliation",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = UpsertReconciliationRuleRequest,
    responses(
        (status = 200, body = ReconciliationRule),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn upsert_reconciliation_rule() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/claims",
    tag = "reconciliation",
    params(("Idempotency-Key" = String, Header, description = "服务 Worker 本轮认领幂等键")),
    request_body = ClaimReconciliationRequest,
    responses(
        (status = 200, body = ReconciliationClaimResponse),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn claim_reconciliation_window() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/claims/{id}/renew",
    tag = "reconciliation",
    params(
        ("id" = Uuid, Path, description = "调度认领 ID"),
        ("Idempotency-Key" = String, Header, description = "本次续租幂等键")
    ),
    request_body = RenewReconciliationClaimRequest,
    responses(
        (status = 200, body = ReconciliationClaimMutation),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn renew_reconciliation_claim() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/claims/{id}/failed",
    tag = "reconciliation",
    params(
        ("id" = Uuid, Path, description = "调度认领 ID"),
        ("Idempotency-Key" = String, Header, description = "失败上报幂等键")
    ),
    request_body = FailReconciliationClaimRequest,
    responses(
        (status = 200, body = ReconciliationClaimMutation),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn fail_reconciliation_claim() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/runs",
    tag = "reconciliation",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = SubmitReconciliationRunRequest,
    responses(
        (status = 200, body = ReconciliationRun),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn run_reconciliation() {}

#[utoipa::path(
    get,
    path = "/api/v1/reconciliation/items",
    tag = "reconciliation",
    params(ReconciliationItemQuery),
    responses(
        (status = 200, body = ReconciliationItemListResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn list_reconciliation_items() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/items/isolation",
    tag = "reconciliation",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = SetIsolationRequest,
    responses(
        (status = 200, body = i64),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn set_reconciliation_isolation() {}

#[utoipa::path(
    post,
    path = "/api/v1/reconciliation/items/{id}/resolve",
    tag = "reconciliation",
    params(
        ("id" = Uuid, Path, description = "对账差异 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ResolveReconciliationRequest,
    responses(
        (status = 200, body = ReconciliationItem),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
#[allow(dead_code)]
pub(crate) fn resolve_reconciliation_item() {}
