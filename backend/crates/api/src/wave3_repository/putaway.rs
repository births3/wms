use super::*;
use serde_json::Value;

#[derive(FromRow)]
struct PutawayProductPolicyRow {
    storage_condition: String,
    attrs: Value,
}

#[derive(FromRow)]
struct PutawayLocationRow {
    location_id: Uuid,
    location_code: String,
    temperature_zone: String,
    quality_color: String,
    available_volume_cm3: i64,
    same_product: bool,
    same_product_distance: Option<i64>,
}

#[derive(FromRow)]
struct PutawayStrategyProfileRow {
    id: Uuid,
    owner_id: Uuid,
    profile_code: String,
    profile_name: String,
    is_default: bool,
    top_n: i32,
    enabled_rules: Value,
    rule_priority: Value,
    warehouse_id: Option<Uuid>,
    product_category: Option<String>,
    notify_on_no_location: bool,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn map_putaway_strategy_profile(row: PutawayStrategyProfileRow) -> PutawayStrategyProfile {
    PutawayStrategyProfile {
        id: row.id,
        owner_id: row.owner_id,
        profile_code: row.profile_code,
        profile_name: row.profile_name,
        is_default: row.is_default,
        top_n: row.top_n,
        enabled_rules: row.enabled_rules,
        rule_priority: row.rule_priority,
        warehouse_id: row.warehouse_id,
        product_category: row.product_category,
        notify_on_no_location: row.notify_on_no_location,
        status: row.status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

const DEFAULT_RULE_PRIORITY: &[&str] = &[
    "temperature_match",
    "owner_isolation",
    "capacity_match",
    "same_product_cluster",
    "abc_class",
    "category_zone",
    "expiry_isolation",
    "empty_location_first",
];

impl PgWave3Repository {
    pub async fn list_putaway_strategy_profiles(
        &self,
        ctx: &AuthContext,
    ) -> Result<PutawayStrategyProfileListResponse, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, PutawayStrategyProfileRow>(
            r#"
            SELECT id, owner_id, profile_code, profile_name, is_default, top_n,
                   enabled_rules, rule_priority, warehouse_id, product_category,
                   notify_on_no_location, status, created_at, updated_at
              FROM putaway_strategy_profiles
             WHERE owner_id = $1
             ORDER BY is_default DESC, profile_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(PutawayStrategyProfileListResponse {
            data: rows.into_iter().map(map_putaway_strategy_profile).collect(),
        })
    }

    pub async fn upsert_putaway_strategy_profile_with_audit(
        &self,
        ctx: &AuthContext,
        req: UpsertPutawayStrategyProfileRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: AuditWriteRequest,
    ) -> Result<IdempotentMutation<PutawayStrategyProfile>, Wave3RepositoryError> {
        let profile_code = req.profile_code.trim();
        let profile_name = req.profile_name.trim();
        if profile_code.is_empty() || profile_name.is_empty() {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        if req.top_n <= 0 || req.top_n > 50 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        if req.status != "active" && req.status != "disabled" {
            return Err(Wave3RepositoryError::InvalidReason);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": &req }))?;
        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        if req.is_default && req.status == "active" {
            sqlx::query(
                "UPDATE putaway_strategy_profiles SET is_default = FALSE, updated_at = $2 WHERE owner_id = $1 AND is_default",
            )
            .bind(ctx.owner_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        if let Some(warehouse_id) = req.warehouse_id {
            ensure_owned_reference(&mut tx, "warehouses", ctx.owner_id, warehouse_id).await?;
        }

        let enabled_rules = req.enabled_rules.clone().unwrap_or_else(|| {
            serde_json::json!({
                "temperature_match": true,
                "owner_isolation": true,
                "capacity_match": true,
                "same_product_cluster": true,
                "quality_color_match": true,
                "abc_class": true,
                "category_zone": true,
                "expiry_isolation": true,
                "empty_location_first": true
            })
        });
        let rule_priority = req
            .rule_priority
            .clone()
            .unwrap_or_else(|| serde_json::json!(DEFAULT_RULE_PRIORITY));
        let product_category = req
            .product_category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, PutawayStrategyProfileRow>(
            r#"
            INSERT INTO putaway_strategy_profiles (
                id, owner_id, profile_code, profile_name, is_default, top_n,
                enabled_rules, rule_priority, warehouse_id, product_category,
                notify_on_no_location, status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
            ON CONFLICT (owner_id, profile_code) DO UPDATE
               SET profile_name = EXCLUDED.profile_name,
                   is_default = EXCLUDED.is_default,
                   top_n = EXCLUDED.top_n,
                   enabled_rules = EXCLUDED.enabled_rules,
                   rule_priority = EXCLUDED.rule_priority,
                   warehouse_id = EXCLUDED.warehouse_id,
                   product_category = EXCLUDED.product_category,
                   notify_on_no_location = EXCLUDED.notify_on_no_location,
                   status = EXCLUDED.status,
                   updated_at = EXCLUDED.updated_at,
                   version = putaway_strategy_profiles.version + 1
            RETURNING id, owner_id, profile_code, profile_name, is_default, top_n,
                      enabled_rules, rule_priority, warehouse_id, product_category,
                      notify_on_no_location, status, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(profile_code)
        .bind(profile_name)
        .bind(req.is_default)
        .bind(req.top_n)
        .bind(enabled_rules)
        .bind(rule_priority)
        .bind(req.warehouse_id)
        .bind(product_category)
        .bind(req.notify_on_no_location)
        .bind(&req.status)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let profile = map_putaway_strategy_profile(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            "/api/v1/inbound/putaway-strategy-profiles",
            "putaway_strategy_profile",
            profile.id.to_string(),
            &profile,
            now,
        )
        .await?;
        let mut audit = audit;
        audit.resource_id = profile.id.to_string();
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: profile,
            replayed: false,
        })
    }

    async fn load_default_putaway_profile(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner_id: Uuid,
        warehouse_id: Option<Uuid>,
        product_category: Option<&str>,
    ) -> Result<Option<PutawayStrategyProfileRow>, Wave3RepositoryError> {
        // 优先精确绑定（仓库+品类）→ 仅仓库 → 仅品类 → 货主通用默认方案。
        sqlx::query_as::<_, PutawayStrategyProfileRow>(
            r#"
            SELECT id, owner_id, profile_code, profile_name, is_default, top_n,
                   enabled_rules, rule_priority, warehouse_id, product_category,
                   notify_on_no_location, status, created_at, updated_at
              FROM putaway_strategy_profiles
             WHERE owner_id = $1
               AND status = 'active'
               AND (warehouse_id IS NULL OR warehouse_id = $2)
               AND (
                    product_category IS NULL
                    OR ($3::text IS NOT NULL AND product_category = $3)
               )
             ORDER BY
               CASE
                 WHEN warehouse_id IS NOT NULL AND product_category IS NOT NULL THEN 0
                 WHEN warehouse_id IS NOT NULL THEN 1
                 WHEN product_category IS NOT NULL THEN 2
                 ELSE 3
               END,
               is_default DESC,
               updated_at DESC
             LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(product_category)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)
    }

    async fn record_no_location_h4_notify(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        receiving_order_id: Uuid,
        product_code: &str,
        batch_no: &str,
        now: DateTime<Utc>,
    ) -> Result<(), Wave3RepositoryError> {
        let dedupe_key = format!(
            "m2-putaway-no-location:{}:{}:{}",
            receiving_order_id, product_code, batch_no
        );
        let content = format!(
            "收货单 {receiving_order_id} 商品 {product_code} 批号 {batch_no} 无可用上架库位，请仓库主管介入。"
        );
        sqlx::query(
            r#"
            INSERT INTO h4_notification_records (
                id, owner_id, config_id, event_type, dedupe_key, recipient, channel,
                content, content_summary, status, retry_count, failure_reason, sent_at,
                created_at, updated_at
            ) VALUES (
                $1, $2, NULL, 'm2.putaway.no_location', $3, 'warehouse_manager', 'wechat',
                $4, $5, 'retrying', 0, 'awaiting_wechat_delivery', NULL, $6, $6
            )
            ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO UPDATE
               SET content = EXCLUDED.content,
                   content_summary = EXCLUDED.content_summary,
                   updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&dedupe_key)
        .bind(&content)
        .bind(&content)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }

    pub async fn recommend_putaway_locations(
        &self,
        ctx: &AuthContext,
        receiving_order_id: Uuid,
        query: PutawayRecommendationQuery,
    ) -> Result<PutawayRecommendationResponse, Wave3RepositoryError> {
        if query.qty <= 0 || query.limit == Some(0) {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let order = lock_receiving_order(&mut tx, ctx.owner_id, receiving_order_id).await?;
        if order.status != "putaway" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "putaway".to_string(),
                actual: order.status,
            });
        }
        let product_category: Option<String> = sqlx::query_scalar(
            r#"
            SELECT NULLIF(TRIM(attrs ->> 'category'), '')
              FROM products
             WHERE owner_id = $1 AND product_code = $2 AND status = 'active'
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&query.product_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .flatten();
        let profile = self
            .load_default_putaway_profile(
                &mut tx,
                ctx.owner_id,
                Some(order.warehouse_id),
                product_category.as_deref(),
            )
            .await?;
        let default_top_n = profile
            .as_ref()
            .map(|row| u32::try_from(row.top_n).unwrap_or(3))
            .unwrap_or(5);
        let same_product_enabled = profile
            .as_ref()
            .and_then(|row| row.enabled_rules.get("same_product_cluster"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let empty_location_first = profile
            .as_ref()
            .and_then(|row| row.enabled_rules.get("empty_location_first"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let notify_on_no_location = profile
            .as_ref()
            .map(|row| row.notify_on_no_location)
            .unwrap_or(true);
        let limit = query.limit.unwrap_or(default_top_n).min(50);

        let valid_line: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2 AND product_code = $3 AND batch_no = $4)",
        )
        .bind(receiving_order_id)
        .bind(ctx.owner_id)
        .bind(&query.product_code)
        .bind(&query.batch_no)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !valid_line {
            return Err(Wave3RepositoryError::NotFound);
        }

        let (accepted_qty, putaway_qty): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE((
                    SELECT SUM(accepted_qty)
                      FROM receiving_inspections
                     WHERE receiving_order_id = $1
                       AND owner_id = $2
                       AND batch_no = $3
                ), 0)::BIGINT,
                COALESCE((
                    SELECT SUM(qty)
                      FROM receiving_putaways
                     WHERE receiving_order_id = $1
                       AND owner_id = $2
                       AND product_code = $4
                       AND batch_no = $3
                ), 0)::BIGINT
            "#,
        )
        .bind(receiving_order_id)
        .bind(ctx.owner_id)
        .bind(&query.batch_no)
        .bind(&query.product_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if accepted_qty <= 0 {
            return Err(Wave3RepositoryError::NotFound);
        }
        let remaining_qty = accepted_qty
            .checked_sub(putaway_qty)
            .ok_or(Wave3RepositoryError::QuantityClosureMismatch)?;
        if query.qty > remaining_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let product = sqlx::query_as::<_, PutawayProductPolicyRow>(
            "SELECT storage_condition, attrs FROM products WHERE owner_id = $1 AND product_code = $2 AND status = 'active'",
        )
        .bind(ctx.owner_id)
        .bind(&query.product_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let unit_volume_cm3 = product_unit_volume_cm3(&product.attrs)?;
        let required_volume_cm3 = unit_volume_cm3
            .checked_mul(query.qty)
            .ok_or(Wave3RepositoryError::InvalidQuantity)?;
        let quality_color =
            resolve_quality_color(&mut tx, ctx.owner_id, &query.quality_status, Utc::now()).await?;

        let locations = sqlx::query_as::<_, PutawayLocationRow>(
            r#"
            SELECT
                location.id AS location_id,
                location.location_code,
                zone.temperature_zone,
                zone.quality_color,
                location.max_volume_cm3 - location.used_volume_cm3 AS available_volume_cm3,
                COUNT(inventory.id) FILTER (
                    WHERE inventory.product_code = $5
                ) > 0 AS same_product,
                MIN(
                    CASE WHEN inventory.product_code = $5 THEN
                        ABS((location.row_no - same_product_location.row_no)::BIGINT)
                        + ABS((location.column_no - same_product_location.column_no)::BIGINT)
                        + ABS((location.layer_no - same_product_location.layer_no)::BIGINT)
                    END
                ) AS same_product_distance
              FROM warehouse_locations AS location
              JOIN warehouse_zones AS zone
                ON zone.id = location.zone_id
               AND zone.owner_id = location.owner_id
             LEFT JOIN inventory_batches AS inventory
                ON inventory.owner_id = location.owner_id
               AND inventory.location_id = location.id
             LEFT JOIN warehouse_locations AS same_product_location
                ON same_product_location.owner_id = inventory.owner_id
               AND same_product_location.id = inventory.location_id
               AND same_product_location.warehouse_id = location.warehouse_id
             WHERE location.owner_id = $1
               AND location.warehouse_id = $2
               AND (location.bound_owner_id IS NULL OR location.bound_owner_id = $1)
               AND location.status IN ('available', 'occupied')
               AND zone.status = 'active'
               AND zone.temperature_zone = $3
               AND zone.quality_color = $4
             GROUP BY location.id, location.location_code, location.max_volume_cm3,
                      location.used_volume_cm3, location.max_sku_count,
                      zone.temperature_zone, zone.quality_color,
                      location.row_no, location.column_no, location.layer_no
             HAVING location.max_volume_cm3 - location.used_volume_cm3 >= $6
                AND (
                    COUNT(inventory.id) FILTER (WHERE inventory.product_code = $5) > 0
                    OR COUNT(DISTINCT inventory.product_code) < location.max_sku_count
                )
             ORDER BY same_product DESC, same_product_distance NULLS LAST,
                      available_volume_cm3, location.location_code
             LIMIT $7
            "#,
        )
        .bind(ctx.owner_id)
        .bind(order.warehouse_id)
        .bind(&product.storage_condition)
        .bind(&quality_color)
        .bind(&query.product_code)
        .bind(required_volume_cm3)
        .bind(i64::from(limit))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if locations.is_empty() {
            if notify_on_no_location {
                self.record_no_location_h4_notify(
                    &mut tx,
                    ctx,
                    receiving_order_id,
                    &query.product_code,
                    &query.batch_no,
                    Utc::now(),
                )
                .await?;
                tx.commit().await.map_err(map_db_error)?;
            }
            return Err(Wave3RepositoryError::NoAvailableLocation);
        }

        let mut locations = locations;
        let priority_keys = profile
            .as_ref()
            .and_then(|row| row.rule_priority.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        // rule_priority 驱动排序；空则回退 enabled_rules 布尔开关。
        locations.sort_by(|left, right| {
            apply_putaway_rule_priority(
                left,
                right,
                &priority_keys,
                same_product_enabled,
                empty_location_first,
            )
        });

        let data = locations
            .into_iter()
            .map(|location| PutawayLocationRecommendation {
                location_id: location.location_id,
                location_code: location.location_code,
                temperature_zone: location.temperature_zone,
                quality_color: location.quality_color,
                available_volume_cm3: location.available_volume_cm3,
                required_volume_cm3,
                same_product: location.same_product,
            })
            .collect();
        tx.commit().await.map_err(map_db_error)?;
        Ok(PutawayRecommendationResponse {
            receiving_order_id,
            owner_id: ctx.owner_id,
            product_code: query.product_code,
            batch_no: query.batch_no,
            qty: query.qty,
            quality_status: query.quality_status,
            data,
        })
    }
}

pub(super) fn product_unit_volume_cm3(attrs: &Value) -> Result<i64, Wave3RepositoryError> {
    let value = attrs
        .get("unit_volume_cm3")
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|value| *value > 0)
        .ok_or(Wave3RepositoryError::InvalidProductVolume)?;
    Ok(value)
}

/// 按策略 `rule_priority` 比较库位；未配置时按 enabled_rules 回退。
fn apply_putaway_rule_priority(
    left: &PutawayLocationRow,
    right: &PutawayLocationRow,
    priority_keys: &[String],
    same_product_enabled: bool,
    empty_location_first: bool,
) -> std::cmp::Ordering {
    let keys: Vec<&str> = if priority_keys.is_empty() {
        let mut defaults = Vec::new();
        if empty_location_first {
            defaults.push("empty_location_first");
        }
        if same_product_enabled {
            defaults.push("same_product_cluster");
        }
        defaults.push("available_volume");
        defaults
    } else {
        priority_keys.iter().map(String::as_str).collect()
    };
    for key in keys {
        let ordering = match key {
            "empty_location_first" => {
                // 空库位（非同品）优先
                (!left.same_product).cmp(&(!right.same_product)).reverse()
            }
            "same_product_cluster" if same_product_enabled || !priority_keys.is_empty() => {
                right.same_product.cmp(&left.same_product).then_with(|| {
                    match (left.same_product_distance, right.same_product_distance) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                })
            }
            "available_volume" | "capacity_match" => {
                left.available_volume_cm3.cmp(&right.available_volume_cm3)
            }
            // temperature_match / quality_color_match / owner_isolation 已在 SQL WHERE 过滤
            _ => std::cmp::Ordering::Equal,
        };
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left.location_code.cmp(&right.location_code)
}
