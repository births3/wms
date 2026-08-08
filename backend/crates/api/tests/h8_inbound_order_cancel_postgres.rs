use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    h8_inbound::{h8_inbound_router, H8InboundAppState},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
    wave4_repository::{PgWave4Repository, Wave4RepositoryError},
};
use wms_domain::{CreateOutboundWaveRequest, ReceiveReceivingOrderRequest};

async fn seed_context(pool: &PgPool, owner_id: Uuid, api_key_id: Uuid) -> Uuid {
    let warehouse_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id,owner_code,owner_name) VALUES ($1,$2,'cancel owner')")
        .bind(owner_id)
        .bind(format!("CANCEL-{}", &owner_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("seed cancel owner");
    sqlx::query("INSERT INTO warehouses (id,owner_id,warehouse_code,warehouse_name,warehouse_type,status) VALUES ($1,$2,$3,'cancel warehouse','normal','active')")
        .bind(warehouse_id).bind(owner_id).bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
        .execute(pool).await.expect("seed cancel warehouse");
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors (
            id,owner_id,connector_code,connector_name,warehouse_ids,directions,message_types,
            channel_mode,api_key_id,status,config_version,first_activated_at,last_tested_version,
            last_tested_at,last_tested_succeeded
        ) VALUES ($1,$2,'H8-CANCEL','H8 cancel',ARRAY[]::uuid[],ARRAY['inbound'],
            ARRAY['order_cancel'],'rest',$3,'active',1,now(),1,now(),TRUE)"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("seed cancel connector");
    warehouse_id
}

fn app(pool: PgPool, owner_id: Uuid, api_key_id: Uuid) -> axum::Router {
    h8_inbound_router(H8InboundAppState::with_postgres(pool)).layer(Extension(AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 cancel API Key".to_string(),
        permissions: vec!["m2.write".to_string(), "m4.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    }))
}

fn cancel_request(code: &str, revision: i32, order_type: i32, command_id: &str) -> Request<Body> {
    let body = json!({
        "schema_version": "1",
        "external_ref": command_id,
        "correlation_id": format!("corr-{command_id}"),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": null,
        "command_id": command_id,
        "command_type": 99,
        "erp_bill_code": code,
        "revision": revision,
        "order_type": order_type,
        "memo": "ERP 作废"
    });
    Request::builder()
        .method("POST")
        .uri("/api/v1/integration/erp-messages/inbound/order_cancel")
        .header("content-type", "application/json")
        .header("Idempotency-Key", command_id)
        .body(Body::from(body.to_string()))
        .expect("order cancel request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_cancel_succeeds_before_wave_and_rejects_after_wave(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = seed_context(&pool, owner_id, api_key_id).await;
    for (code, status) in [("ERP-OUT-CANCEL", "confirmed"), ("ERP-OUT-WAVE", "in_wave")] {
        sqlx::query("INSERT INTO outbound_orders (id,owner_id,wms_order_no,erp_bill_code,erp_revision,erp_order_type,customer_id,delivery_address_id,delivery_address_snapshot,warehouse_id,status) VALUES ($1,$2,$3,$4,1,2,$5,$6,'{}'::jsonb,$7,$8)")
            .bind(Uuid::new_v4()).bind(owner_id).bind(format!("WMS-{code}"))
            .bind(code).bind(Uuid::new_v4()).bind(Uuid::new_v4()).bind(warehouse_id).bind(status)
            .execute(&pool).await.expect("seed outbound order");
    }
    let service = app(pool.clone(), owner_id, api_key_id);
    for (code, command) in [
        ("ERP-OUT-CANCEL", "cmd-out-ok"),
        ("ERP-OUT-WAVE", "cmd-out-reject"),
    ] {
        let response = service
            .clone()
            .oneshot(cancel_request(code, 1, 2, command))
            .await
            .expect("outbound cancel should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let evidence: Vec<(String, String, Value)> = sqlx::query_as(
        r#"SELECT o.erp_bill_code,o.status,f.payload
             FROM outbound_orders o
             JOIN shipment_confirm_erp_feedback_outbox f ON f.outbound_order_id=o.id
            WHERE o.owner_id=$1 ORDER BY o.erp_bill_code"#,
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("load outbound cancel evidence");
    assert_eq!(evidence[0].0, "ERP-OUT-CANCEL");
    assert_eq!(evidence[0].1, "cancelled");
    assert_eq!(evidence[0].2["feedback_type"], 100);
    assert_eq!(evidence[1].0, "ERP-OUT-WAVE");
    assert_eq!(evidence[1].1, "in_wave");
    assert_eq!(evidence[1].2["result_code"], "ORDER_ALREADY_IN_WAVE");
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_multi_asn_cancel_is_all_or_nothing_and_missing_order_is_too_early(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = seed_context(&pool, owner_id, api_key_id).await;
    for (line, status) in [(1, "released"), (2, "receiving")] {
        sqlx::query("INSERT INTO receiving_orders (id,owner_id,receipt_no,document_type,warehouse_id,erp_bill_code,erp_revision,erp_line_no,status) VALUES ($1,$2,$3,'purchase_inbound',$4,'ERP-IN-CANCEL',1,$5,$6)")
            .bind(Uuid::new_v4()).bind(owner_id).bind(format!("ERP-IN-CANCEL-{line}"))
            .bind(warehouse_id).bind(line).bind(status).execute(&pool).await
            .expect("seed inbound ASN");
    }
    let service = app(pool.clone(), owner_id, api_key_id);
    let rejected = service
        .clone()
        .oneshot(cancel_request("ERP-IN-CANCEL", 1, 1, "cmd-in-reject"))
        .await
        .expect("inbound cancel should respond");
    assert_eq!(rejected.status(), StatusCode::OK);
    let evidence: (Vec<String>, Value) = sqlx::query_as(
        r#"SELECT ARRAY_AGG(o.status ORDER BY o.erp_line_no),
                  (SELECT f.payload FROM receiving_putaway_erp_feedback_outbox f
                    WHERE f.owner_id=$1 AND f.command_id='cmd-in-reject')
             FROM receiving_orders o
            WHERE o.owner_id=$1 AND o.erp_bill_code='ERP-IN-CANCEL'"#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load inbound cancel evidence");
    assert_eq!(evidence.0, vec!["released", "receiving"]);
    assert_eq!(evidence.1["result_code"], "INBOUND_RECEIPT_STARTED");

    let missing = service
        .oneshot(cancel_request("ERP-NOT-READY", 1, 2, "cmd-not-ready"))
        .await
        .expect("missing order should respond");
    assert_eq!(missing.status(), StatusCode::TOO_EARLY);
}

#[sqlx::test(migrations = "../../migrations")]
async fn waiting_cancel_blocks_wave_and_receipt_until_worker_retries(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = seed_context(&pool, owner_id, api_key_id).await;
    let service = app(pool.clone(), owner_id, api_key_id);
    for (code, order_type, command_id) in [
        ("ERP-WAIT-OUT", 2, "cmd-wait-out"),
        ("ERP-WAIT-IN", 1, "cmd-wait-in"),
    ] {
        let response = service
            .clone()
            .oneshot(cancel_request(code, 1, order_type, command_id))
            .await
            .expect("early cancel should respond");
        assert_eq!(response.status(), StatusCode::TOO_EARLY);
    }

    let outbound_id = Uuid::new_v4();
    sqlx::query("INSERT INTO outbound_orders (id,owner_id,wms_order_no,erp_bill_code,erp_revision,erp_order_type,customer_id,delivery_address_id,delivery_address_snapshot,warehouse_id,status) VALUES ($1,$2,'WMS-WAIT-OUT','ERP-WAIT-OUT',1,2,$3,$4,'{}'::jsonb,$5,'confirmed')")
        .bind(outbound_id).bind(owner_id).bind(Uuid::new_v4()).bind(Uuid::new_v4()).bind(warehouse_id)
        .execute(&pool).await.expect("seed waiting outbound");
    let receiving_id = Uuid::new_v4();
    sqlx::query("INSERT INTO receiving_orders (id,owner_id,receipt_no,document_type,warehouse_id,erp_bill_code,erp_revision,erp_line_no,status) VALUES ($1,$2,'WMS-WAIT-IN','purchase_inbound',$3,'ERP-WAIT-IN',1,1,'released')")
        .bind(receiving_id).bind(owner_id).bind(warehouse_id)
        .execute(&pool).await.expect("seed waiting inbound");
    let ctx = AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "cancel priority".to_string(),
        permissions: vec!["m2.write".to_string(), "m4.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    };

    let wave_error = PgWave4Repository::new(pool.clone())
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-MUST-WAIT".to_string(),
                order_ids: vec![outbound_id],
            },
            Utc::now(),
            "wave-must-wait",
            None,
        )
        .await
        .expect_err("pending ERP cancel must block wave entry");
    assert_eq!(wave_error, Wave4RepositoryError::PendingErpCancel);

    let receive_error = PgWave3Repository::new(pool)
        .receive_receiving_order_with_audit(
            &ctx,
            receiving_id,
            ReceiveReceivingOrderRequest {
                actual_qty: wms_domain::Quantity::ZERO,
                shortage_qty: wms_domain::Quantity::ZERO,
                rejected_qty: wms_domain::Quantity::ZERO,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: None,
            },
            Utc::now(),
            "receive-must-wait",
            None,
        )
        .await
        .expect_err("pending ERP cancel must block receipt start");
    assert_eq!(receive_error, Wave3RepositoryError::PendingErpCancel);
}
