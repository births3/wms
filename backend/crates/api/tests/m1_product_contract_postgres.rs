use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext, master_data::MasterDataError,
    master_data_postgres::PgMasterDataReadRepository,
};
use wms_domain::{CreateProductRequest, ProductPackagingLevelInput, UpdateProductRequest};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m1-product-contract-test".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn complete_product_request() -> CreateProductRequest {
    CreateProductRequest {
        product_code: "P-CONTRACT-001".to_string(),
        product_name: "三级包装测试商品".to_string(),
        approval_no: Some("国药准字 H20260001".to_string()),
        spec: "10mg*12支".to_string(),
        dosage_form: Some("片剂".to_string()),
        manufacturer: Some("测试药业".to_string()),
        special_drug_category_code: Some("none".to_string()),
        udi_code: Some("06912345678901".to_string()),
        electronic_regulatory_code: Some("REG-2026-0001".to_string()),
        length_mm: Some(120.0),
        width_mm: Some(80.0),
        height_mm: Some(50.0),
        volume_cm3: None,
        weight_g: Some(350.5),
        packaging_levels: vec![
            ProductPackagingLevelInput {
                unit_code: "piece".to_string(),
                unit_name: "支".to_string(),
                ratio_to_base: 1,
                is_base: true,
                is_default: false,
                sort_order: 1,
            },
            ProductPackagingLevelInput {
                unit_code: "box".to_string(),
                unit_name: "盒".to_string(),
                ratio_to_base: 12,
                is_base: false,
                is_default: true,
                sort_order: 2,
            },
            ProductPackagingLevelInput {
                unit_code: "case".to_string(),
                unit_name: "件".to_string(),
                ratio_to_base: 120,
                is_base: false,
                is_default: false,
                sort_order: 3,
            },
        ],
        attrs: json!({"storage_condition": "normal", "source": "api_import"}),
    }
}

#[test]
fn update_contract_preserves_explicit_null_for_nullable_fields() {
    let request: UpdateProductRequest = serde_json::from_value(json!({
        "approval_no": null,
        "udi_code": null,
        "length_mm": null
    }))
    .expect("nullable product patch should deserialize");
    assert!(request.approval_no.is_some());
    assert!(request.udi_code.is_some());
    assert!(request.length_mm.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn complete_product_contract_is_atomic_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let ctx = context(owner_id);
    let request = complete_product_request();

    let created = repo
        .create_product(&ctx, request.clone(), Utc::now(), "product-contract-key")
        .await
        .expect("complete product should be created");
    assert_eq!(created.packaging_levels.len(), 3);
    assert_eq!(
        created.volume_cm3,
        Some(480.0),
        "volume should be calculated from millimetres"
    );
    assert!(created.packaging_levels[0].is_base);
    assert!(created.packaging_levels[1].is_default);

    let replay = repo
        .create_product(&ctx, request, Utc::now(), "product-contract-key")
        .await
        .expect("same request should replay");
    assert_eq!(replay.id, created.id);
    assert_eq!(replay.packaging_levels.len(), 3);

    let packaging_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM product_packaging_levels WHERE product_id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .expect("packaging count");
    assert_eq!(packaging_count, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_packaging_is_rejected_without_partial_product(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let ctx = context(owner_id);
    let mut request = complete_product_request();
    request.packaging_levels[1].is_base = true;

    let result = repo
        .create_product(&ctx, request, Utc::now(), "invalid-packaging-key")
        .await;
    assert!(matches!(
        result,
        Err(MasterDataError::InvalidProductPackaging)
    ));
    let product_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("product count");
    assert_eq!(product_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_packaging_update_replaces_levels_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let ctx = context(owner_id);
    let created = repo
        .create_product(
            &ctx,
            complete_product_request(),
            Utc::now(),
            "product-update-create-key",
        )
        .await
        .expect("product should be created");
    let update = UpdateProductRequest {
        product_name: None,
        approval_no: None,
        spec: None,
        dosage_form: None,
        manufacturer: None,
        special_drug_category_code: None,
        udi_code: None,
        electronic_regulatory_code: None,
        length_mm: Some(Some(100.0)),
        width_mm: Some(Some(60.0)),
        height_mm: Some(Some(40.0)),
        volume_cm3: None,
        weight_g: None,
        packaging_levels: Some(vec![
            ProductPackagingLevelInput {
                unit_code: "piece".to_string(),
                unit_name: "支".to_string(),
                ratio_to_base: 1,
                is_base: true,
                is_default: true,
                sort_order: 1,
            },
            ProductPackagingLevelInput {
                unit_code: "case".to_string(),
                unit_name: "件".to_string(),
                ratio_to_base: 100,
                is_base: false,
                is_default: false,
                sort_order: 2,
            },
        ]),
        status: None,
        attrs: None,
    };

    let updated = repo
        .update_product(
            &ctx,
            created.id,
            update.clone(),
            Utc::now(),
            "product-update-key",
        )
        .await
        .expect("product should update");
    assert_eq!(updated.packaging_levels.len(), 2);
    assert_eq!(updated.volume_cm3, Some(240.0));

    let replay = repo
        .update_product(&ctx, created.id, update, Utc::now(), "product-update-key")
        .await
        .expect("same update should replay");
    assert_eq!(replay.packaging_levels.len(), 2);
    let stored = repo
        .get_product(&ctx, created.id)
        .await
        .expect("updated product should load");
    assert_eq!(stored.packaging_levels.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn application_role_can_replace_product_packaging_levels(pool: PgPool) {
    let can_delete: bool = sqlx::query_scalar(
        "SELECT has_table_privilege('wms_app', 'product_packaging_levels', 'DELETE')",
    )
    .fetch_one(&pool)
    .await
    .expect("application role packaging privilege should query");

    assert!(
        can_delete,
        "wms_app must be able to delete old levels during atomic replacement"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_update_normalizes_udi_before_owner_unique_check(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let ctx = context(owner_id);
    let first = repo
        .create_product(
            &ctx,
            complete_product_request(),
            Utc::now(),
            "udi-normalization-first",
        )
        .await
        .expect("first product should create");
    let mut second_request = complete_product_request();
    second_request.product_code = "P-CONTRACT-002".to_string();
    second_request.product_name = "UDI 更新测试商品".to_string();
    second_request.udi_code = Some("06912345678902".to_string());
    let second = repo
        .create_product(&ctx, second_request, Utc::now(), "udi-normalization-second")
        .await
        .expect("second product should create");

    let result = repo
        .update_product(
            &ctx,
            second.id,
            UpdateProductRequest {
                product_name: None,
                approval_no: None,
                spec: None,
                dosage_form: None,
                manufacturer: None,
                special_drug_category_code: None,
                udi_code: Some(Some(format!(" {} ", first.udi_code.expect("first UDI")))),
                electronic_regulatory_code: None,
                length_mm: None,
                width_mm: None,
                height_mm: None,
                volume_cm3: None,
                weight_g: None,
                packaging_levels: None,
                status: None,
                attrs: None,
            },
            Utc::now(),
            "udi-normalization-update",
        )
        .await;

    assert!(matches!(result, Err(MasterDataError::DuplicateProductUdi)));
    let stored = repo
        .get_product(&ctx, second.id)
        .await
        .expect("second product should remain readable");
    assert_eq!(stored.udi_code.as_deref(), Some("06912345678902"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_update_can_clear_nullable_contract_fields(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repo = PgMasterDataReadRepository::new(pool.clone());
    let ctx = context(owner_id);
    let created = repo
        .create_product(
            &ctx,
            complete_product_request(),
            Utc::now(),
            "product-clear-create-key",
        )
        .await
        .expect("product should be created");
    let cleared = repo
        .update_product(
            &ctx,
            created.id,
            UpdateProductRequest {
                product_name: None,
                approval_no: Some(None),
                spec: None,
                dosage_form: Some(None),
                manufacturer: Some(None),
                special_drug_category_code: None,
                udi_code: Some(None),
                electronic_regulatory_code: Some(None),
                length_mm: Some(None),
                width_mm: Some(None),
                height_mm: Some(None),
                volume_cm3: Some(None),
                weight_g: Some(None),
                packaging_levels: None,
                status: None,
                attrs: None,
            },
            Utc::now(),
            "product-clear-update-key",
        )
        .await
        .expect("nullable product fields should clear");

    assert_eq!(cleared.approval_no, None);
    assert_eq!(cleared.dosage_form, None);
    assert_eq!(cleared.manufacturer, None);
    assert_eq!(cleared.udi_code, None);
    assert_eq!(cleared.electronic_regulatory_code, None);
    assert_eq!(cleared.length_mm, None);
    assert_eq!(cleared.width_mm, None);
    assert_eq!(cleared.height_mm, None);
    assert_eq!(cleared.volume_cm3, None);
    assert_eq!(cleared.weight_g, None);
    assert_eq!(cleared.packaging_levels.len(), 3);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'update_product' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("load update audit");
    assert_eq!(audit_count, 1);
}
