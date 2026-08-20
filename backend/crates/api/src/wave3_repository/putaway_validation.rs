use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;
use wms_domain::{
    is_temperature_zone_subset, validate_category_zone, validate_external_fragrant,
    validate_pack_granularity, validate_quality_match, DualPersonPolicy,
    PutawayLocationValidationRequest, PutawayLocationValidationResponse,
    ResolveDualPersonPolicyQuery, M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT,
    M2_PUTAWAY_PACK_GRANULARITY_INVALID, M2_PUTAWAY_QUALITY_LOCKED,
    M2_PUTAWAY_SPECIAL_DUAL_REQUIRED, M2_PUTAWAY_TEMPERATURE_MISMATCH,
    M2_PUTAWAY_ZONE_CATEGORY_DENIED, PUTAWAY_DIMENSION_CATEGORY_ZONE,
    PUTAWAY_DIMENSION_EXTERNAL_FRAGRANT, PUTAWAY_DIMENSION_PACK_GRANULARITY,
    PUTAWAY_DIMENSION_QUALITY_LOCK, PUTAWAY_DIMENSION_SPECIAL_DUAL,
    PUTAWAY_DIMENSION_TEMPERATURE_ZONE,
};

use super::{map_db_error, PgWave3Repository, Wave3RepositoryError};
use crate::auth::AuthContext;

#[derive(FromRow)]
pub(super) struct LocationZoneValidationRow {
    pub(super) location_id: Uuid,
    pub(super) location_code: String,
    pub(super) location_type: String,
    pub(super) allows_container: bool,
    pub(super) zone_code: String,
    pub(super) temperature_zone: String,
    pub(super) quality_color: String,
    pub(super) allowed_categories: Value,
    pub(super) is_external_use_zone: bool,
    pub(super) is_fragrant_zone: bool,
    pub(super) is_special_drug_zone: bool,
    #[sqlx(default)]
    pub(super) warehouse_id: Option<Uuid>,
}

#[derive(FromRow)]
pub(super) struct ProductValidationRow {
    pub(super) id: Uuid,
    pub(super) product_code: String,
    pub(super) storage_condition: Option<String>,
    pub(super) special_drug_category: Option<String>,
    pub(super) is_external_use: bool,
    pub(super) is_fragrant: bool,
    pub(super) attrs: Value,
    #[sqlx(default)]
    pub(super) volume_cm3: Option<f64>,
}

#[derive(FromRow)]
struct ContainerValidationRow {
    current_lock_category: Option<String>,
}

/// 6 维维度开关：探测端点全开；推荐主流程关闭依赖请求上下文的维度
/// （④ 特药双人见证——推荐阶段无见证人；⑤ 包装粒度——推荐阶段未知容器/散货粒度）。
#[derive(Clone, Copy, Debug)]
pub(super) struct PutawayDimensionScope {
    pub(super) category_zone: bool,
    pub(super) temperature: bool,
    pub(super) quality_lock: bool,
    pub(super) special_dual: bool,
    pub(super) pack_granularity: bool,
    pub(super) external_fragrant: bool,
    pub(super) capacity: bool,
}

impl PutawayDimensionScope {
    /// 探测端点：6 维全开。
    pub(super) fn all() -> Self {
        Self {
            category_zone: true,
            temperature: true,
            quality_lock: true,
            special_dual: true,
            pack_granularity: true,
            external_fragrant: true,
            capacity: true,
        }
    }

    /// 推荐主流程：可判定维度全开，④⑤ 关闭。
    #[allow(dead_code)]
    pub(super) fn recommendation() -> Self {
        Self {
            category_zone: true,
            temperature: true,
            quality_lock: true,
            special_dual: false,
            pack_granularity: false,
            external_fragrant: true,
            capacity: true,
        }
    }
}

/// 6 维⑥ 容量/混品上限共享校验（推荐 SQL 的容量过滤与此合并为同一规则）：
/// 目标位放入后不超 `max_volume_cm3 - used_volume_cm3`，且不超 `max_sku_count` 混品上限
/// （同品已存在时不占混品额度）。
pub(super) async fn location_capacity_allows<'e, E>(
    executor: E,
    owner_id: Uuid,
    location_id: Uuid,
    product_code: &str,
    required_volume_cm3: i64,
) -> Result<bool, Wave3RepositoryError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let allows: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT location.max_volume_cm3 - location.used_volume_cm3 >= $3
           AND (
               EXISTS (
                   SELECT 1 FROM inventory_batches existing
                    WHERE existing.owner_id = $2
                      AND existing.location_id = location.id
                      AND existing.product_code = $4
                      AND existing.qty_on_hand > 0
               )
               OR (SELECT COUNT(DISTINCT sibling.product_code)
                     FROM inventory_batches sibling
                    WHERE sibling.owner_id = $2
                      AND sibling.location_id = location.id
                      AND sibling.qty_on_hand > 0) < location.max_sku_count
           )
          FROM warehouse_locations location
         WHERE location.owner_id = $2 AND location.id = $1
        "#,
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(required_volume_cm3)
    .bind(product_code)
    .fetch_optional(executor)
    .await
    .map_err(map_db_error)?;
    Ok(allows.unwrap_or(false))
}

/// 上架拦截日志的一行参数，收敛 9 参数函数签名。
struct RejectionLog<'a> {
    owner_id: Uuid,
    operated_by: Uuid,
    container_code: Option<&'a str>,
    product_id: Option<Uuid>,
    target_location_id: Uuid,
    rejection_dimension: &'a str,
    error_code: &'a str,
    reason: &'a str,
    occurred_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub async fn validate_putaway_location(
        &self,
        ctx: &AuthContext,
        req: PutawayLocationValidationRequest,
        now: DateTime<Utc>,
    ) -> Result<PutawayLocationValidationResponse, Wave3RepositoryError> {
        // 1. Query target location and its zone
        let loc_zone = sqlx::query_as::<_, LocationZoneValidationRow>(
            r#"
            SELECT
                l.id AS location_id,
                l.location_code,
                l.location_type,
                l.allows_container,
                z.zone_code,
                z.temperature_zone,
                z.quality_color,
                z.allowed_categories,
                z.is_external_use_zone,
                z.is_fragrant_zone,
                z.is_special_drug_zone,
                l.warehouse_id
            FROM warehouse_locations l
            JOIN warehouse_zones z ON z.id = l.zone_id AND z.owner_id = l.owner_id
            WHERE l.owner_id = $1 AND l.id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.target_location_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::InvalidLocation)?;

        // 2. Query product if product_id or product_code is provided
        let product: Option<ProductValidationRow> = if let Some(pid) = req.product_id {
            sqlx::query_as::<_, ProductValidationRow>(
                r#"
                SELECT
                    id, product_code, storage_condition, special_drug_category,
                    is_external_use, is_fragrant, attrs, volume_cm3
                FROM products
                WHERE owner_id = $1 AND id = $2 AND status = 'active'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(pid)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
        } else if let Some(pcode) = req.product_code.as_deref() {
            sqlx::query_as::<_, ProductValidationRow>(
                r#"
                SELECT
                    id, product_code, storage_condition, special_drug_category,
                    is_external_use, is_fragrant, attrs, volume_cm3
                FROM products
                WHERE owner_id = $1 AND product_code = $2 AND status = 'active'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(pcode)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
        } else {
            None
        };

        // 3. Query container if container_code is provided
        let container: Option<ContainerValidationRow> =
            if let Some(code) = req.container_code.as_deref() {
                sqlx::query_as::<_, ContainerValidationRow>(
                    r#"
                SELECT current_lock_category
                FROM lpn_containers
                WHERE owner_id = $1 AND lpn_code = $2
                "#,
                )
                .bind(ctx.owner_id)
                .bind(code)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_db_error)?
            } else {
                None
            };

        let is_container_val = req
            .is_container
            .unwrap_or_else(|| req.container_code.is_some());
        let lock_category = container
            .as_ref()
            .and_then(|c| c.current_lock_category.as_deref());

        // 容量维度：qty 与商品体积齐备时才可计算所需体积，否则跳过容量校验。
        let required_volume_cm3 = match (req.qty, product.as_ref().and_then(|p| p.volume_cm3)) {
            (Some(qty), Some(volume_cm3)) => {
                super::putaway::product_required_volume_cm3(Some(volume_cm3), qty).ok()
            }
            _ => None,
        };

        self.check_putaway_dimensions(
            ctx,
            &loc_zone,
            product.as_ref(),
            lock_category,
            is_container_val,
            req.batch_status.as_deref(),
            req.witness_id,
            required_volume_cm3,
            req.container_code.as_deref(),
            &PutawayDimensionScope::all(),
            now,
        )
        .await?;

        Ok(PutawayLocationValidationResponse {
            valid: true,
            message: "上架 6 维合规校验通过".to_string(),
            location_id: Some(loc_zone.location_id),
            location_code: Some(loc_zone.location_code),
            zone_code: Some(loc_zone.zone_code),
            temperature_zone: Some(loc_zone.temperature_zone),
            quality_color: Some(loc_zone.quality_color),
        })
    }

    /// 单候选位 6 维正交校验（探测端点与推荐主流程共用的引擎）。
    /// 任一启用维度不通过：记录阻断日志并返回对应已登记 M2_PUTAWAY_* 错误。
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn check_putaway_dimensions(
        &self,
        ctx: &AuthContext,
        loc_zone: &LocationZoneValidationRow,
        product: Option<&ProductValidationRow>,
        container_lock_category: Option<&str>,
        is_container: bool,
        batch_status: Option<&str>,
        witness_id: Option<Uuid>,
        required_volume_cm3: Option<i64>,
        container_code: Option<&str>,
        scope: &PutawayDimensionScope,
        now: DateTime<Utc>,
    ) -> Result<(), Wave3RepositoryError> {
        // Extract product attributes
        let product_category = product
            .and_then(|p| {
                p.attrs
                    .get("category")
                    .or_else(|| p.attrs.get("product_category"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("drug");

        let product_storage_condition = product
            .and_then(|p| p.storage_condition.as_deref())
            .unwrap_or("normal_10_30");

        let is_special_drug = product.map_or(false, |p| {
            p.special_drug_category
                .as_deref()
                .map_or(false, |c| c != "none" && c != "normal" && !c.is_empty())
        }) || loc_zone.is_special_drug_zone;

        let prod_is_external = product.map_or(false, |p| p.is_external_use);
        let prod_is_fragrant = product.map_or(false, |p| p.is_fragrant);
        let resolved_product_id = product.map(|p| p.id);

        // ① Category Zone Isolation
        if scope.category_zone
            && !validate_category_zone(&loc_zone.allowed_categories, product_category)
        {
            self.record_rejection(
                ctx,
                container_code,
                resolved_product_id,
                loc_zone,
                PUTAWAY_DIMENSION_CATEGORY_ZONE,
                M2_PUTAWAY_ZONE_CATEGORY_DENIED,
                "商品品类与目标库区准入大区不匹配（6 维①）",
                now,
            )
            .await?;
            return Err(Wave3RepositoryError::PutawayZoneCategoryDenied);
        }

        // ② Temperature Zone Matching
        if scope.temperature
            && !is_temperature_zone_subset(&loc_zone.temperature_zone, product_storage_condition)
        {
            self.record_rejection(
                ctx,
                container_code,
                resolved_product_id,
                loc_zone,
                PUTAWAY_DIMENSION_TEMPERATURE_ZONE,
                M2_PUTAWAY_TEMPERATURE_MISMATCH,
                "目标库位温区不满足商品存储温区要求（6 维②）",
                now,
            )
            .await?;
            return Err(Wave3RepositoryError::PutawayTemperatureMismatch);
        }

        // ③ Container Quality Lock & Quality Color Matching
        if scope.quality_lock {
            let quality_match = if is_container {
                let cat = container_lock_category.unwrap_or("qualified");
                validate_quality_match(&loc_zone.quality_color, cat)
            } else {
                let batch_stat = batch_status.unwrap_or("qualified");
                validate_quality_match(&loc_zone.quality_color, batch_stat)
            };
            if !quality_match {
                self.record_rejection(
                    ctx,
                    container_code,
                    resolved_product_id,
                    loc_zone,
                    PUTAWAY_DIMENSION_QUALITY_LOCK,
                    M2_PUTAWAY_QUALITY_LOCKED,
                    "容器质量锁/批次状态非合格，禁止上架合格位（6 维③）",
                    now,
                )
                .await?;
                return Err(Wave3RepositoryError::PutawayQualityLocked);
            }
        }

        // ④ Special Drug Dual Verification（M-VR 策略；Single 不强制见证人）
        if scope.special_dual && is_special_drug {
            let requires_dual = match (product, loc_zone.warehouse_id) {
                (Some(prod), Some(warehouse_id)) => {
                    let policy = crate::dual_person_policy::PgDualPersonPolicyRepository::new(
                        self.pool.clone(),
                    )
                    .resolve(
                        ctx,
                        &ResolveDualPersonPolicyQuery {
                            product_id: prod.id,
                            process: "入库".to_string(),
                            node: "上架".to_string(),
                            owner_id: ctx.owner_id,
                            warehouse_id: Some(warehouse_id),
                        },
                    )
                    .await;
                    match policy {
                        Ok(value) => !matches!(value.policy, DualPersonPolicy::Single),
                        Err(_) => true,
                    }
                }
                _ => true,
            };
            if !wms_domain::special_dual_passes(true, !requires_dual, ctx.user_id, witness_id) {
                self.record_rejection(
                    ctx,
                    container_code,
                    resolved_product_id,
                    loc_zone,
                    PUTAWAY_DIMENSION_SPECIAL_DUAL,
                    M2_PUTAWAY_SPECIAL_DUAL_REQUIRED,
                    "特药上架需要双人核验（6 维④）",
                    now,
                )
                .await?;
                return Err(Wave3RepositoryError::PutawaySpecialDualRequired);
            }
        }

        // ⑤ Pack Granularity & Location Type
        if scope.pack_granularity
            && !validate_pack_granularity(
                &loc_zone.location_type,
                loc_zone.allows_container,
                is_container,
                container_lock_category,
            )
        {
            self.record_rejection(
                ctx,
                container_code,
                resolved_product_id,
                loc_zone,
                PUTAWAY_DIMENSION_PACK_GRANULARITY,
                M2_PUTAWAY_PACK_GRANULARITY_INVALID,
                "包装粒度与目标位作业形态不符（6 维⑤）",
                now,
            )
            .await?;
            return Err(Wave3RepositoryError::PutawayPackGranularityInvalid);
        }

        // ⑥ External-Use & Fragrant Exclusivity
        if scope.external_fragrant
            && !validate_external_fragrant(
                prod_is_external,
                loc_zone.is_external_use_zone,
                prod_is_fragrant,
                loc_zone.is_fragrant_zone,
            )
        {
            self.record_rejection(
                ctx,
                container_code,
                resolved_product_id,
                loc_zone,
                PUTAWAY_DIMENSION_EXTERNAL_FRAGRANT,
                M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT,
                "外用/易串味商品与目标库区互斥（6 维⑥）",
                now,
            )
            .await?;
            return Err(Wave3RepositoryError::PutawayExternalFragrantConflict);
        }

        // ⑥ 容量/混品上限防呆（与推荐 SQL 容量过滤合并的共享校验）
        if scope.capacity {
            if let (Some(product_row), Some(required_volume)) = (product, required_volume_cm3) {
                let allows = location_capacity_allows(
                    &self.pool,
                    ctx.owner_id,
                    loc_zone.location_id,
                    &product_row.product_code,
                    required_volume,
                )
                .await?;
                if !allows {
                    self.record_rejection(
                        ctx,
                        container_code,
                        resolved_product_id,
                        loc_zone,
                        wms_domain::PUTAWAY_DIMENSION_CAPACITY,
                        wms_domain::M2_PUTAWAY_CAPACITY_EXCEEDED,
                        "目标库位剩余容量或混品上限不足（6 维⑥）",
                        now,
                    )
                    .await?;
                    return Err(Wave3RepositoryError::PutawayCapacityExceeded);
                }
            }
        }

        Ok(())
    }

    /// 6 维拦截共用的日志落库：组装 RejectionLog 后插入。
    #[allow(clippy::too_many_arguments)]
    async fn record_rejection(
        &self,
        ctx: &AuthContext,
        container_code: Option<&str>,
        resolved_product_id: Option<Uuid>,
        loc_zone: &LocationZoneValidationRow,
        rejection_dimension: &str,
        error_code: &str,
        reason: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), Wave3RepositoryError> {
        self.insert_putaway_rejection_log(RejectionLog {
            owner_id: ctx.owner_id,
            operated_by: ctx.user_id,
            container_code,
            product_id: resolved_product_id,
            target_location_id: loc_zone.location_id,
            rejection_dimension,
            error_code,
            reason,
            occurred_at,
        })
        .await
    }

    async fn insert_putaway_rejection_log(
        &self,
        log: RejectionLog<'_>,
    ) -> Result<(), Wave3RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO putaway_validation_rejection_logs (
                id, owner_id, operated_by, container_code, product_id,
                target_location_id, rejection_dimension, error_code, reason, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(log.owner_id)
        .bind(log.operated_by)
        .bind(log.container_code)
        .bind(log.product_id)
        .bind(log.target_location_id)
        .bind(log.rejection_dimension)
        .bind(log.error_code)
        .bind(log.reason)
        .bind(log.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }
}
