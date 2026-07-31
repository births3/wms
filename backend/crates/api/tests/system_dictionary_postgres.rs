use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    master_data_postgres::PgMasterDataReadRepository,
    system_dictionary::{PgSystemDictionaryRepository, SystemDictionaryError},
};
use wms_domain::{
    DisableSystemDictionaryItemRequest, UpsertSystemDictionaryItemRequest,
    DOCUMENT_TYPE_PURCHASE_INBOUND, DOCUMENT_TYPE_SALES_OUTBOUND,
    PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD, PRINT_TEMPLATE_TYPE_ASN,
    PRINT_TEMPLATE_TYPE_DELIVERY_NOTE, PRINT_TEMPLATE_TYPE_LOCATION_LABEL,
    PRINT_TEMPLATE_TYPE_LPN_LABEL, PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
    SYSTEM_DICTIONARY_DOCUMENT_TYPE, SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "system-dictionary-test".to_string(),
        permissions: vec!["m1.system_dictionary.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn valid_document_type_params() -> serde_json::Value {
    json!({
        "direction": "inbound",
        "workflow_template": "purchase_inbound",
        "batch_policy": "standard_batch"
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_presets_are_queryable(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(&ctx(owner_id), SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("document_type presets should be queryable");
    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();

    assert_eq!(
        codes,
        vec![
            "purchase_inbound",
            "purchase_return_outbound",
            "quality_liaison",
            "sales_outbound",
            "sales_return",
            "stock_loss",
            "stock_surplus"
        ]
    );
    assert!(items.iter().all(|item| item.source == "global"));
    assert!(items
        .iter()
        .all(|item| item.params.get("direction").is_some()));
    assert!(items
        .iter()
        .all(|item| item.params.get("workflow_template").is_some()));
    assert!(items
        .iter()
        .all(|item| item.params.get("batch_policy").is_some()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn special_drug_category_presets_expose_compliance_defaults(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 12, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(&ctx(owner_id), "special_drug_category", now)
        .await
        .expect("special drug category defaults should be queryable");

    assert_eq!(items.len(), 8);
    assert!(items.iter().all(|item| {
        item.params
            .get("requires_dual_person_matrix")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|matrix| matrix.len() == 12)
            && item.params.get("requires_dedicated_ledger").is_some()
            && item.params.get("requires_dedicated_storage").is_some()
            && item.params.get("requires_qualification").is_some()
            && item
                .params
                .get("regulation_basis")
                .and_then(serde_json::Value::as_str)
                .is_some()
    }));
    let narcotic = items
        .iter()
        .find(|item| item.item_code == "narcotic")
        .expect("narcotic preset should exist");
    assert_eq!(narcotic.params["requires_dedicated_ledger"], json!(true));
    assert_eq!(narcotic.params["requires_dedicated_storage"], json!(true));
    assert!(narcotic.params["requires_dual_person_matrix"]
        .as_array()
        .expect("narcotic matrix")
        .iter()
        .all(|entry| entry["policy"] == "dual_scan_with_approval"));
    let ordinary = items
        .iter()
        .find(|item| item.item_code == "none")
        .expect("ordinary category preset should exist");
    assert_eq!(ordinary.params["requires_dedicated_ledger"], json!(false));
    assert_eq!(ordinary.params["regulation_basis"], json!(""));
}

#[sqlx::test(migrations = "../../migrations")]
async fn print_template_type_presets_are_queryable_and_require_field_library(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 6, 9, 0, 0)
        .single()
        .expect("valid time");

    let items = repo
        .list_effective_items(&ctx(owner_id), SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE, now)
        .await
        .expect("print_template_type presets should be queryable");
    let codes: Vec<_> = items.iter().map(|item| item.item_code.as_str()).collect();

    assert_eq!(
        codes,
        vec![
            PRINT_TEMPLATE_TYPE_ASN,
            PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD,
            PRINT_TEMPLATE_TYPE_DELIVERY_NOTE,
            PRINT_TEMPLATE_TYPE_LOCATION_LABEL,
            PRINT_TEMPLATE_TYPE_LPN_LABEL,
            PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
        ]
    );
    assert_eq!(
        items.iter().map(|item| item.sort_order).collect::<Vec<_>>(),
        vec![10, 20, 30, 40, 50, 60]
    );
    assert!(items.iter().all(|item| item.source == "global"));
    assert!(items
        .iter()
        .all(|item| item.params.get("field_library_code").is_some()));

    let error = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_ASN,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "坏模板类型".to_string(),
                enabled: true,
                sort_order: 10,
                params: json!({
                    "business_module": "M2",
                    "business_direction": "inbound",
                    "paper_type": "a4",
                    "default_scope": "global"
                }),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-print-template-invalid",
        )
        .await
        .expect_err("enabled print template type without field library must be rejected");
    assert_eq!(
        error,
        SystemDictionaryError::PrintTemplateFieldLibraryRequired
    );

    let blank_error = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_ASN,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "空字段库模板类型".to_string(),
                enabled: true,
                sort_order: 10,
                params: json!({
                    "field_library_code": "  ",
                    "business_module": "M2",
                    "business_direction": "inbound",
                    "paper_type": "a4",
                    "default_scope": "owner"
                }),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-print-template-blank-library",
        )
        .await
        .expect_err("blank field library must be rejected");
    assert_eq!(
        blank_error,
        SystemDictionaryError::PrintTemplateFieldLibraryRequired
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn print_template_type_create_update_disable_are_idempotent_and_audited(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 6, 10, 0, 0)
        .single()
        .expect("valid time");
    let request = UpsertSystemDictionaryItemRequest {
        owner_id: Some(owner_id),
        item_name: "货主商品标签".to_string(),
        enabled: true,
        sort_order: 5,
        params: json!({
            "field_library_code": "m1_product_label_owner",
            "business_module": "M1",
            "business_direction": "label",
            "paper_type": "a4",
            "default_scope": "owner"
        }),
        effective_from: None,
        effective_to: None,
    };
    let created = repo
        .upsert_item(
            &auth,
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
            request.clone(),
            now,
            "h9-template-type-create",
        )
        .await
        .expect("owner template type should create");
    let replay = repo
        .upsert_item(
            &auth,
            SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
            PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
            request.clone(),
            now,
            "h9-template-type-create",
        )
        .await
        .expect("same create should replay");
    assert_eq!(replay.value.id, created.value.id);
    assert!(replay.replayed);

    let mut updated_request = request;
    updated_request.item_name = "货主商品标签已更新".to_string();
    repo.upsert_item(
        &auth,
        SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
        PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
        updated_request,
        now + chrono::Duration::minutes(1),
        "h9-template-type-update",
    )
    .await
    .expect("owner template type should update");
    repo.disable_item(
        &auth,
        SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE,
        PRINT_TEMPLATE_TYPE_PRODUCT_LABEL,
        DisableSystemDictionaryItemRequest {
            owner_id: Some(owner_id),
            disabled_reason: Some("test disable".to_string()),
        },
        now + chrono::Duration::minutes(2),
        "h9-template-type-disable",
    )
    .await
    .expect("owner template type should disable");

    let audits: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT action, diff FROM audit_event WHERE resource_id = $1 ORDER BY occurred_at",
    )
    .bind(created.value.id.to_string())
    .fetch_all(&pool)
    .await
    .expect("template type audits should query");
    assert_eq!(
        audits
            .iter()
            .map(|(action, _)| action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "upsert_system_dictionary_item",
            "upsert_system_dictionary_item",
            "disable_system_dictionary_item"
        ]
    );
    assert_eq!(audits[0].1["before"], serde_json::Value::Null);
    assert_eq!(
        audits[1].1["after"]["item_name"],
        json!("货主商品标签已更新")
    );
    assert_eq!(audits[2].1["after"]["enabled"], json!(false));
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_dictionary_item_overrides_global_and_disable_hides_it(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let request = UpsertSystemDictionaryItemRequest {
        owner_id: Some(owner_id),
        item_name: "货主采购入库".to_string(),
        enabled: true,
        sort_order: 10,
        params: valid_document_type_params(),
        effective_from: None,
        effective_to: None,
    };
    let created = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            request.clone(),
            now,
            "system-dictionary-owner-override",
        )
        .await
        .expect("owner override should save");
    let replay = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            request.clone(),
            now,
            "system-dictionary-owner-override",
        )
        .await
        .expect("same dictionary idempotency key should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert_eq!(created.value.source, "owner");
    assert!(replay.replayed);

    sqlx::query(
        "UPDATE idempotency_request SET method = 'POST', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("system-dictionary-owner-override")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = repo
        .upsert_item(
            &ctx,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            request,
            now,
            "system-dictionary-owner-override",
        )
        .await
        .expect_err("method and path changes must invalidate a replay");
    assert_eq!(
        metadata_conflict,
        SystemDictionaryError::IdempotencyConflict
    );

    let item_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM system_dictionary_items WHERE dict_code = $1 AND item_code = $2 AND owner_id = $3",
    )
    .bind(SYSTEM_DICTIONARY_DOCUMENT_TYPE)
    .bind(DOCUMENT_TYPE_PURCHASE_INBOUND)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("dictionary item count should query");
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'upsert_system_dictionary_item' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("dictionary audit count should query");
    let audit_diff: serde_json::Value = sqlx::query_scalar(
        "SELECT diff FROM audit_event WHERE owner_id = $1 AND action = 'upsert_system_dictionary_item' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("dictionary audit diff should query");
    assert_eq!(item_rows, 1);
    assert_eq!(audit_rows, 1);
    assert_eq!(audit_diff["before"], serde_json::Value::Null);
    assert_eq!(audit_diff["after"]["item_name"], json!("货主采购入库"));

    let items = repo
        .list_effective_items(&ctx, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("merged items should be queryable");
    let purchase = items
        .iter()
        .find(|item| item.item_code == DOCUMENT_TYPE_PURCHASE_INBOUND)
        .expect("purchase inbound should exist");
    assert_eq!(purchase.item_name, "货主采购入库");
    assert_eq!(purchase.owner_id, Some(owner_id));

    repo.disable_item(
        &ctx,
        SYSTEM_DICTIONARY_DOCUMENT_TYPE,
        DOCUMENT_TYPE_PURCHASE_INBOUND,
        DisableSystemDictionaryItemRequest {
            owner_id: Some(owner_id),
            disabled_reason: Some("owner closed".to_string()),
        },
        now,
        "system-dictionary-owner-disable",
    )
    .await
    .expect("owner override should disable");

    let after_disable = repo
        .list_effective_items(&ctx, SYSTEM_DICTIONARY_DOCUMENT_TYPE, now)
        .await
        .expect("merged items after disable should be queryable");
    assert!(
        after_disable
            .iter()
            .all(|item| item.item_code != DOCUMENT_TYPE_PURCHASE_INBOUND),
        "disabled owner override must fail closed instead of falling back to global"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn special_drug_category_write_replays_once_and_is_queryable_with_one_audit(pool: PgPool) {
    let dictionary = PgSystemDictionaryRepository::new(pool.clone());
    let master_data = PgMasterDataReadRepository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 6, 9, 0, 0)
        .single()
        .expect("valid time");
    let request = UpsertSystemDictionaryItemRequest {
        owner_id: Some(owner_id),
        item_name: "货主管制药品".to_string(),
        enabled: true,
        sort_order: 10,
        params: json!({ "requires_dual_sign": true }),
        effective_from: None,
        effective_to: None,
    };

    let created = dictionary
        .upsert_item(
            &ctx,
            "special_drug_category",
            "narcotic",
            request.clone(),
            now,
            "special-drug-category-controlled",
        )
        .await
        .expect("owner special drug category should save");
    let replay = dictionary
        .upsert_item(
            &ctx,
            "special_drug_category",
            "narcotic",
            request,
            now,
            "special-drug-category-controlled",
        )
        .await
        .expect("same special drug category idempotency key should replay");
    assert_eq!(created.value.id, replay.value.id);
    assert!(replay.replayed);

    let categories = master_data
        .list_special_drug_categories(&ctx)
        .await
        .expect("special drug categories should query through postgres");
    let category = categories
        .iter()
        .find(|category| category.category_code == "narcotic")
        .expect("owner special drug category should override global item");
    assert_eq!(category.category_name, "货主管制药品");
    assert!(category.requires_dual_sign);

    let item_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM system_dictionary_items WHERE dict_code = 'special_drug_category' AND item_code = 'narcotic' AND owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("special drug category row count should query");
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'upsert_system_dictionary_item' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("special drug category audit count should query");
    assert_eq!(item_rows, 1);
    assert_eq!(audit_rows, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_rejects_invalid_params(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool);
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 28, 9, 0, 0)
        .single()
        .expect("valid time");

    let error = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "坏参数".to_string(),
                enabled: true,
                sort_order: 10,
                params: json!({
                    "direction": "sideways",
                    "workflow_template": "purchase_inbound"
                }),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-invalid-params",
        )
        .await
        .expect_err("invalid document_type params must be rejected");

    assert!(matches!(error, SystemDictionaryError::ParamInvalid { .. }));

    let negative_sort_order = repo
        .upsert_item(
            &ctx(owner_id),
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            UpsertSystemDictionaryItemRequest {
                owner_id: Some(owner_id),
                item_name: "坏排序号".to_string(),
                enabled: true,
                sort_order: -1,
                params: valid_document_type_params(),
                effective_from: None,
                effective_to: None,
            },
            now,
            "system-dictionary-invalid-sort-order",
        )
        .await
        .expect_err("negative sort order must be rejected");
    assert!(matches!(
        negative_sort_order,
        SystemDictionaryError::ParamInvalid { ref field, .. } if field == "sort_order"
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn document_type_impact_preview_counts_owner_scoped_m2_and_m4_references(pool: PgPool) {
    let repo = PgSystemDictionaryRepository::new(pool.clone());
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let ctx_a = ctx(owner_a);
    let effective_at = Utc
        .with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
        .single()
        .expect("valid time");

    for (owner_id, receipt_no, document_type, created_at) in [
        (
            owner_a,
            "ASN-IMPACT-001",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(2),
        ),
        (
            owner_a,
            "ASN-IMPACT-002",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(1),
        ),
        (
            owner_a,
            "ASN-IMPACT-FUTURE",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at + chrono::Duration::hours(1),
        ),
        (
            owner_b,
            "ASN-IMPACT-OTHER",
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            effective_at - chrono::Duration::hours(1),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, warehouse_id, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'draft', $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(receipt_no)
        .bind(document_type)
        .bind(Uuid::new_v4())
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed receiving order");
    }

    for (owner_id, wms_order_no, created_at) in [
        (
            owner_a,
            "OUT-IMPACT-001",
            effective_at - chrono::Duration::hours(2),
        ),
        (
            owner_a,
            "OUT-IMPACT-002",
            effective_at - chrono::Duration::hours(1),
        ),
        (
            owner_a,
            "OUT-IMPACT-FUTURE",
            effective_at + chrono::Duration::hours(1),
        ),
        (
            owner_b,
            "OUT-IMPACT-OTHER",
            effective_at - chrono::Duration::hours(1),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, wms_order_no, customer_id,
                delivery_address_id, delivery_address_snapshot,
                warehouse_id, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, gen_random_uuid(), '{}'::jsonb, $5, 'confirmed', $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(wms_order_no)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(created_at)
        .execute(&pool)
        .await
        .expect("seed outbound order");
    }

    let inbound_preview = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            owner_a,
            effective_at,
        )
        .await
        .expect("inbound document type impact preview");

    assert_eq!(inbound_preview.total_references, 2);
    assert_eq!(inbound_preview.references.len(), 1);
    assert_eq!(inbound_preview.references[0].module_code, "M2");
    assert_eq!(
        inbound_preview.references[0].business_object,
        "receiving_orders"
    );
    assert_eq!(inbound_preview.references[0].reference_count, 2);

    let outbound_preview = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_SALES_OUTBOUND,
            owner_a,
            effective_at,
        )
        .await
        .expect("outbound document type impact preview");

    assert_eq!(outbound_preview.total_references, 2);
    assert_eq!(outbound_preview.references.len(), 1);
    assert_eq!(outbound_preview.references[0].module_code, "M4");
    assert_eq!(
        outbound_preview.references[0].business_object,
        "outbound_orders"
    );
    assert_eq!(outbound_preview.references[0].reference_count, 2);

    let error = repo
        .preview_item_impact(
            &ctx_a,
            SYSTEM_DICTIONARY_DOCUMENT_TYPE,
            DOCUMENT_TYPE_PURCHASE_INBOUND,
            owner_b,
            effective_at,
        )
        .await
        .expect_err("cross-owner preview must be rejected");
    assert_eq!(error, SystemDictionaryError::CrossOwnerAccess);
}
