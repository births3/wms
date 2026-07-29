//! US-H9-008 print suite versions, readiness precheck and instance snapshots.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreatePrintSuiteDraftRequest, DeliveryNoteGroup, PrintDocumentCategory,
    PrintDocumentCategoryListResponse, PrintSuiteFileBinding, PrintSuiteInstance,
    PrintSuiteInstanceItem, PrintSuiteInstanceListResponse, PrintSuiteItem,
    PrintSuiteItemReadiness, PrintSuiteReadyPolicy, PrintSuiteScope, PrintSuiteSourceMode,
    PrintSuiteTestResult, PrintSuiteTestSample, PrintSuiteVersion, PrintSuiteVersionListResponse,
    TestPrintSuiteRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::{
    repository::{
        json_request_hash, lock_idempotency_key, map_db_error, replay_idempotency,
        store_idempotency_success, PgPrintOrchestrationRepository,
    },
    IdempotentMutation, PrintOrchestrationError,
};

#[derive(Debug, FromRow)]
struct SuiteVersionRow {
    id: Uuid,
    owner_id: Uuid,
    version_no: i32,
    name: String,
    status: String,
    warehouse_id: Uuid,
    scope_type: String,
    customer_id: Option<Uuid>,
    delivery_address_id: Option<Uuid>,
    route_code: Option<String>,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    tested_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    disabled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct SuiteItemRow {
    id: Uuid,
    category_code: String,
    copies: i32,
    sort_order: i32,
    output_slot: String,
    required: bool,
    ready_policy: String,
    failure_policy: String,
    source_mode: String,
    template_version_id: Option<Uuid>,
    external_file_ref: Option<String>,
}

#[derive(Debug, FromRow)]
struct GroupOrderRow {
    id: Uuid,
    wms_order_no: String,
    erp_order_no: Option<String>,
    invoice_no: Option<String>,
}

#[derive(Debug, FromRow)]
struct IngestedFileRow {
    id: Uuid,
    file_ref: String,
    file_version: i32,
    content_hash: String,
    invoice_no: Option<String>,
    product_code: Option<String>,
    batch_no: Option<String>,
}

#[derive(Debug, FromRow)]
struct GroupBoundaryRow {
    warehouse_id: Uuid,
    customer_id: Uuid,
    delivery_address_id: Uuid,
    route_code: String,
    delivery_note_no: String,
    cutoff_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct InstanceRow {
    id: Uuid,
    owner_id: Uuid,
    group_id: Uuid,
    delivery_note_no: String,
    suite_version_id: Uuid,
    suite_version_no: i32,
    suite_snapshot: Value,
    aggregation_rule_version_id: Option<Uuid>,
    aggregation_rule_version_no: Option<i32>,
    source_documents: Value,
    status: String,
    hold_scope: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct InstanceItemRow {
    id: Uuid,
    category_code: String,
    copies: i32,
    sort_order: i32,
    output_slot: String,
    required: bool,
    ready_policy: String,
    failure_policy: String,
    source_mode: String,
    template_version_id: Option<Uuid>,
    external_file_ref: Option<String>,
    file_bindings: Value,
    ready: bool,
    missing: Value,
}

impl PgPrintOrchestrationRepository {
    pub(super) async fn list_print_document_categories(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrintDocumentCategoryListResponse, PrintOrchestrationError> {
        let rows: Vec<(String, String, Value)> = sqlx::query_as(
            r#"
            SELECT item_code, item_name, params
              FROM system_dictionary_items
             WHERE dict_code = 'print_document_category'
               AND enabled
               AND (owner_id IS NULL OR owner_id = $1)
             ORDER BY item_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data = rows
            .into_iter()
            .filter_map(|(item_code, item_name, params)| {
                let source_mode = params.get("source_mode")?.as_str()?;
                let source_mode = PrintSuiteSourceMode::try_from(source_mode).ok()?;
                Some(PrintDocumentCategory {
                    item_code,
                    item_name,
                    source_mode,
                })
            })
            .collect();
        Ok(PrintDocumentCategoryListResponse { data })
    }

    pub(super) async fn list_print_suites(
        &self,
        ctx: &AuthContext,
    ) -> Result<PrintSuiteVersionListResponse, PrintOrchestrationError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, SuiteVersionRow>(
            r#"
            SELECT id, owner_id, version_no, name, status, warehouse_id, scope_type,
                   customer_id, delivery_address_id, route_code, effective_from,
                   effective_to, tested_at, published_at, disabled_at, created_at
              FROM h9_print_suite_versions
             WHERE owner_id = $1
             ORDER BY version_no DESC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let categories = load_category_names(&mut tx, ctx.owner_id).await?;
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            let items = load_suite_items(&mut tx, ctx.owner_id, row.id, &categories).await?;
            data.push(map_suite_version(row, items)?);
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(PrintSuiteVersionListResponse { data })
    }

    pub(super) async fn create_print_suite_draft(
        &self,
        ctx: &AuthContext,
        request: CreatePrintSuiteDraftRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(suite) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: suite,
                replayed: true,
            });
        }
        lock_suite_versions(&mut tx, ctx.owner_id).await?;
        ensure_suite_scope_exists(&mut tx, ctx.owner_id, &request).await?;
        let categories = load_category_names(&mut tx, ctx.owner_id).await?;
        ensure_registered_categories(&mut tx, ctx.owner_id, &request, &categories).await?;
        ensure_rendered_template_bindings(&mut tx, ctx.owner_id, &request).await?;
        let version_no: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_no), 0) + 1 FROM h9_print_suite_versions WHERE owner_id = $1",
        )
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let suite_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, SuiteVersionRow>(
            r#"
            INSERT INTO h9_print_suite_versions (
                id, owner_id, version_no, name, status, warehouse_id, scope_type,
                customer_id, delivery_address_id, route_code,
                effective_from, effective_to, created_by, created_at
            )
            VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, owner_id, version_no, name, status, warehouse_id, scope_type,
                      customer_id, delivery_address_id, route_code, effective_from,
                      effective_to, tested_at, published_at, disabled_at, created_at
            "#,
        )
        .bind(suite_id)
        .bind(ctx.owner_id)
        .bind(version_no)
        .bind(request.name.trim())
        .bind(request.warehouse_id)
        .bind(request.scope.as_str())
        .bind(request.customer_id)
        .bind(request.delivery_address_id)
        .bind(request.route_code.as_deref().map(str::trim))
        .bind(request.effective_from)
        .bind(request.effective_to)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        for item in &request.items {
            sqlx::query(
                r#"
                INSERT INTO h9_print_suite_items (
                    id, owner_id, suite_version_id, category_code, copies, sort_order,
                    output_slot, required, ready_policy, failure_policy, source_mode,
                    template_version_id, external_file_ref, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(suite_id)
            .bind(item.category_code.trim())
            .bind(item.copies)
            .bind(item.sort_order)
            .bind(item.output_slot.trim())
            .bind(item.required)
            .bind(item.ready_policy.as_str())
            .bind(item.failure_policy.as_str())
            .bind(item.source_mode.as_str())
            .bind(item.template_version_id)
            .bind(item.external_file_ref.as_deref().map(str::trim))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let items = load_suite_items(&mut tx, ctx.owner_id, suite_id, &categories).await?;
        let suite = map_suite_version(row, items)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "/api/v1/print-orchestration/print-suites/versions",
            "print_suite_version",
            &suite,
            now,
        )
        .await?;
        append_suite_audit(&mut tx, ctx, "create_print_suite_draft", &suite, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: suite,
            replayed: false,
        })
    }

    pub(super) async fn test_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        request: TestPrintSuiteRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteTestResult>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({
            "version_id": version_id,
            "group_ids": request.group_ids,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(result) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: result,
                replayed: true,
            });
        }
        let row = load_suite_for_update(&mut tx, ctx.owner_id, version_id).await?;
        if !matches!(row.status.as_str(), "draft" | "tested") {
            return Err(PrintOrchestrationError::PrintSuiteInvalidState);
        }
        let categories = load_category_names(&mut tx, ctx.owner_id).await?;
        let items = load_suite_items(&mut tx, ctx.owner_id, version_id, &categories).await?;
        let candidate = map_suite_version(row, items)?;
        let mut samples = Vec::with_capacity(request.group_ids.len());
        for group_id in &request.group_ids {
            let boundary = load_group_boundary(&mut tx, ctx.owner_id, *group_id).await?;
            let matches_this_version = suite_matches_boundary(&candidate, &boundary);
            let resolved_scope = resolve_scope_with_candidate(
                &mut tx,
                ctx.owner_id,
                &boundary,
                matches_this_version.then_some(candidate.scope),
            )
            .await?;
            let orders = load_group_orders(&mut tx, ctx.owner_id, *group_id).await?;
            let mut item_readiness = Vec::with_capacity(candidate.items.len());
            for item in &candidate.items {
                let readiness =
                    compute_item_readiness(&mut tx, ctx.owner_id, item, &orders).await?;
                item_readiness.push(readiness);
            }
            samples.push(PrintSuiteTestSample {
                group_id: *group_id,
                delivery_note_no: boundary.delivery_note_no,
                resolved_scope,
                matches_this_version,
                item_readiness,
            });
        }
        let stored_result = serde_json::to_value(&samples).map_err(serialize_error)?;
        let row = sqlx::query_as::<_, SuiteVersionRow>(
            r#"
            UPDATE h9_print_suite_versions
               SET status = 'tested', test_result = $3,
                   tested_by = $4, tested_at = $5
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, version_no, name, status, warehouse_id, scope_type,
                      customer_id, delivery_address_id, route_code, effective_from,
                      effective_to, tested_at, published_at, disabled_at, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(stored_result)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let items = load_suite_items(&mut tx, ctx.owner_id, version_id, &categories).await?;
        let result = PrintSuiteTestResult {
            suite: map_suite_version(row, items)?,
            samples,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-orchestration/print-suites/versions/{version_id}/test"),
            "print_suite_test",
            &result,
            now,
        )
        .await?;
        append_suite_audit(&mut tx, ctx, "test_print_suite", &result.suite, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: result,
            replayed: false,
        })
    }

    pub(super) async fn publish_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({
            "version_id": version_id,
            "target_status": "published",
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(suite) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: suite,
                replayed: true,
            });
        }
        let row = load_suite_for_update(&mut tx, ctx.owner_id, version_id).await?;
        if row.status != "tested" {
            return Err(PrintOrchestrationError::PrintSuiteInvalidState);
        }
        lock_suite_scope(&mut tx, &row).await?;
        if suite_period_overlaps(&mut tx, &row).await? {
            return Err(PrintOrchestrationError::EffectivePeriodOverlap);
        }
        let row = sqlx::query_as::<_, SuiteVersionRow>(
            r#"
            UPDATE h9_print_suite_versions
               SET status = 'published', published_by = $3, published_at = $4
             WHERE owner_id = $1 AND id = $2 AND status = 'tested'
            RETURNING id, owner_id, version_no, name, status, warehouse_id, scope_type,
                      customer_id, delivery_address_id, route_code, effective_from,
                      effective_to, tested_at, published_at, disabled_at, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintOrchestrationError::PrintSuiteInvalidState)?;
        let categories = load_category_names(&mut tx, ctx.owner_id).await?;
        let items = load_suite_items(&mut tx, ctx.owner_id, version_id, &categories).await?;
        let suite = map_suite_version(row, items)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-orchestration/print-suites/versions/{version_id}/publish"),
            "print_suite_version",
            &suite,
            now,
        )
        .await?;
        append_suite_audit(&mut tx, ctx, "publish_print_suite", &suite, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: suite,
            replayed: false,
        })
    }

    pub(super) async fn disable_print_suite(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintSuiteVersion>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({
            "version_id": version_id,
            "target_status": "disabled",
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(suite) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: suite,
                replayed: true,
            });
        }
        let row = load_suite_for_update(&mut tx, ctx.owner_id, version_id).await?;
        if row.status != "published" {
            return Err(PrintOrchestrationError::PrintSuiteInvalidState);
        }
        let row = sqlx::query_as::<_, SuiteVersionRow>(
            r#"
            UPDATE h9_print_suite_versions
               SET status = 'disabled', disabled_by = $3, disabled_at = $4
             WHERE owner_id = $1 AND id = $2 AND status = 'published'
            RETURNING id, owner_id, version_no, name, status, warehouse_id, scope_type,
                      customer_id, delivery_address_id, route_code, effective_from,
                      effective_to, tested_at, published_at, disabled_at, created_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintOrchestrationError::PrintSuiteInvalidState)?;
        let categories = load_category_names(&mut tx, ctx.owner_id).await?;
        let items = load_suite_items(&mut tx, ctx.owner_id, version_id, &categories).await?;
        let suite = map_suite_version(row, items)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &format!("/api/v1/print-orchestration/print-suites/versions/{version_id}/disable"),
            "print_suite_version",
            &suite,
            now,
        )
        .await?;
        append_suite_audit(&mut tx, ctx, "disable_print_suite", &suite, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: suite,
            replayed: false,
        })
    }

    pub(super) async fn list_print_suite_instances(
        &self,
        ctx: &AuthContext,
        group_id: Option<Uuid>,
    ) -> Result<PrintSuiteInstanceListResponse, PrintOrchestrationError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, InstanceRow>(
            r#"
            SELECT instance.id, instance.owner_id, instance.group_id,
                   grp.delivery_note_no, instance.suite_version_id,
                   instance.suite_version_no, instance.suite_snapshot,
                   instance.aggregation_rule_version_id,
                   instance.aggregation_rule_version_no,
                   instance.source_documents, instance.status,
                   instance.hold_scope, instance.created_at
              FROM h9_print_suite_instances instance
              JOIN h9_delivery_note_groups grp
                ON grp.owner_id = instance.owner_id AND grp.id = instance.group_id
             WHERE instance.owner_id = $1
               AND ($2::uuid IS NULL OR instance.group_id = $2)
             ORDER BY instance.created_at DESC, instance.id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(group_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            let items = load_instance_items(&mut tx, ctx.owner_id, row.id).await?;
            data.push(map_instance(row, items)?);
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(PrintSuiteInstanceListResponse { data })
    }

    /// US-H9-008 AC7/AC8: freezes suite version, rule version, source-document
    /// snapshot and per-item policies for one delivery-note group. Returns
    /// `None` when no published suite resolves, keeping US-H9-006 cutoff
    /// behaviour unchanged.
    pub(super) async fn create_suite_instance_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        group: &DeliveryNoteGroup,
        now: DateTime<Utc>,
    ) -> Result<Option<PrintSuiteInstance>, PrintOrchestrationError> {
        let boundary = GroupBoundaryRow {
            warehouse_id: group.warehouse_id,
            customer_id: group.customer_id,
            delivery_address_id: group.delivery_address_id,
            route_code: group.route_code.clone(),
            delivery_note_no: group.delivery_note_no.clone(),
            cutoff_at: group.cutoff_at,
        };
        let Some(row) = resolve_published_suite(tx, ctx.owner_id, &boundary).await? else {
            return Ok(None);
        };
        let categories = load_category_names(tx, ctx.owner_id).await?;
        let items = load_suite_items(tx, ctx.owner_id, row.id, &categories).await?;
        let suite = map_suite_version(row, items)?;
        let orders = load_group_orders(tx, ctx.owner_id, group.id).await?;
        let source_documents = orders
            .iter()
            .map(|order| {
                json!({
                    "order_id": order.id,
                    "wms_order_no": order.wms_order_no,
                    "erp_order_no": order.erp_order_no,
                    "invoice_no": order.invoice_no,
                })
            })
            .collect::<Vec<_>>();
        let mut instance_items = Vec::with_capacity(suite.items.len());
        for item in &suite.items {
            let readiness = compute_item_readiness(tx, ctx.owner_id, item, &orders).await?;
            instance_items.push((item.clone(), readiness));
        }
        let pause_queue_policy = instance_items.iter().any(|(item, readiness)| {
            item.required
                && !readiness.ready
                && item.ready_policy == PrintSuiteReadyPolicy::PauseAgentQueue
        });
        let (status, hold_scope) = if pause_queue_policy {
            ("waiting_documents", Some("agent_queue"))
        } else {
            // US-H9-009: source readiness is necessary but not sufficient.
            // The instance queues only after every category PDF is prepared.
            ("waiting_documents", Some("instance"))
        };
        let instance_id = Uuid::new_v4();
        let suite_snapshot = serde_json::to_value(&suite).map_err(serialize_error)?;
        sqlx::query(
            r#"
            INSERT INTO h9_print_suite_instances (
                id, owner_id, group_id, suite_version_id, suite_version_no,
                suite_snapshot, aggregation_rule_version_id,
                aggregation_rule_version_no, source_documents, status,
                hold_scope, created_by, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(instance_id)
        .bind(ctx.owner_id)
        .bind(group.id)
        .bind(suite.id)
        .bind(suite.version_no)
        .bind(&suite_snapshot)
        .bind(group.aggregation_rule_version_id)
        .bind(group.aggregation_rule_version_no)
        .bind(Value::Array(source_documents.clone()))
        .bind(status)
        .bind(hold_scope)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let mut frozen_items = Vec::with_capacity(instance_items.len());
        for (item, readiness) in instance_items {
            let item_id = Uuid::new_v4();
            let file_bindings =
                serde_json::to_value(&readiness.file_bindings).map_err(serialize_error)?;
            let missing = serde_json::to_value(&readiness.missing).map_err(serialize_error)?;
            sqlx::query(
                r#"
                INSERT INTO h9_print_suite_instance_items (
                    id, owner_id, instance_id, category_code, copies, sort_order,
                    output_slot, required, ready_policy, failure_policy,
                    source_mode, template_version_id, external_file_ref,
                    file_bindings, ready, missing, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                "#,
            )
            .bind(item_id)
            .bind(ctx.owner_id)
            .bind(instance_id)
            .bind(&item.category_code)
            .bind(item.copies)
            .bind(item.sort_order)
            .bind(&item.output_slot)
            .bind(item.required)
            .bind(item.ready_policy.as_str())
            .bind(item.failure_policy.as_str())
            .bind(item.source_mode.as_str())
            .bind(item.template_version_id)
            .bind(item.external_file_ref.as_deref())
            .bind(&file_bindings)
            .bind(readiness.ready)
            .bind(&missing)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
            frozen_items.push(PrintSuiteInstanceItem {
                id: item_id,
                category_code: item.category_code,
                copies: item.copies,
                sort_order: item.sort_order,
                output_slot: item.output_slot,
                required: item.required,
                ready_policy: item.ready_policy,
                failure_policy: item.failure_policy,
                source_mode: item.source_mode,
                template_version_id: item.template_version_id,
                external_file_ref: item.external_file_ref,
                file_bindings: readiness.file_bindings,
                ready: readiness.ready,
                missing: readiness.missing,
            });
        }
        let instance = PrintSuiteInstance {
            id: instance_id,
            owner_id: ctx.owner_id,
            group_id: group.id,
            delivery_note_no: group.delivery_note_no.clone(),
            suite_version_id: suite.id,
            suite_version_no: suite.version_no,
            suite_snapshot,
            aggregation_rule_version_id: group.aggregation_rule_version_id,
            aggregation_rule_version_no: group.aggregation_rule_version_no,
            source_documents: Value::Array(source_documents),
            status: status.to_string(),
            hold_scope: hold_scope.map(str::to_string),
            items: frozen_items,
            created_at: now,
        };
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "create_print_suite_instance",
            "H9",
            "print_suite_instance",
            instance.id.to_string(),
            Some(AuditDiff::compute(
                Value::Null,
                serde_json::to_value(&instance).map_err(serialize_error)?,
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(tx, &audit)
            .await
            .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
        Ok(Some(instance))
    }
}

include!("print_suite_support.rs");
