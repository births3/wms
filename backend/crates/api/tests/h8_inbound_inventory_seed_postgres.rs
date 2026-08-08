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
};

#[sqlx::test(migrations = "../../migrations")]
async fn initial_inventory_snapshot_only_enters_approval_staging(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let warehouse_code = format!("H8-SEED-{}", &warehouse_id.to_string()[..8]);
    sqlx::query("INSERT INTO auth_owners (id,owner_code,owner_name) VALUES ($1,$2,'seed owner')")
        .bind(owner_id)
        .bind(format!("SEED-{}", &owner_id.to_string()[..8]))
        .execute(&pool)
        .await
        .expect("seed inventory owner");
    sqlx::query("INSERT INTO warehouses (id,owner_id,warehouse_code,warehouse_name,warehouse_type,status) VALUES ($1,$2,$3,'seed warehouse','normal','active')")
        .bind(warehouse_id).bind(owner_id).bind(&warehouse_code).execute(&pool).await
        .expect("seed inventory warehouse");
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors (
            id,owner_id,connector_code,connector_name,warehouse_ids,directions,message_types,
            channel_mode,api_key_id,status,config_version,first_activated_at,last_tested_version,
            last_tested_at,last_tested_succeeded
        ) VALUES ($1,$2,'H8-SEED','H8 seed',ARRAY[$3]::uuid[],ARRAY['inbound'],
            ARRAY['inventory_seed_snapshot'],'rest',$4,'active',1,now(),1,now(),TRUE)"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(api_key_id)
    .execute(&pool)
    .await
    .expect("seed inventory connector");

    let snapshot_id = format!("SNP-{}", &Uuid::new_v4().to_string()[..8]);
    let body = json!({
        "schema_version": "1",
        "external_ref": snapshot_id,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": null,
        "snapshot_id": snapshot_id,
        "depot_code": warehouse_code,
        "push_type": 1,
        "push_time": Utc::now(),
        "items": [{
            "row_no": 1,
            "product_code": "P-SEED-001",
            "batch_no": "B-SEED-001",
            "expiry_date": "2028-08-05",
            "location_code": null,
            "goods_status": null,
            "quantity": "50.5000"
        }]
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/integration/erp-messages/inbound/inventory_seed_snapshot")
        .header("content-type", "application/json")
        .header("Idempotency-Key", &snapshot_id)
        .body(Body::from(body.to_string()))
        .expect("inventory seed request should build");
    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(AuthContext {
            user_id: api_key_id,
            owner_id,
            actor_name: "H8 seed API Key".to_string(),
            permissions: vec!["m3.write".to_string()],
            jti: format!("api-key:{api_key_id}"),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request)
        .await
        .expect("inventory seed should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let evidence: (String, i64, i64, Value) = sqlx::query_as(
        r#"SELECT status,
              (SELECT COUNT(*) FROM erp_inventory_snapshot_staging_items i WHERE i.snapshot_staging_id=h.id),
              (SELECT COUNT(*) FROM inventory_batches b WHERE b.owner_id=h.owner_id),
              summary
           FROM erp_inventory_snapshot_staging h
          WHERE owner_id=$1 AND snapshot_id=$2"#,
    )
    .bind(owner_id)
    .bind(&snapshot_id)
    .fetch_one(&pool)
    .await
    .expect("inventory snapshot staging should persist");
    assert_eq!(evidence.0, "pending_approval");
    assert_eq!(evidence.1, 1);
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3["quarantined_items"], 1);
}
