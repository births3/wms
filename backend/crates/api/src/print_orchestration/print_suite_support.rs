async fn load_category_names(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<BTreeMap<String, (String, String)>, PrintOrchestrationError> {
    let rows: Vec<(String, String, Value)> = sqlx::query_as(
        r#"
        SELECT item_code, item_name, params
          FROM system_dictionary_items
         WHERE dict_code = 'print_document_category'
           AND enabled
           AND (owner_id IS NULL OR owner_id = $1)
         ORDER BY owner_id NULLS FIRST
        "#,
    )
    .bind(owner_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .filter_map(|(item_code, item_name, params)| {
            let source_mode = params.get("source_mode")?.as_str()?.to_string();
            Some((item_code, (item_name, source_mode)))
        })
        .collect())
}

async fn ensure_suite_scope_exists(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreatePrintSuiteDraftRequest,
) -> Result<(), PrintOrchestrationError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2
        )
        AND (
            $3::uuid IS NULL
            OR EXISTS (SELECT 1 FROM customers WHERE owner_id = $1 AND id = $3)
        )
        AND (
            $4::uuid IS NULL
            OR EXISTS (
                SELECT 1 FROM customer_addresses
                 WHERE owner_id = $1 AND id = $4 AND customer_id = $3
            )
        )
        "#,
    )
    .bind(owner_id)
    .bind(request.warehouse_id)
    .bind(request.customer_id)
    .bind(request.delivery_address_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if valid {
        Ok(())
    } else {
        Err(PrintOrchestrationError::InvalidRequest)
    }
}

async fn ensure_registered_categories(
    _tx: &mut Transaction<'_, Postgres>,
    _owner_id: Uuid,
    request: &CreatePrintSuiteDraftRequest,
    categories: &BTreeMap<String, (String, String)>,
) -> Result<(), PrintOrchestrationError> {
    for item in &request.items {
        let Some((_, source_mode)) = categories.get(item.category_code.trim()) else {
            return Err(PrintOrchestrationError::PrintSuiteCategoryInvalid);
        };
        if source_mode != item.source_mode.as_str() {
            return Err(PrintOrchestrationError::PrintSuiteCategoryInvalid);
        }
    }
    Ok(())
}

async fn ensure_rendered_template_bindings(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &CreatePrintSuiteDraftRequest,
) -> Result<(), PrintOrchestrationError> {
    for item in &request.items {
        let Some(template_version_id) = item.template_version_id else {
            continue;
        };
        let valid: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM print_template_versions version
                  JOIN print_templates template ON template.id = version.template_id
                 WHERE version.id = $1
                   AND version.status = 'published'
                   AND template.enabled
                   AND template.owner_id IN (
                       $2,
                       '00000000-0000-0000-0000-000000000000'::uuid
                   )
                   AND template.template_type_code = $3
                   AND version.template_type_code = $3
            )
            "#,
        )
        .bind(template_version_id)
        .bind(owner_id)
        .bind(item.category_code.trim())
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if !valid {
            return Err(PrintOrchestrationError::PrintSuiteBindingInvalid);
        }
    }
    Ok(())
}

async fn lock_suite_versions(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('h9-print-suite'), hashtext($1::text))")
        .bind(owner_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn lock_suite_scope(
    tx: &mut Transaction<'_, Postgres>,
    suite: &SuiteVersionRow,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('h9-print-suite-scope'), hashtext($1))")
        .bind(format!(
            "{}:{}:{}:{}",
            suite.owner_id,
            suite.warehouse_id,
            suite.scope_type,
            suite
                .delivery_address_id
                .map(|id| id.to_string())
                .or_else(|| suite.customer_id.map(|id| id.to_string()))
                .or_else(|| suite.route_code.clone())
                .unwrap_or_else(|| "default".to_string())
        ))
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn suite_period_overlaps(
    tx: &mut Transaction<'_, Postgres>,
    suite: &SuiteVersionRow,
) -> Result<bool, PrintOrchestrationError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM h9_print_suite_versions
             WHERE owner_id = $1
               AND warehouse_id = $2
               AND scope_type = $3
               AND customer_id IS NOT DISTINCT FROM $4
               AND delivery_address_id IS NOT DISTINCT FROM $5
               AND route_code IS NOT DISTINCT FROM $6
               AND status = 'published'
               AND id <> $7
               AND effective_from < COALESCE($9, 'infinity'::timestamptz)
               AND COALESCE(effective_to, 'infinity'::timestamptz) > $8
        )
        "#,
    )
    .bind(suite.owner_id)
    .bind(suite.warehouse_id)
    .bind(&suite.scope_type)
    .bind(suite.customer_id)
    .bind(suite.delivery_address_id)
    .bind(&suite.route_code)
    .bind(suite.id)
    .bind(suite.effective_from)
    .bind(suite.effective_to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn load_suite_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    version_id: Uuid,
) -> Result<SuiteVersionRow, PrintOrchestrationError> {
    sqlx::query_as::<_, SuiteVersionRow>(
        r#"
        SELECT id, owner_id, version_no, name, status, warehouse_id, scope_type,
               customer_id, delivery_address_id, route_code, effective_from,
               effective_to, tested_at, published_at, disabled_at, created_at
          FROM h9_print_suite_versions
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintOrchestrationError::PrintSuiteNotFound)
}

async fn load_suite_items(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    suite_version_id: Uuid,
    categories: &BTreeMap<String, (String, String)>,
) -> Result<Vec<PrintSuiteItem>, PrintOrchestrationError> {
    let rows = sqlx::query_as::<_, SuiteItemRow>(
        r#"
        SELECT id, category_code, copies, sort_order, output_slot,
               required, ready_policy, failure_policy, source_mode,
               template_version_id, external_file_ref
          FROM h9_print_suite_items
         WHERE owner_id = $1 AND suite_version_id = $2
         ORDER BY sort_order
        "#,
    )
    .bind(owner_id)
    .bind(suite_version_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    rows.into_iter()
        .map(|row| {
            let category_name = categories
                .get(&row.category_code)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| row.category_code.clone());
            Ok(PrintSuiteItem {
                id: row.id,
                category_code: row.category_code,
                category_name,
                copies: row.copies,
                sort_order: row.sort_order,
                output_slot: row.output_slot,
                required: row.required,
                ready_policy: parse_enum(&row.ready_policy)?,
                failure_policy: parse_enum(&row.failure_policy)?,
                source_mode: parse_enum(&row.source_mode)?,
                template_version_id: row.template_version_id,
                external_file_ref: row.external_file_ref,
            })
        })
        .collect()
}

fn parse_enum<'a, T: TryFrom<&'a str, Error = ()>>(
    value: &'a str,
) -> Result<T, PrintOrchestrationError> {
    T::try_from(value)
        .map_err(|()| PrintOrchestrationError::Serialize(format!("unknown code: {value}")))
}

fn map_suite_version(
    row: SuiteVersionRow,
    items: Vec<PrintSuiteItem>,
) -> Result<PrintSuiteVersion, PrintOrchestrationError> {
    Ok(PrintSuiteVersion {
        id: row.id,
        owner_id: row.owner_id,
        version_no: row.version_no,
        name: row.name,
        status: row.status,
        warehouse_id: row.warehouse_id,
        scope: parse_enum(&row.scope_type)?,
        customer_id: row.customer_id,
        delivery_address_id: row.delivery_address_id,
        route_code: row.route_code,
        effective_from: row.effective_from,
        effective_to: row.effective_to,
        items,
        tested_at: row.tested_at,
        published_at: row.published_at,
        disabled_at: row.disabled_at,
        created_at: row.created_at,
    })
}

async fn load_group_boundary(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    group_id: Uuid,
) -> Result<GroupBoundaryRow, PrintOrchestrationError> {
    sqlx::query_as::<_, GroupBoundaryRow>(
        r#"
        SELECT warehouse_id, customer_id, delivery_address_id, route_code,
               delivery_note_no, cutoff_at
          FROM h9_delivery_note_groups
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(group_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintOrchestrationError::DeliveryNoteGroupNotFound)
}

async fn load_group_orders(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    group_id: Uuid,
) -> Result<Vec<GroupOrderRow>, PrintOrchestrationError> {
    sqlx::query_as::<_, GroupOrderRow>(
        r#"
        SELECT order_row.id, order_row.wms_order_no, order_row.erp_order_no,
               order_row.invoice_no
          FROM h9_delivery_note_group_orders grouped
          JOIN outbound_orders order_row
            ON order_row.owner_id = grouped.owner_id
           AND order_row.id = grouped.outbound_order_id
         WHERE grouped.owner_id = $1 AND grouped.group_id = $2
         ORDER BY order_row.wms_order_no, order_row.id
        "#,
    )
    .bind(owner_id)
    .bind(group_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

fn suite_matches_boundary(suite: &PrintSuiteVersion, boundary: &GroupBoundaryRow) -> bool {
    if suite.warehouse_id != boundary.warehouse_id {
        return false;
    }
    if suite.effective_from > boundary.cutoff_at
        || suite
            .effective_to
            .is_some_and(|effective_to| effective_to <= boundary.cutoff_at)
    {
        return false;
    }
    match suite.scope {
        PrintSuiteScope::DeliveryAddress => {
            suite.customer_id == Some(boundary.customer_id)
                && suite.delivery_address_id == Some(boundary.delivery_address_id)
        }
        PrintSuiteScope::Customer => suite.customer_id == Some(boundary.customer_id),
        PrintSuiteScope::Route => suite.route_code.as_deref() == Some(boundary.route_code.as_str()),
        PrintSuiteScope::WarehouseDefault => true,
    }
}

fn scope_rank(scope: PrintSuiteScope) -> u8 {
    match scope {
        PrintSuiteScope::DeliveryAddress => 4,
        PrintSuiteScope::Customer => 3,
        PrintSuiteScope::Route => 2,
        PrintSuiteScope::WarehouseDefault => 1,
    }
}

async fn resolve_scope_with_candidate(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    boundary: &GroupBoundaryRow,
    candidate_scope: Option<PrintSuiteScope>,
) -> Result<Option<PrintSuiteScope>, PrintOrchestrationError> {
    let published = resolve_published_suite(tx, owner_id, boundary).await?;
    let published_scope = published
        .map(|row| parse_enum::<PrintSuiteScope>(&row.scope_type))
        .transpose()?;
    Ok(match (published_scope, candidate_scope) {
        (Some(published), Some(candidate)) => {
            Some(if scope_rank(candidate) > scope_rank(published) {
                candidate
            } else {
                published
            })
        }
        (Some(published), None) => Some(published),
        (None, candidate) => candidate,
    })
}

async fn resolve_published_suite(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    boundary: &GroupBoundaryRow,
) -> Result<Option<SuiteVersionRow>, PrintOrchestrationError> {
    sqlx::query_as::<_, SuiteVersionRow>(
        r#"
        SELECT id, owner_id, version_no, name, status, warehouse_id, scope_type,
               customer_id, delivery_address_id, route_code, effective_from,
               effective_to, tested_at, published_at, disabled_at, created_at
          FROM h9_print_suite_versions
         WHERE owner_id = $1
           AND warehouse_id = $2
           AND status = 'published'
           AND effective_from <= $6
           AND (effective_to IS NULL OR effective_to > $6)
           AND (
                (scope_type = 'delivery_address'
                    AND customer_id = $3 AND delivery_address_id = $4)
                OR (scope_type = 'customer' AND customer_id = $3)
                OR (scope_type = 'route' AND route_code = $5)
                OR scope_type = 'warehouse_default'
           )
         ORDER BY CASE scope_type
                      WHEN 'delivery_address' THEN 4
                      WHEN 'customer' THEN 3
                      WHEN 'route' THEN 2
                      ELSE 1
                  END DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(boundary.warehouse_id)
    .bind(boundary.customer_id)
    .bind(boundary.delivery_address_id)
    .bind(&boundary.route_code)
    .bind(boundary.cutoff_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// US-H9-008 AC5: computable readiness check for one print item over the
/// real source orders of a delivery-note group.
async fn compute_item_readiness(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    item: &PrintSuiteItem,
    orders: &[GroupOrderRow],
) -> Result<PrintSuiteItemReadiness, PrintOrchestrationError> {
    let mut missing = Vec::new();
    let mut file_bindings = Vec::new();
    match item.source_mode {
        PrintSuiteSourceMode::Rendered => {}
        PrintSuiteSourceMode::ExternalFile => match item.category_code.as_str() {
            "invoice" => {
                let mut invoice_nos = BTreeSet::new();
                for order in orders {
                    match order.invoice_no.as_deref().map(str::trim) {
                        Some(invoice_no) if !invoice_no.is_empty() => {
                            invoice_nos.insert(invoice_no.to_string());
                        }
                        _ => missing.push(format!("订单 {} 未登记发票号", order.wms_order_no)),
                    }
                }
                let invoice_list = invoice_nos.iter().cloned().collect::<Vec<_>>();
                let files = sqlx::query_as::<_, IngestedFileRow>(
                    r#"
                    SELECT DISTINCT ON (invoice_no)
                           id, file_ref, file_version, content_hash,
                           invoice_no, product_code, batch_no
                      FROM h9_ingested_document_files
                     WHERE owner_id = $1
                       AND category_code = 'invoice'
                       AND status = 'valid'
                       AND invoice_no = ANY($2)
                     ORDER BY invoice_no, file_version DESC
                    "#,
                )
                .bind(owner_id)
                .bind(&invoice_list)
                .fetch_all(&mut **tx)
                .await
                .map_err(map_db_error)?;
                let covered = files
                    .iter()
                    .filter_map(|file| file.invoice_no.clone())
                    .collect::<BTreeSet<_>>();
                for invoice_no in &invoice_nos {
                    if !covered.contains(invoice_no) {
                        missing.push(format!("发票 {invoice_no} 未摄取有效文件"));
                    }
                }
                file_bindings = files.into_iter().map(map_file_binding).collect();
            }
            "drug_inspection_report" => {
                let pairs: Vec<(String, String)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT product_code, batch_no
                      FROM outbound_order_lines
                     WHERE owner_id = $1 AND outbound_order_id = ANY($2)
                     ORDER BY product_code, batch_no
                    "#,
                )
                .bind(owner_id)
                .bind(orders.iter().map(|order| order.id).collect::<Vec<_>>())
                .fetch_all(&mut **tx)
                .await
                .map_err(map_db_error)?;
                let keys = pairs
                    .iter()
                    .map(|(product, batch)| format!("{product}||{batch}"))
                    .collect::<Vec<_>>();
                let files = sqlx::query_as::<_, IngestedFileRow>(
                    r#"
                    SELECT DISTINCT ON (product_code, batch_no)
                           id, file_ref, file_version, content_hash,
                           invoice_no, product_code, batch_no
                      FROM h9_ingested_document_files
                     WHERE owner_id = $1
                       AND category_code = 'drug_inspection_report'
                       AND status = 'valid'
                       AND product_code || '||' || batch_no = ANY($2)
                     ORDER BY product_code, batch_no, file_version DESC
                    "#,
                )
                .bind(owner_id)
                .bind(&keys)
                .fetch_all(&mut **tx)
                .await
                .map_err(map_db_error)?;
                let covered = files
                    .iter()
                    .filter_map(|file| {
                        Some(format!(
                            "{}||{}",
                            file.product_code.as_deref()?,
                            file.batch_no.as_deref()?
                        ))
                    })
                    .collect::<BTreeSet<_>>();
                for (product, batch) in &pairs {
                    if !covered.contains(&format!("{product}||{batch}")) {
                        missing.push(format!("商品 {product} 批号 {batch} 缺少有效药检报告"));
                    }
                }
                file_bindings = files.into_iter().map(map_file_binding).collect();
            }
            other => {
                missing.push(format!("分类 {other} 尚无已实现的摄取来源"));
            }
        },
    }
    Ok(PrintSuiteItemReadiness {
        category_code: item.category_code.clone(),
        category_name: item.category_name.clone(),
        source_mode: item.source_mode,
        required: item.required,
        ready: missing.is_empty(),
        missing,
        file_bindings,
    })
}

fn map_file_binding(file: IngestedFileRow) -> PrintSuiteFileBinding {
    PrintSuiteFileBinding {
        file_id: file.id,
        file_ref: file.file_ref,
        file_version: file.file_version,
        content_hash: file.content_hash,
    }
}

async fn load_instance_items(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    instance_id: Uuid,
) -> Result<Vec<PrintSuiteInstanceItem>, PrintOrchestrationError> {
    let rows = sqlx::query_as::<_, InstanceItemRow>(
        r#"
        SELECT id, category_code, copies, sort_order, output_slot,
               required, ready_policy, failure_policy, source_mode,
               template_version_id, external_file_ref, file_bindings, ready, missing
          FROM h9_print_suite_instance_items
         WHERE owner_id = $1 AND instance_id = $2
         ORDER BY sort_order, id
        "#,
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    rows.into_iter()
        .map(|row| {
            Ok(PrintSuiteInstanceItem {
                id: row.id,
                category_code: row.category_code,
                copies: row.copies,
                sort_order: row.sort_order,
                output_slot: row.output_slot,
                required: row.required,
                ready_policy: parse_enum(&row.ready_policy)?,
                failure_policy: parse_enum(&row.failure_policy)?,
                source_mode: parse_enum(&row.source_mode)?,
                template_version_id: row.template_version_id,
                external_file_ref: row.external_file_ref,
                file_bindings: serde_json::from_value(row.file_bindings)
                    .map_err(serialize_error)?,
                ready: row.ready,
                missing: serde_json::from_value(row.missing).map_err(serialize_error)?,
            })
        })
        .collect()
}

fn map_instance(
    row: InstanceRow,
    items: Vec<PrintSuiteInstanceItem>,
) -> Result<PrintSuiteInstance, PrintOrchestrationError> {
    Ok(PrintSuiteInstance {
        id: row.id,
        owner_id: row.owner_id,
        group_id: row.group_id,
        delivery_note_no: row.delivery_note_no,
        suite_version_id: row.suite_version_id,
        suite_version_no: row.suite_version_no,
        suite_snapshot: row.suite_snapshot,
        aggregation_rule_version_id: row.aggregation_rule_version_id,
        aggregation_rule_version_no: row.aggregation_rule_version_no,
        source_documents: row.source_documents,
        status: row.status,
        hold_scope: row.hold_scope,
        items,
        created_at: row.created_at,
    })
}

fn serialize_error(error: serde_json::Error) -> PrintOrchestrationError {
    PrintOrchestrationError::Serialize(error.to_string())
}

async fn append_suite_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    suite: &PrintSuiteVersion,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        "print_suite_version",
        suite.id.to_string(),
        Some(AuditDiff::compute(
            Value::Null,
            serde_json::to_value(suite).map_err(serialize_error)?,
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
    Ok(())
}
