use chrono::{DateTime, Datelike, FixedOffset, NaiveTime, TimeZone, Utc};
use sqlx::PgPool;
use wms_domain::{
    validate_aggregation_rule, validate_cutoff_plan, validate_manual_delivery_note_cutoff,
    validate_print_suite, validate_route_binding, AggregationFieldCatalogResponse,
    AggregationRuleTestResult, AggregationRuleVersion, AggregationRuleVersionListResponse,
    CreateAggregationRuleDraftRequest, CreateCutoffPlanRequest, CreatePrintSuiteDraftRequest,
    CutoffPlan, CutoffPlanListResponse, DeliveryNoteCandidateListResponse,
    DeliveryNoteGroupListResponse, ManualDeliveryNoteCutoffRequest,
    PrintDocumentCategoryListResponse, PrintSuiteInstanceListResponse, PrintSuiteTestResult,
    PrintSuiteVersion, PrintSuiteVersionListResponse, PublishRouteBindingRequest, RouteBinding,
    RouteBindingListResponse, TestAggregationRuleRequest, TestPrintSuiteRequest,
};

use crate::auth::AuthContext;

use super::{
    repository::PgPrintOrchestrationRepository, DeliveryNoteGroup, IdempotentMutation,
    PrintOrchestrationError,
};

/// H9 delivery-note cutoff use cases.
#[derive(Clone, Debug)]
pub struct PrintOrchestrationService {
    repository: PgPrintOrchestrationRepository,
}

impl PrintOrchestrationService {
    /// Builds the H9 orchestration service with PostgreSQL persistence.
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            repository: PgPrintOrchestrationRepository::new(pool),
        }
    }

    /// Freezes one hard-boundary order set into a delivery-note group.
    pub async fn manual_cutoff(
        &self,
        ctx: &AuthContext,
        request: ManualDeliveryNoteCutoffRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DeliveryNoteGroup>, PrintOrchestrationError> {
        validate_manual_delivery_note_cutoff(&request)
            .map_err(|_| PrintOrchestrationError::InvalidRequest)?;
        if idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .manual_cutoff(ctx, request, now, idempotency_key)
            .await
    }

    /// Publishes one non-overlapping address-to-route binding.
    pub async fn publish_route_binding(
        &self,
        ctx: &AuthContext,
        request: PublishRouteBindingRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<RouteBinding>, PrintOrchestrationError> {
        validate_route_binding(&request).map_err(|_| PrintOrchestrationError::InvalidRequest)?;
        if idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .publish_route_binding(ctx, request, now, idempotency_key)
            .await
    }

    /// Lists owner-scoped address-to-route bindings.
    pub async fn list_route_bindings(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<uuid::Uuid>,
    ) -> Result<RouteBindingListResponse, PrintOrchestrationError> {
        self.repository.list_route_bindings(ctx, warehouse_id).await
    }

    /// Lists confirmed, route-frozen orders that have not been cut off.
    pub async fn list_delivery_note_candidates(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<uuid::Uuid>,
    ) -> Result<DeliveryNoteCandidateListResponse, PrintOrchestrationError> {
        self.repository
            .list_delivery_note_candidates(ctx, warehouse_id)
            .await
    }

    /// Lists the latest persisted delivery-note cutoff results.
    pub async fn list_delivery_note_groups(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<uuid::Uuid>,
    ) -> Result<DeliveryNoteGroupListResponse, PrintOrchestrationError> {
        self.repository
            .list_delivery_note_groups(ctx, warehouse_id)
            .await
    }

    /// Creates a validated cutoff-plan draft.
    pub async fn create_cutoff_plan(
        &self,
        ctx: &AuthContext,
        request: CreateCutoffPlanRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<CutoffPlan>, PrintOrchestrationError> {
        validate_cutoff_plan(&request).map_err(|_| PrintOrchestrationError::InvalidRequest)?;
        if idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .create_cutoff_plan(ctx, request, now, idempotency_key)
            .await
    }

    /// Lists owner-scoped cutoff plans.
    pub async fn list_cutoff_plans(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<uuid::Uuid>,
    ) -> Result<CutoffPlanListResponse, PrintOrchestrationError> {
        self.repository.list_cutoff_plans(ctx, warehouse_id).await
    }

    /// Publishes one draft after same-level overlap validation.
    pub async fn publish_cutoff_plan(
        &self,
        ctx: &AuthContext,
        plan_id: uuid::Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<CutoffPlan>, PrintOrchestrationError> {
        if plan_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .publish_cutoff_plan(ctx, plan_id, now, idempotency_key)
            .await
    }

    /// Lists the controlled standard-order fields available to equality grouping.
    pub async fn list_aggregation_fields(
        &self,
        _ctx: &AuthContext,
    ) -> Result<AggregationFieldCatalogResponse, PrintOrchestrationError> {
        self.repository.list_aggregation_fields().await
    }

    /// Lists all owner-scoped immutable aggregation-rule versions.
    pub async fn list_aggregation_rules(
        &self,
        ctx: &AuthContext,
    ) -> Result<AggregationRuleVersionListResponse, PrintOrchestrationError> {
        self.repository.list_aggregation_rules(ctx).await
    }

    /// Creates the next aggregation-rule draft from controlled dimensions.
    pub async fn create_aggregation_rule_draft(
        &self,
        ctx: &AuthContext,
        request: CreateAggregationRuleDraftRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        validate_aggregation_rule(&request).map_err(|_| PrintOrchestrationError::InvalidRequest)?;
        if idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .create_aggregation_rule_draft(ctx, request, now, idempotency_key)
            .await
    }

    /// Tests a draft against real owner-scoped sample orders.
    pub async fn test_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        request: TestAggregationRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleTestResult>, PrintOrchestrationError> {
        if version_id.is_nil()
            || request.order_ids.is_empty()
            || request.order_ids.iter().any(uuid::Uuid::is_nil)
            || request
                .order_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != request.order_ids.len()
            || idempotency_key.trim().is_empty()
        {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .test_aggregation_rule(ctx, version_id, request, now, idempotency_key)
            .await
    }

    /// Publishes a tested aggregation-rule version.
    pub async fn publish_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        if version_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .publish_aggregation_rule(ctx, version_id, now, idempotency_key)
            .await
    }

    /// Disables one published aggregation-rule version.
    pub async fn disable_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        if version_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .disable_aggregation_rule(ctx, version_id, now, idempotency_key)
            .await
    }

    /// Lists the controlled M1 print-document categories with source modes.
    pub async fn list_print_document_categories(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrintDocumentCategoryListResponse, PrintOrchestrationError> {
        self.repository.list_print_document_categories(ctx).await
    }

    /// Lists all owner-scoped immutable print-suite versions with items.
    pub async fn list_print_suites(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrintSuiteVersionListResponse, PrintOrchestrationError> {
        self.repository.list_print_suites(ctx).await
    }

    /// Creates the next print-suite draft with ordered, policy-frozen items.
    pub async fn create_print_suite_draft(
        &self,
        ctx: &AuthContext,
        request: CreatePrintSuiteDraftRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        validate_print_suite(&request).map_err(|_| PrintOrchestrationError::InvalidRequest)?;
        if idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .create_print_suite_draft(ctx, request, now, idempotency_key)
            .await
    }

    /// Runs the readiness/completeness precheck against real sample groups.
    pub async fn test_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        request: TestPrintSuiteRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteTestResult>, PrintOrchestrationError> {
        if version_id.is_nil()
            || request.group_ids.is_empty()
            || request.group_ids.iter().any(uuid::Uuid::is_nil)
            || request
                .group_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != request.group_ids.len()
            || idempotency_key.trim().is_empty()
        {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .test_print_suite(ctx, version_id, request, now, idempotency_key)
            .await
    }

    /// Publishes a tested print-suite version after same-level overlap checks.
    pub async fn publish_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        if version_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .publish_print_suite(ctx, version_id, now, idempotency_key)
            .await
    }

    /// Disables one published print-suite version.
    pub async fn disable_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: uuid::Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        if version_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .disable_print_suite(ctx, version_id, now, idempotency_key)
            .await
    }

    /// Lists frozen suite instances, optionally for one delivery-note group.
    pub async fn list_print_suite_instances(
        &self,
        ctx: &AuthContext,
        group_id: Option<uuid::Uuid>,
    ) -> Result<PrintSuiteInstanceListResponse, PrintOrchestrationError> {
        self.repository
            .list_print_suite_instances(ctx, group_id)
            .await
    }

    /// Resolves the effective plan using customer > route > owner+warehouse.
    pub async fn resolve_cutoff_plan(
        &self,
        ctx: &AuthContext,
        warehouse_id: uuid::Uuid,
        customer_id: uuid::Uuid,
        route_code: &str,
        effective_at: DateTime<Utc>,
    ) -> Result<CutoffPlan, PrintOrchestrationError> {
        if warehouse_id.is_nil() || customer_id.is_nil() || route_code.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        self.repository
            .resolve_cutoff_plan(ctx, warehouse_id, customer_id, route_code, effective_at)
            .await
    }

    /// Runs owner-scoped due cutoffs; H-SCH can call this idempotent use case.
    pub async fn run_scheduled_cutoffs(
        &self,
        ctx: &AuthContext,
        now: DateTime<Utc>,
    ) -> Result<Vec<DeliveryNoteGroup>, PrintOrchestrationError> {
        let boundaries = self
            .repository
            .list_pending_cutoff_boundaries(ctx.owner_id)
            .await?;
        let mut groups = Vec::new();
        for boundary in boundaries {
            let plan = match self
                .repository
                .resolve_cutoff_plan(
                    ctx,
                    boundary.warehouse_id,
                    boundary.customer_id,
                    &boundary.route_code,
                    now,
                )
                .await
            {
                Ok(plan) => plan,
                Err(PrintOrchestrationError::CutoffPlanNotFound) => continue,
                Err(error) => return Err(error),
            };
            let Some(scheduled_at) = scheduled_cutoff_at(&plan, now)? else {
                continue;
            };
            groups.extend(
                self.repository
                    .scheduled_cutoff(ctx, &plan, &boundary, scheduled_at, now)
                    .await?,
            );
        }
        Ok(groups)
    }
}

fn scheduled_cutoff_at(
    plan: &CutoffPlan,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, PrintOrchestrationError> {
    let offset = FixedOffset::east_opt(i32::from(plan.utc_offset_minutes) * 60)
        .ok_or_else(|| PrintOrchestrationError::Serialize("invalid UTC offset".to_string()))?;
    let local_now = now.with_timezone(&offset);
    let local_date = local_now.date_naive();
    let cutoff_time = if let Some(exception) = plan
        .exceptions
        .iter()
        .find(|exception| exception.date == local_date)
    {
        exception.cutoff_time.as_deref()
    } else {
        let weekday = local_date.weekday().number_from_monday() as u8;
        plan.weekly_schedule
            .iter()
            .find(|slot| slot.weekday == weekday)
            .map(|slot| slot.cutoff_time.as_str())
    };
    let Some(cutoff_time) = cutoff_time else {
        return Ok(None);
    };
    let cutoff_time = NaiveTime::parse_from_str(cutoff_time, "%H:%M")
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    let scheduled_at = offset
        .from_local_datetime(&local_date.and_time(cutoff_time))
        .single()
        .ok_or_else(|| PrintOrchestrationError::Serialize("invalid local cutoff".to_string()))?
        .with_timezone(&Utc);
    Ok((scheduled_at <= now).then_some(scheduled_at))
}
