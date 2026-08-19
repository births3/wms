use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CreateReceivingOrderRequest, ReceivingOrderLine, RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-reference-test".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request() -> CreateReceivingOrderRequest {
    CreateReceivingOrderRequest {
        receipt_no: format!("M2-REF-{}", Uuid::new_v4()),
        document_type: RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND.to_string(),
        supplier_id: Some(Uuid::new_v4()),
        warehouse_id: Uuid::new_v4(),
        external_ref: None,
        expected_arrival_at: Some(chrono::Utc::now() + chrono::Duration::days(1)),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "M2-REFERENCE-PRODUCT".to_string(),
            expected_qty: 1.into(),
            batch_no: None,
            production_date: None,
            expiry_date: None,
        }],
    }
}

async fn seed_references(
    pool: &PgPool,
    owner_id: Uuid,
    request: &mut CreateReceivingOrderRequest,
) -> Uuid {
    let supplier_id = request.supplier_id.expect("supplier id");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'Reference Supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("M2-REF-SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("M2-REF-USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed supplier");
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'Reference Product', '1 unit', 'normal_10_30', 'active') RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(&request.lines[0].product_code)
    .fetch_one(pool)
    .await
    .expect("seed product");
    request.lines[0].product_id = Some(product_id);
    product_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_asn_rejects_inactive_supplier_and_cross_owner_product(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut request = request();
    let own_product_id = seed_references(&pool, owner_id, &mut request).await;
    let supplier_id = request.supplier_id.expect("supplier id");
    let product_code = request.lines[0].product_code.clone();

    sqlx::query("UPDATE suppliers SET status = 'disabled' WHERE id = $1 AND owner_id = $2")
        .bind(supplier_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("disable supplier");
    assert!(matches!(
        repository
            .create_receiving_order(&ctx, request.clone(), chrono::Utc::now())
            .await,
        Err(Wave3RepositoryError::NotFound)
    ));

    sqlx::query("UPDATE suppliers SET status = 'active' WHERE id = $1 AND owner_id = $2")
        .bind(supplier_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("restore supplier");
    let foreign_product_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'Foreign Product', '1 unit', 'normal_10_30', 'active')",
    )
    .bind(foreign_product_id)
    .bind(Uuid::new_v4())
    .bind(&product_code)
    .execute(&pool)
    .await
    .expect("seed foreign product");
    request.lines[0].product_id = Some(foreign_product_id);
    assert!(matches!(
        repository
            .create_receiving_order(&ctx, request, chrono::Utc::now())
            .await,
        Err(Wave3RepositoryError::NotFound)
    ));

    let order_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("count rolled-back ASN");
    assert_eq!(order_count, 0);
    assert_ne!(own_product_id, foreign_product_id);
}
