use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    AggregationDimension, AggregationFieldCatalogResponse, AggregationFieldCode,
    AggregationFieldDefinition, AggregationGroupKeyItem, AggregationMethod,
    AggregationRuleTestGroup, AggregationRuleTestResult, AggregationRuleVersion,
    AggregationRuleVersionListResponse, CreateAggregationRuleDraftRequest,
    TestAggregationRuleRequest,
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
struct AggregationFieldRow {
    field_code: String,
    display_name: String,
    value_type: String,
}

#[derive(Debug, FromRow)]
struct AggregationRuleRow {
    id: Uuid,
    owner_id: Uuid,
    version_no: i32,
    name: String,
    status: String,
    dimensions: Value,
    tested_at: Option<DateTime<Utc>>,
    published_at: Option<DateTime<Utc>>,
    disabled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct AggregationOrderRow {
    id: Uuid,
    wms_order_no: String,
    warehouse_id: Uuid,
    delivery_address_id: Uuid,
    document_type: String,
    erp_order_no: Option<String>,
    invoice_no: Option<String>,
    transport_mode_code: Option<String>,
    department_code: Option<String>,
    sales_group_code: Option<String>,
    order_group_no: Option<String>,
    business_type_code: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct AggregationRuleApplication {
    pub(super) version_id: Option<Uuid>,
    pub(super) version_no: Option<i32>,
    pub(super) snapshot: Value,
    pub(super) group_key: Value,
}

pub(super) struct AggregationRulePartition {
    pub(super) order_ids: Vec<Uuid>,
    pub(super) application: AggregationRuleApplication,
}

impl PgPrintOrchestrationRepository {
    pub(super) async fn list_aggregation_fields(
        &self,
    ) -> Result<AggregationFieldCatalogResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, AggregationFieldRow>(
            r#"
            SELECT field_code, display_name, value_type
              FROM h9_aggregation_field_catalog
             WHERE enabled
             ORDER BY sort_order, field_code
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let data = rows
            .into_iter()
            .map(|row| {
                Ok::<_, PrintOrchestrationError>(AggregationFieldDefinition {
                    field_code: AggregationFieldCode::try_from(row.field_code.as_str())
                        .map_err(|_| PrintOrchestrationError::InvalidRequest)?,
                    display_name: row.display_name,
                    value_type: row.value_type,
                    method: AggregationMethod::Equals,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(AggregationFieldCatalogResponse { data })
    }

    pub(super) async fn list_aggregation_rules(
        &self,
        ctx: &AuthContext,
    ) -> Result<AggregationRuleVersionListResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, AggregationRuleRow>(
            r#"
            SELECT id, owner_id, version_no, name, status, dimensions,
                   tested_at, published_at, disabled_at, created_at
              FROM h9_aggregation_rule_versions
             WHERE owner_id = $1
             ORDER BY version_no DESC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(AggregationRuleVersionListResponse {
            data: rows
                .into_iter()
                .map(AggregationRuleVersion::try_from)
                .collect::<Result<_, _>>()?,
        })
    }

    pub(super) async fn create_aggregation_rule_draft(
        &self,
        ctx: &AuthContext,
        request: CreateAggregationRuleDraftRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&request)?;
        let path = "/api/v1/print-orchestration/aggregation-rules/versions";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(rule) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: rule,
                replayed: true,
            });
        }
        lock_rule_versions(&mut tx, ctx.owner_id).await?;
        ensure_registered_dimensions(&mut tx, &request.dimensions).await?;
        let version_no: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_no), 0) + 1 FROM h9_aggregation_rule_versions WHERE owner_id = $1",
        )
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let id = Uuid::new_v4();
        let dimensions = serde_json::to_value(&request.dimensions).map_err(serialize_error)?;
        let row = sqlx::query_as::<_, AggregationRuleRow>(
            r#"
            INSERT INTO h9_aggregation_rule_versions (
                id, owner_id, version_no, name, status, dimensions,
                created_by, created_at
            )
            VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7)
            RETURNING id, owner_id, version_no, name, status, dimensions,
                      tested_at, published_at, disabled_at, created_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(version_no)
        .bind(request.name.trim())
        .bind(dimensions)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let rule = AggregationRuleVersion::try_from(row)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            path,
            "aggregation_rule_version",
            &rule,
            now,
        )
        .await?;
        append_rule_audit(&mut tx, ctx, "create_aggregation_rule_draft", &rule, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: rule,
            replayed: false,
        })
    }

    pub(super) async fn test_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        request: TestAggregationRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleTestResult>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({
            "version_id": version_id,
            "order_ids": request.order_ids,
        }))?;
        let path =
            format!("/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/test");
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(result) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: result,
                replayed: true,
            });
        }
        let row = load_rule_for_update(&mut tx, ctx.owner_id, version_id).await?;
        if !matches!(row.status.as_str(), "draft" | "tested") {
            return Err(PrintOrchestrationError::AggregationRuleInvalidState);
        }
        let dimensions = parse_dimensions(&row.dimensions)?;
        let orders = load_aggregation_orders(&mut tx, ctx.owner_id, &request.order_ids).await?;
        if orders.len() != request.order_ids.len() {
            return Err(PrintOrchestrationError::OrderNotFound);
        }
        let groups = build_test_groups(&orders, &dimensions);
        let stored_result = serde_json::to_value(&groups).map_err(serialize_error)?;
        let row = sqlx::query_as::<_, AggregationRuleRow>(
            r#"
            UPDATE h9_aggregation_rule_versions
               SET status = 'tested', test_result = $3,
                   tested_by = $4, tested_at = $5
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, version_no, name, status, dimensions,
                      tested_at, published_at, disabled_at, created_at
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
        let result = AggregationRuleTestResult {
            rule: AggregationRuleVersion::try_from(row)?,
            groups,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &path,
            "aggregation_rule_test",
            &result,
            now,
        )
        .await?;
        append_rule_audit(&mut tx, ctx, "test_aggregation_rule", &result.rule, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: result,
            replayed: false,
        })
    }

    pub(super) async fn publish_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        self.transition_aggregation_rule(
            ctx,
            version_id,
            "tested",
            "published",
            now,
            idempotency_key,
        )
        .await
    }

    pub(super) async fn disable_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        self.transition_aggregation_rule(
            ctx,
            version_id,
            "published",
            "disabled",
            now,
            idempotency_key,
        )
        .await
    }

    async fn transition_aggregation_rule(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        expected_status: &str,
        target_status: &str,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<AggregationRuleVersion>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&json!({
            "version_id": version_id,
            "target_status": target_status,
        }))?;
        let endpoint = format!(
            "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/{target_status}"
        );
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(rule) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &endpoint,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: rule,
                replayed: true,
            });
        }
        lock_rule_versions(&mut tx, ctx.owner_id).await?;
        let before = load_rule_for_update(&mut tx, ctx.owner_id, version_id).await?;
        if before.status != expected_status {
            return Err(PrintOrchestrationError::AggregationRuleInvalidState);
        }
        if target_status == "published" {
            sqlx::query(
                r#"
                UPDATE h9_aggregation_rule_versions
                   SET status = 'disabled', disabled_by = $2, disabled_at = $3
                 WHERE owner_id = $1 AND status = 'published'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(ctx.user_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let row = if target_status == "published" {
            sqlx::query_as::<_, AggregationRuleRow>(
                r#"
                UPDATE h9_aggregation_rule_versions
                   SET status = 'published', published_by = $3, published_at = $4
                 WHERE owner_id = $1 AND id = $2
                RETURNING id, owner_id, version_no, name, status, dimensions,
                          tested_at, published_at, disabled_at, created_at
                "#,
            )
            .bind(ctx.owner_id)
            .bind(version_id)
            .bind(ctx.user_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            sqlx::query_as::<_, AggregationRuleRow>(
                r#"
                UPDATE h9_aggregation_rule_versions
                   SET status = 'disabled', disabled_by = $3, disabled_at = $4
                 WHERE owner_id = $1 AND id = $2
                RETURNING id, owner_id, version_no, name, status, dimensions,
                          tested_at, published_at, disabled_at, created_at
                "#,
            )
            .bind(ctx.owner_id)
            .bind(version_id)
            .bind(ctx.user_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        };
        let rule = AggregationRuleVersion::try_from(row)?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &endpoint,
            "aggregation_rule_version",
            &rule,
            now,
        )
        .await?;
        append_rule_audit(
            &mut tx,
            ctx,
            if target_status == "published" {
                "publish_aggregation_rule"
            } else {
                "disable_aggregation_rule"
            },
            &rule,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: rule,
            replayed: false,
        })
    }
}

pub(super) async fn resolve_rule_application_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<AggregationRuleApplication, PrintOrchestrationError> {
    let mut partitions = partition_orders_by_rule_in_tx(tx, owner_id, order_ids).await?;
    if partitions.len() != 1 {
        return Err(PrintOrchestrationError::AggregationRuleMismatch);
    }
    Ok(partitions
        .pop()
        .expect("one partition was checked")
        .application)
}

pub(super) async fn partition_orders_by_rule_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<Vec<AggregationRulePartition>, PrintOrchestrationError> {
    let rule = sqlx::query_as::<_, AggregationRuleRow>(
        r#"
        SELECT id, owner_id, version_no, name, status, dimensions,
               tested_at, published_at, disabled_at, created_at
          FROM h9_aggregation_rule_versions
         WHERE owner_id = $1 AND status = 'published'
         FOR SHARE
        "#,
    )
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some(rule) = rule else {
        return Ok(vec![AggregationRulePartition {
            order_ids: order_ids.to_vec(),
            application: AggregationRuleApplication {
                version_id: None,
                version_no: None,
                snapshot: json!({}),
                group_key: json!({}),
            },
        }]);
    };
    let dimensions = parse_dimensions(&rule.dimensions)?;
    let orders = load_aggregation_orders(tx, owner_id, order_ids).await?;
    if orders.len() != order_ids.len() {
        return Err(PrintOrchestrationError::OrderNotFound);
    }
    let snapshot = json!({
        "version_id": rule.id,
        "version_no": rule.version_no,
        "name": rule.name,
        "dimensions": dimensions,
    });
    let mut groups = BTreeMap::<String, (Value, Vec<Uuid>)>::new();
    for order in orders {
        let group_key = build_group_key(&order, &dimensions);
        let key = serde_json::to_string(&group_key).map_err(serialize_error)?;
        groups
            .entry(key)
            .or_insert_with(|| (group_key, Vec::new()))
            .1
            .push(order.id);
    }
    Ok(groups
        .into_values()
        .map(|(group_key, order_ids)| AggregationRulePartition {
            order_ids,
            application: AggregationRuleApplication {
                version_id: Some(rule.id),
                version_no: Some(rule.version_no),
                snapshot: snapshot.clone(),
                group_key,
            },
        })
        .collect())
}

async fn load_aggregation_orders(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<Vec<AggregationOrderRow>, PrintOrchestrationError> {
    sqlx::query_as::<_, AggregationOrderRow>(
        r#"
        SELECT order_row.id, order_row.wms_order_no,
               snapshot.warehouse_id, snapshot.delivery_address_id,
               order_row.document_type, order_row.erp_order_no,
               order_row.invoice_no, order_row.transport_mode_code,
               order_row.department_code, order_row.sales_group_code,
               order_row.order_group_no, order_row.business_type_code
          FROM outbound_orders order_row
          JOIN h9_outbound_route_snapshots snapshot
            ON snapshot.owner_id = order_row.owner_id
           AND snapshot.outbound_order_id = order_row.id
         WHERE order_row.owner_id = $1 AND order_row.id = ANY($2)
         ORDER BY order_row.wms_order_no, order_row.id
        "#,
    )
    .bind(owner_id)
    .bind(order_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

fn build_test_groups(
    orders: &[AggregationOrderRow],
    dimensions: &[AggregationDimension],
) -> Vec<AggregationRuleTestGroup> {
    let mut groups = BTreeMap::<String, AggregationRuleTestGroup>::new();
    for order in orders {
        let group_key = build_group_key_items(order, dimensions);
        let key = format!(
            "{}:{}:{}",
            order.warehouse_id,
            order.delivery_address_id,
            serde_json::to_string(&group_key).unwrap_or_default()
        );
        let group = groups
            .entry(key)
            .or_insert_with(|| AggregationRuleTestGroup {
                warehouse_id: order.warehouse_id,
                delivery_address_id: order.delivery_address_id,
                group_key,
                order_ids: Vec::new(),
                order_nos: Vec::new(),
            });
        group.order_ids.push(order.id);
        group.order_nos.push(order.wms_order_no.clone());
    }
    groups.into_values().collect()
}

fn build_group_key(order: &AggregationOrderRow, dimensions: &[AggregationDimension]) -> Value {
    let mut key = Map::new();
    for dimension in dimensions {
        key.insert(
            dimension.field_code.as_str().to_string(),
            Value::String(order_value(order, dimension.field_code).to_string()),
        );
    }
    Value::Object(key)
}

fn build_group_key_items(
    order: &AggregationOrderRow,
    dimensions: &[AggregationDimension],
) -> Vec<AggregationGroupKeyItem> {
    dimensions
        .iter()
        .map(|dimension| AggregationGroupKeyItem {
            field_code: dimension.field_code.as_str().to_string(),
            display_name: field_display_name(dimension.field_code).to_string(),
            value: order_value(order, dimension.field_code).to_string(),
        })
        .collect()
}

fn order_value(order: &AggregationOrderRow, field: AggregationFieldCode) -> &str {
    match field {
        AggregationFieldCode::DocumentType => &order.document_type,
        AggregationFieldCode::ErpOrderNo => order.erp_order_no.as_deref().unwrap_or(""),
        AggregationFieldCode::InvoiceNo => order.invoice_no.as_deref().unwrap_or(""),
        AggregationFieldCode::TransportModeCode => {
            order.transport_mode_code.as_deref().unwrap_or("")
        }
        AggregationFieldCode::DepartmentCode => order.department_code.as_deref().unwrap_or(""),
        AggregationFieldCode::SalesGroupCode => order.sales_group_code.as_deref().unwrap_or(""),
        AggregationFieldCode::OrderGroupNo => order.order_group_no.as_deref().unwrap_or(""),
        AggregationFieldCode::BusinessTypeCode => order.business_type_code.as_deref().unwrap_or(""),
    }
}

fn field_display_name(field: AggregationFieldCode) -> &'static str {
    match field {
        AggregationFieldCode::DocumentType => "单据类型",
        AggregationFieldCode::ErpOrderNo => "ERP 订单号",
        AggregationFieldCode::InvoiceNo => "发票号",
        AggregationFieldCode::TransportModeCode => "运输方式",
        AggregationFieldCode::DepartmentCode => "业务部门",
        AggregationFieldCode::SalesGroupCode => "销售组",
        AggregationFieldCode::OrderGroupNo => "订单组号",
        AggregationFieldCode::BusinessTypeCode => "业务类型",
    }
}

async fn lock_rule_versions(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtext('h9-aggregation-rule'), hashtext($1::text))",
    )
    .bind(owner_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn ensure_registered_dimensions(
    tx: &mut Transaction<'_, Postgres>,
    dimensions: &[AggregationDimension],
) -> Result<(), PrintOrchestrationError> {
    let field_codes = dimensions
        .iter()
        .map(|dimension| dimension.field_code.as_str())
        .collect::<Vec<_>>();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM h9_aggregation_field_catalog WHERE enabled AND field_code = ANY($1)",
    )
    .bind(&field_codes)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if count == dimensions.len() as i64 {
        Ok(())
    } else {
        Err(PrintOrchestrationError::InvalidRequest)
    }
}

async fn load_rule_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    version_id: Uuid,
) -> Result<AggregationRuleRow, PrintOrchestrationError> {
    sqlx::query_as::<_, AggregationRuleRow>(
        r#"
        SELECT id, owner_id, version_no, name, status, dimensions,
               tested_at, published_at, disabled_at, created_at
          FROM h9_aggregation_rule_versions
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintOrchestrationError::AggregationRuleNotFound)
}

fn parse_dimensions(value: &Value) -> Result<Vec<AggregationDimension>, PrintOrchestrationError> {
    serde_json::from_value(value.clone()).map_err(serialize_error)
}

fn serialize_error(error: serde_json::Error) -> PrintOrchestrationError {
    PrintOrchestrationError::Serialize(error.to_string())
}

impl TryFrom<AggregationRuleRow> for AggregationRuleVersion {
    type Error = PrintOrchestrationError;

    fn try_from(row: AggregationRuleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            owner_id: row.owner_id,
            version_no: row.version_no,
            name: row.name,
            status: row.status,
            dimensions: parse_dimensions(&row.dimensions)?,
            tested_at: row.tested_at,
            published_at: row.published_at,
            disabled_at: row.disabled_at,
            created_at: row.created_at,
        })
    }
}

async fn append_rule_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    rule: &AggregationRuleVersion,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        "aggregation_rule_version",
        rule.id.to_string(),
        Some(AuditDiff::compute(
            Value::Null,
            serde_json::to_value(rule).map_err(serialize_error)?,
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
    Ok(())
}
