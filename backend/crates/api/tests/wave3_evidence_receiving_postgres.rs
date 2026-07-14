use chrono::{Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest, auth::AuthContext, inventory::STATUS_QUALIFIED,
    wave3_repository::PgWave3Repository,
};
use wms_domain::{
    CreateReceivingOrderRequest, InspectReceivingOrderRequest, ReceiveReceivingOrderRequest,
    ReceivingOrderLine, SignInspectionRequest,
};

#[path = "support/auth.rs"]
mod auth_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "postgres-test".to_string(),
        permissions: vec!["m2.write".to_string(), "m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

fn audit(ctx: &AuthContext, action: &str, module: &str, resource_type: &str) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(ctx, action, module, resource_type, "", None)
}

fn receiving_order_req(receipt_no: &str) -> CreateReceivingOrderRequest {
    CreateReceivingOrderRequest {
        receipt_no: receipt_no.to_string(),
        document_type: "purchase_inbound".to_string(),
        supplier_id: Some(Uuid::new_v4()),
        warehouse_id: Uuid::new_v4(),
        external_ref: Some(format!("ERP-{receipt_no}")),
        expected_arrival_at: Some(Utc::now() + Duration::days(1)),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10,
            batch_no: None,
            production_date: None,
            expiry_date: None,
        }],
    }
}

async fn seed_receiving_references(
    pool: &PgPool,
    owner_id: Uuid,
    request: &mut CreateReceivingOrderRequest,
) {
    let supplier_id = request.supplier_id.expect("request supplier");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'Evidence Supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("EVIDENCE-SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("EVIDENCE-USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed evidence supplier");
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'Evidence Product', '1 unit', 'normal', 'active') RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(&request.lines[0].product_code)
    .fetch_one(pool)
    .await
    .expect("seed evidence product");
    request.lines[0].product_id = Some(product_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inspect_and_sign_receiving_order_replay_without_duplicate_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let second_signer_id = Uuid::new_v4();
    auth_support::seed_receiving_verifiers(&pool, owner_id, &[ctx.user_id, second_signer_id]).await;
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid time");
    let mut request = receiving_order_req("ASN-PG-INSPECT-001");
    seed_receiving_references(&pool, owner_id, &mut request).await;
    let order = repo
        .create_receiving_order(&ctx, request, now)
        .await
        .expect("create receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repo.receive_receiving_order_with_audit(
        &ctx,
        order.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 10,
            shortage_qty: 0,
            rejected_qty: 0,
            arrival_temperature_celsius: None,
            exception_note: None,
            details: None,
        },
        now,
        "receive-inspect-replay",
        None,
    )
    .await
    .expect("receive before inspection");

    let inspect_req = InspectReceivingOrderRequest {
        batch_no: "B202606".to_string(),
        accepted_qty: 10,
        rejected_qty: 0,
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        quality_status: STATUS_QUALIFIED.to_string(),
        trace_codes: vec!["TRACE-PG-001".to_string()],
    };
    let first = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            inspect_req.clone(),
            now.date_naive(),
            now,
            "idem-inspect-1",
            Some(audit(&ctx, "inspect", "M2", "receiving_inspection")),
        )
        .await
        .expect("inspect receiving order");
    let replay = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            inspect_req,
            now.date_naive(),
            now,
            "idem-inspect-1",
            Some(audit(&ctx, "inspect", "M2", "receiving_inspection")),
        )
        .await
        .expect("replay inspection");
    assert_eq!(first.value.id, replay.value.id);
    assert!(replay.replayed);
    let inspected_line: (Option<String>, Option<NaiveDate>, Option<NaiveDate>) = sqlx::query_as(
        "SELECT batch_no, production_date, expiry_date FROM receiving_order_lines WHERE receiving_order_id = $1 AND owner_id = $2",
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("inspection should persist confirmed batch dates on the order line");
    assert_eq!(inspected_line.0.as_deref(), Some("B202606"));
    assert_eq!(inspected_line.1, NaiveDate::from_ymd_opt(2026, 1, 1));
    assert_eq!(inspected_line.2, NaiveDate::from_ymd_opt(2028, 1, 1));

    let sign_req = SignInspectionRequest {
        first_signer_id: ctx.user_id,
        second_signer_id: Some(second_signer_id),
        dual_required: true,
    };
    let first_sign = repo
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            sign_req.clone(),
            now,
            "idem-sign-1",
            Some(audit(&ctx, "sign", "M2", "receiving_inspection_signature")),
        )
        .await
        .expect("sign receiving inspection");
    let replay_sign = repo
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            sign_req,
            now,
            "idem-sign-1",
            Some(audit(&ctx, "sign", "M2", "receiving_inspection_signature")),
        )
        .await
        .expect("replay signature");
    assert_eq!(first_sign.value.id, replay_sign.value.id);
    assert!(replay_sign.replayed);

    let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            (SELECT COUNT(*) FROM receiving_inspections WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM receiving_inspection_signatures WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-inspect-1'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-sign-1'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'inspect'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'sign')"#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("inspection evidence counts");
    assert_eq!(counts, (1, 1, 1, 1, 1, 1));
}
