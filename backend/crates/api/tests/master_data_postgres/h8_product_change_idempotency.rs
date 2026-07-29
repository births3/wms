#[sqlx::test(migrations = "../../migrations")]
async fn h8_product_change_replays_without_duplicate_update_or_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 23, 9, 0, 0)
        .single()
        .expect("valid time");
    seed_product(
        &pool,
        owner_id,
        "P-H8-CHANGE-001",
        "H8 变更前商品",
        "normal",
        now,
    )
    .await;
    let product_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM products WHERE owner_id = $1 AND product_code = 'P-H8-CHANGE-001'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("seeded product should exist");
    let repository = PgMasterDataReadRepository::new(pool.clone());
    let auth = ctx(owner_id);
    let first = repository
        .update_product(
            &auth,
            product_id,
            product_update(json!({"spec": "10mg*30片"})),
            now,
            "h8-product-change-1",
        )
        .await
        .expect("controlled H8 product change should succeed");
    let replayed = repository
        .update_product(
            &auth,
            product_id,
            product_update(json!({"spec": "10mg*30片"})),
            now,
            "h8-product-change-1",
        )
        .await
        .expect("controlled H8 product change should replay");

    assert_eq!(replayed.id, first.id);
    assert_eq!(replayed.updated_at, first.updated_at);
    let evidence: (String, i64, i64, i64) = sqlx::query_as(
        "SELECT specification, version, (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'update_product' AND resource_id = $2::text), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'h8-product-change-1') FROM products WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(product_id)
    .fetch_one(&pool)
    .await
    .expect("product change evidence should query");
    assert_eq!(evidence, ("10mg*30片".to_string(), 2, 1, 1));
}
