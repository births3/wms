#[sqlx::test(migrations = "../../migrations")]
async fn create_asn_rejects_expired_supplier_qualification(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut request).await;
    let supplier_id = request.supplier_id.expect("supplier");
    sqlx::query(
        "UPDATE suppliers SET qualification_valid_until = $3 WHERE id = $1 AND owner_id = $2",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(chrono::Utc::now() - chrono::Duration::days(1))
    .execute(&pool)
    .await
    .expect("expire supplier qualification");

    let error = repository
        .create_receiving_order(&ctx, request, chrono::Utc::now())
        .await
        .expect_err("expired supplier must fail");
    assert!(matches!(
        error,
        Wave3RepositoryError::SupplierQualificationExpired
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn cancel_released_asn_requires_approval_and_writes_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut request).await;
    let order = repository
        .create_receiving_order(&ctx, request, chrono::Utc::now())
        .await
        .expect("create asn");
    repository
        .release_receiving_order(&ctx, order.id, chrono::Utc::now())
        .await
        .expect("release asn");

    let missing_approval = repository
        .cancel_receiving_order_with_audit(
            &ctx,
            order.id,
            CancelReceivingOrderRequest {
                reason: "供应商取消发货".to_string(),
                approval_id: None,
            },
            chrono::Utc::now(),
            "m2-cancel-no-approval",
            None,
        )
        .await
        .expect_err("released cancel needs approval");
    assert!(matches!(
        missing_approval,
        Wave3RepositoryError::MissingApprovalSource
    ));

    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "cancel",
        "M2",
        "receiving_order",
        order.id.to_string(),
        None,
    );
    let cancelled = repository
        .cancel_receiving_order_with_audit(
            &ctx,
            order.id,
            CancelReceivingOrderRequest {
                reason: "供应商取消发货".to_string(),
                approval_id: Some("H4-APPROVAL-M2-001".to_string()),
            },
            chrono::Utc::now(),
            "m2-cancel-approved",
            Some(audit),
        )
        .await
        .expect("cancel asn");
    assert!(!cancelled.replayed);
    assert_eq!(cancelled.value.status, "cancelled");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_id = $2 AND action = 'cancel'",
    )
    .bind(owner_id)
    .bind(order.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count cancel audit");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn force_close_shortage_from_receiving_with_shortage_qty(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut request).await;
    let order = repository
        .create_receiving_order(&ctx, request, chrono::Utc::now())
        .await
        .expect("create asn");
    repository
        .release_receiving_order(&ctx, order.id, chrono::Utc::now())
        .await
        .expect("release asn");
    repository
        .receive_receiving_order(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8,
                shortage_qty: 2,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: Some("到货短少 2".to_string()),
                details: None,
            },
            chrono::Utc::now(),
            "m2-receive-shortage",
        )
        .await
        .expect("receive with shortage");

    let closed = repository
        .force_close_shortage_with_audit(
            &ctx,
            order.id,
            ForceCloseShortageRequest {
                reason: "缺货部分由 ERP 重推".to_string(),
            },
            chrono::Utc::now(),
            "m2-force-close-shortage",
            Some(AuditWriteRequest::from_auth_context(
                &ctx,
                "force_close_shortage",
                "M2",
                "receiving_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("force close shortage");
    assert_eq!(closed.value.status, "closed_shortage");
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_strategy_profile_drives_default_top_n(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-putaway-strategy".to_string(),
        permissions: vec!["m2.putaway.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    };
    let repository = PgWave3Repository::new(pool.clone());
    let profile = repository
        .upsert_putaway_strategy_profile_with_audit(
            &ctx,
            UpsertPutawayStrategyProfileRequest {
                profile_code: "default".to_string(),
                profile_name: "通用方案".to_string(),
                is_default: true,
                top_n: 2,
                enabled_rules: Some(serde_json::json!({
                    "temperature_match": true,
                    "owner_isolation": true,
                    "capacity_match": true,
                    "same_product_cluster": false,
                    "quality_color_match": true,
                    "empty_location_first": true
                })),
                rule_priority: Some(serde_json::json!([
                    "temperature_match",
                    "owner_isolation",
                    "capacity_match",
                    "empty_location_first",
                    "same_product_cluster"
                ])),
                warehouse_id: None,
                product_category: Some("western_medicine".to_string()),
                notify_on_no_location: true,
                status: "active".to_string(),
            },
            chrono::Utc::now(),
            "m2-upsert-putaway-strategy",
            AuditWriteRequest::from_auth_context(
                &ctx,
                "upsert",
                "M2",
                "putaway_strategy_profile",
                "pending".to_string(),
                None,
            ),
        )
        .await
        .expect("upsert strategy");
    assert_eq!(profile.value.top_n, 2);
    assert!(profile.value.is_default);
    assert_eq!(
        profile.value.product_category.as_deref(),
        Some("western_medicine")
    );
    assert!(profile.value.notify_on_no_location);

    let listed = repository
        .list_putaway_strategy_profiles(&ctx)
        .await
        .expect("list strategies");
    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].profile_code, "default");
    assert_eq!(
        listed.data[0].product_category.as_deref(),
        Some("western_medicine")
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_strategy_rejects_foreign_warehouse_binding(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-putaway-strategy-bind".to_string(),
        permissions: vec!["m2.putaway.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    };
    let repository = PgWave3Repository::new(pool.clone());
    let foreign_warehouse = Uuid::new_v4();
    let error = repository
        .upsert_putaway_strategy_profile_with_audit(
            &ctx,
            UpsertPutawayStrategyProfileRequest {
                profile_code: "wh-bound".to_string(),
                profile_name: "仓库绑定方案".to_string(),
                is_default: false,
                top_n: 3,
                enabled_rules: None,
                rule_priority: None,
                warehouse_id: Some(foreign_warehouse),
                product_category: None,
                notify_on_no_location: true,
                status: "active".to_string(),
            },
            chrono::Utc::now(),
            "m2-upsert-foreign-warehouse",
            AuditWriteRequest::from_auth_context(
                &ctx,
                "upsert",
                "M2",
                "putaway_strategy_profile",
                "pending".to_string(),
                None,
            ),
        )
        .await
        .expect_err("foreign warehouse must fail");
    assert!(matches!(error, Wave3RepositoryError::NotFound));
}
