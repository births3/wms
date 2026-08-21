use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave3_handlers::{wave3_router, Wave3AppState},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CancelReceivingOrderRequest, CreateReceivingOrderRequest, InspectReceivingOrderRequest,
    ReceiveReceivingOrderRequest, ReceivingOrderLine, RejectReceivingOrderRequest,
    RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-adversarial-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn forbidden_inbound(
    pool: PgPool,
    method: &str,
    uri: String,
    permissions: &[&str],
    body: String,
) {
    let app = wave3_router(Wave3AppState::with_postgres(pool));
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("idempotency-key", "m2-adversarial-forbidden")
        .body(Body::from(body))
        .expect("m2 adversarial request should build");
    request
        .extensions_mut()
        .insert(ctx(Uuid::new_v4(), permissions));
    let response = app
        .oneshot(request)
        .await
        .expect("m2 adversarial route should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_write_http_requires_m2_write_permission(pool: PgPool) {
    let order_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let signer_id = Uuid::new_v4();
    forbidden_inbound(
        pool.clone(),
        "POST",
        "/api/v1/inbound/receiving-orders".to_string(),
        &["m2.read"],
        format!(
            r#"{{"receipt_no":"M2-ADV","document_type":"purchase_inbound","warehouse_id":"{warehouse_id}","expected_arrival_at":"2026-08-22T00:00:00Z","lines":[{{"line_no":1,"product_code":"P-M2-001","expected_qty":"10"}}]}}"#
        ),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/receive"),
        &["m2.read"],
        r#"{"actual_qty":"10","shortage_qty":"0","rejected_qty":"0"}"#.to_string(),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/inspect"),
        &["m2.read"],
        r#"{"batch_no":"B-1","accepted_qty":"1","rejected_qty":"0","production_date":"2026-01-01","expiry_date":"2028-01-01","quality_status":"qualified","trace_codes":[]}"#.to_string(),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/sign"),
        &["m2.read"],
        format!(r#"{{"first_signer_id":"{signer_id}","dual_required":false}}"#),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/reject"),
        &["m2.read"],
        r#"{"reason":"质量拒收"}"#.to_string(),
    )
    .await;
    forbidden_inbound(
        pool,
        "GET",
        "/api/v1/inbound/receiving-dashboard".to_string(),
        &["m3.read"],
        String::new(),
    )
    .await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_http_requires_putaway_write_permission(pool: PgPool) {
    let order_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/putaway"),
        &["m2.write", "m2.read"],
        format!(
            r#"{{"batch_no":"B-1","product_code":"P-M2-001","qty":"1","location_id":"{location_id}","location_code":"A01-01-01-01","quality_status":"qualified"}}"#
        ),
    )
    .await;
    forbidden_inbound(
        pool,
        "PUT",
        "/api/v1/inbound/putaway-strategy-profiles".to_string(),
        &["m2.write", "m2.read"],
        r#"{"profile_code":"default","profile_name":"默认上架"}"#.to_string(),
    )
    .await;
}

async fn seed_draft_order(pool: &PgPool, owner_id: Uuid) -> Uuid {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M2 对抗货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M2-ADV-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("adversarial owner should seed");
    let mut request = CreateReceivingOrderRequest {
        receipt_no: format!("M2-ADV-{}", Uuid::new_v4()),
        document_type: RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND.to_string(),
        supplier_id: Some(Uuid::new_v4()),
        warehouse_id: Uuid::new_v4(),
        external_ref: None,
        expected_arrival_at: Some(Utc::now() + chrono::Duration::days(1)),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-M2-ADV".to_string(),
            expected_qty: 10.into(),
            batch_no: None,
            production_date: None,
            expiry_date: None,
        }],
    };
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M2 对抗仓', 'normal', 'active')",
    )
    .bind(request.warehouse_id)
    .bind(owner_id)
    .bind(format!("M2-ADV-WH-{}", &request.warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("adversarial warehouse should seed");
    let supplier_id = request.supplier_id.expect("supplier");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'M2 对抗供应商', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("M2-ADV-SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("M2-ADV-USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("adversarial supplier should seed");
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'M2 对抗商品', '1 unit', 'normal_10_30', 'active') RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(&request.lines[0].product_code)
    .fetch_one(pool)
    .await
    .expect("adversarial product should seed");
    request.lines[0].product_id = Some(product_id);
    PgWave3Repository::new(pool.clone())
        .create_receiving_order(&ctx(owner_id, &["m2.write"]), request, Utc::now())
        .await
        .expect("draft ASN should create")
        .id
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_actions_reject_cross_owner_order_and_draft_receive(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let order_id = seed_draft_order(&pool, owner_id).await;
    let repository = PgWave3Repository::new(pool.clone());
    let stranger = ctx(Uuid::new_v4(), &["m2.write", "m2.putaway.write"]);
    let now = Utc::now();
    let receive = ReceiveReceivingOrderRequest {
        actual_qty: 10.into(),
        shortage_qty: 0.into(),
        rejected_qty: 0.into(),
        arrival_temperature_celsius: None,
        exception_note: None,
        details: None,
    };
    let inspect = InspectReceivingOrderRequest {
        batch_no: "B-ADV".to_string(),
        accepted_qty: 1.into(),
        rejected_qty: 0.into(),
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        quality_status: "qualified".to_string(),
        trace_codes: vec![],
        appearance_check: Some("完好".to_string()),
        package_check: Some("完好".to_string()),
        instruction_check: Some("有".to_string()),
        label_check: Some("清晰".to_string()),
        sampling_qty: Some(1.into()),
        approval_no: None,
    };
    assert_eq!(
        repository
            .get_receiving_order_print_data(&stranger, order_id)
            .await
            .expect_err("cross-owner print data must fail"),
        Wave3RepositoryError::NotFound
    );
    assert_eq!(
        repository
            .receive_receiving_order_with_audit(
                &stranger,
                order_id,
                receive.clone(),
                now,
                "m2-adv-receive-cross",
                None,
            )
            .await
            .expect_err("cross-owner receive must fail"),
        Wave3RepositoryError::NotFound
    );
    assert_eq!(
        repository
            .inspect_receiving_order_with_audit(
                &stranger,
                order_id,
                inspect,
                now.date_naive(),
                now,
                "m2-adv-inspect-cross",
                None,
            )
            .await
            .expect_err("cross-owner inspect must fail"),
        Wave3RepositoryError::NotFound
    );
    assert_eq!(
        repository
            .reject_receiving_order_with_audit(
                &stranger,
                order_id,
                RejectReceivingOrderRequest {
                    reason: "跨货主拒收".to_string(),
                },
                now,
                "m2-adv-reject-cross",
                None,
            )
            .await
            .expect_err("cross-owner reject must fail"),
        Wave3RepositoryError::NotFound
    );
    assert_eq!(
        repository
            .cancel_receiving_order_with_audit(
                &stranger,
                order_id,
                CancelReceivingOrderRequest {
                    reason: "跨货主作废".to_string(),
                    approval_id: None,
                },
                now,
                "m2-adv-cancel-cross",
                None,
            )
            .await
            .expect_err("cross-owner cancel must fail"),
        Wave3RepositoryError::NotFound
    );

    let owner = ctx(owner_id, &["m2.write"]);
    let draft_receive = repository
        .receive_receiving_order_with_audit(
            &owner,
            order_id,
            receive,
            now,
            "m2-adv-receive-draft",
            None,
        )
        .await
        .expect_err("draft ASN must not be received");
    assert!(matches!(
        draft_receive,
        Wave3RepositoryError::InvalidStatus { ref expected, .. } if expected == "released"
    ));
    let unchanged = repository
        .get_receiving_order(&owner, order_id)
        .await
        .expect("owner draft should remain");
    assert_eq!(unchanged.status, "draft");
}
