use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};
use tower::ServiceExt;
use uuid::Uuid;
use wms_customer_portal_api::{export::process_next_export, portal_router, PortalState};

const JWT_SECRET: &str = "portal-test-jwt-secret-at-least-32-bytes";
const PROJECTION_KEY: &str = "portal-test-projection-key";
const PASSWORD: &str = "PortalTest1234";

include!("customer_portal_postgres/support.rs");

async fn seed_user(
    pool: &PgPool,
    customer_id: Uuid,
    username: &str,
    role: &str,
    history: bool,
    address_ids: &[Uuid],
) -> Uuid {
    let user_id = Uuid::new_v4();
    let password_hash = bcrypt::hash(PASSWORD, 4).expect("test password should hash");
    sqlx::query(
        "INSERT INTO portal_users (
            id, customer_id, username, display_name, password_hash,
            role, status, can_view_report_history
         )
         VALUES ($1, $2, $3, $3, $4, $5, 'active', $6)",
    )
    .bind(user_id)
    .bind(customer_id)
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(history)
    .execute(pool)
    .await
    .expect("portal user should seed");
    for address_id in address_ids {
        sqlx::query("INSERT INTO portal_user_addresses (user_id, address_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(address_id)
            .execute(pool)
            .await
            .expect("portal address scope should seed");
    }
    user_id
}

async fn login(app: &axum::Router, username: &str) -> String {
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            None,
            json!({ "username": username, "password": PASSWORD }),
        ))
        .await
        .expect("login should respond");
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await["access_token"]
        .as_str()
        .expect("access token should exist")
        .to_string()
}

fn order_payload(
    id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
    order_no: &str,
    status: &str,
    product_id: Uuid,
    batch_no: &str,
) -> Value {
    json!({
        "id": id,
        "customer_id": customer_id,
        "order_no": order_no,
        "status": status,
        "delivery_address_id": address_id,
        "address_snapshot": { "address": format!("{order_no} 收货地址") },
        "shipped_at": Utc::now() - Duration::hours(1),
        "signed_at": if status == "signed" { Some(Utc::now()) } else { None },
        "updated_at": Utc::now(),
        "lines": [{
            "id": Uuid::new_v4(),
            "product_id": product_id,
            "product_code": "P-001",
            "product_name": "真实药品",
            "batch_no": batch_no,
            "quantity": 12.0
        }]
    })
}

#[allow(clippy::too_many_arguments)]
fn report_payload(
    id: Uuid,
    report_id: Uuid,
    owner_id: Uuid,
    product_id: Uuid,
    version_number: i32,
    current: bool,
    copy_status: &str,
    storage_key: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "report_id": report_id,
        "owner_id": owner_id,
        "product_id": product_id,
        "batch_no": "BATCH-001",
        "version_number": version_number,
        "report_no": format!("REPORT-{version_number}"),
        "status": if current { "confirmed" } else { "superseded" },
        "is_current": current,
        "modification_reason": if version_number > 1 { Some("供应商更正") } else { None },
        "customer_copy_status": copy_status,
        "customer_copy_storage_key": storage_key,
        "customer_copy_file_name": storage_key.map(|_| format!("report-v{version_number}.pdf")),
        "customer_copy_size": storage_key.map(|_| 24_i64),
        "customer_copy_hash": storage_key.map(|_| format!("hash-{version_number}")),
        "digitally_signed_original": false,
        "confirmed_at": Utc::now() - Duration::minutes(3 - version_number as i64),
        "updated_at": Utc::now() + Duration::seconds(version_number as i64)
    })
}

async fn seed_customer_and_addresses(app: &axum::Router) -> (Uuid, Uuid, Uuid) {
    let customer_id = Uuid::new_v4();
    let address_a = Uuid::new_v4();
    let address_b = Uuid::new_v4();
    project(
        app,
        "customer.upsert",
        json!({
            "id": customer_id,
            "customer_code": "C-REAL",
            "customer_name": "真实客户",
            "updated_at": Utc::now()
        }),
    )
    .await;
    for (id, code, name) in [
        (address_a, "A-001", "一号门店"),
        (address_b, "A-002", "二号门店"),
    ] {
        project(
            app,
            "customer_address.upsert",
            json!({
                "id": id,
                "customer_id": customer_id,
                "address_code": code,
                "address_name": name,
                "address_snapshot": { "address": name },
                "updated_at": Utc::now()
            }),
        )
        .await;
    }
    (customer_id, address_a, address_b)
}

#[sqlx::test(migrations = "./migrations")]
async fn independent_projection_and_address_history_permissions_are_enforced(pool: PgPool) {
    let state = PortalState::new(pool.clone(), JWT_SECRET, PROJECTION_KEY, storage_root());
    let app = portal_router(state);
    let (customer_id, address_a, address_b) = seed_customer_and_addresses(&app).await;
    let owner_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order_a = Uuid::new_v4();
    let order_b = Uuid::new_v4();
    project(
        &app,
        "outbound_order.upsert",
        order_payload(
            order_a,
            customer_id,
            address_a,
            "SO-A",
            "shipped",
            product_id,
            "BATCH-001",
        ),
    )
    .await;
    project(
        &app,
        "outbound_order.upsert",
        order_payload(
            order_b,
            customer_id,
            address_b,
            "SO-B",
            "signed",
            product_id,
            "BATCH-001",
        ),
    )
    .await;
    project(
        &app,
        "outbound_order.upsert",
        order_payload(
            Uuid::new_v4(),
            customer_id,
            address_a,
            "SO-DRAFT",
            "draft",
            product_id,
            "BATCH-001",
        ),
    )
    .await;
    let report_id = Uuid::new_v4();
    project(
        &app,
        "drug_inspection_report.upsert",
        report_payload(
            Uuid::new_v4(),
            report_id,
            owner_id,
            product_id,
            1,
            false,
            "available",
            Some("wms-attachments/old.pdf"),
        ),
    )
    .await;
    project(
        &app,
        "drug_inspection_report.upsert",
        report_payload(
            Uuid::new_v4(),
            report_id,
            owner_id,
            product_id,
            2,
            true,
            "processing",
            None,
        ),
    )
    .await;

    seed_user(
        &pool,
        customer_id,
        "portal-admin",
        "customer_admin",
        false,
        &[],
    )
    .await;
    seed_user(
        &pool,
        customer_id,
        "address-a",
        "customer_user",
        false,
        &[address_a],
    )
    .await;
    seed_user(
        &pool,
        customer_id,
        "no-address",
        "customer_user",
        false,
        &[],
    )
    .await;
    seed_user(
        &pool,
        customer_id,
        "history-a",
        "customer_user",
        true,
        &[address_a],
    )
    .await;
    let admin_token = login(&app, "portal-admin").await;
    let address_token = login(&app, "address-a").await;
    let no_address_token = login(&app, "no-address").await;
    let history_token = login(&app, "history-a").await;

    for (token, expected) in [
        (&admin_token, 2_usize),
        (&address_token, 1),
        (&no_address_token, 0),
    ] {
        let response = app
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/v1/orders",
                Some(token),
                json!({}),
            ))
            .await
            .expect("orders should respond");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response_json(response)
                .await
                .as_array()
                .expect("orders should be array")
                .len(),
            expected
        );
    }
    let denied = app
        .clone()
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/orders/{order_b}"),
            Some(&address_token),
            json!({}),
        ))
        .await
        .expect("denied order should respond");
    assert_eq!(denied.status(), StatusCode::NOT_FOUND);

    let current_only = app
        .clone()
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/orders/{order_a}"),
            Some(&address_token),
            json!({}),
        ))
        .await
        .expect("current order should respond");
    assert_eq!(
        response_json(current_only).await["lines"][0]["reports"]
            .as_array()
            .expect("current reports should be array")
            .len(),
        1
    );
    let history = app
        .clone()
        .oneshot(json_request(
            "GET",
            &format!("/api/v1/orders/{order_a}"),
            Some(&history_token),
            json!({}),
        ))
        .await
        .expect("history order should respond");
    assert_eq!(
        response_json(history).await["lines"][0]["reports"]
            .as_array()
            .expect("history reports should be array")
            .len(),
        2
    );
    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM portal_audit_events WHERE action IN ('login', 'query', 'view')",
    )
    .fetch_one(&pool)
    .await
    .expect("audit count should read");
    assert!(audit_count >= 7);
}

#[sqlx::test(migrations = "./migrations")]
async fn projection_replay_is_idempotent_and_failures_enter_dead_letter(pool: PgPool) {
    let state = PortalState::new(pool.clone(), JWT_SECRET, PROJECTION_KEY, storage_root());
    let app = portal_router(state);
    let event_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let payload = json!({
        "id": customer_id,
        "customer_code": "C-IDEMPOTENT",
        "customer_name": "幂等投影客户",
        "updated_at": Utc::now()
    });

    let (first_status, first) =
        project_event(&app, event_id, "customer.upsert", payload.clone()).await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(first["duplicate"], false);
    let (replay_status, replay) = project_event(&app, event_id, "customer.upsert", payload).await;
    assert_eq!(replay_status, StatusCode::OK);
    assert_eq!(replay["duplicate"], true);
    let replay_state: (String, i32, i64) = sqlx::query_as(
        "SELECT event.status, event.attempt_count,
                (SELECT COUNT(*) FROM portal_customers WHERE id = $2)
           FROM portal_projection_events AS event
          WHERE event.event_id = $1",
    )
    .bind(event_id)
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("replayed projection state should query");
    assert_eq!(replay_state, ("succeeded".to_string(), 1, 1));

    let failed_event_id = Uuid::new_v4();
    for attempt in 1..=5 {
        let (status, _) = project_event(
            &app,
            failed_event_id,
            "unsupported.event",
            json!({ "attempt": attempt }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let state: (String, i32, bool) = sqlx::query_as(
            "SELECT status, attempt_count, next_attempt_at IS NOT NULL
               FROM portal_projection_events
              WHERE event_id = $1",
        )
        .bind(failed_event_id)
        .fetch_one(&pool)
        .await
        .expect("failed projection state should query");
        assert_eq!(state.1, attempt);
        if attempt < 5 {
            assert_eq!(state.0, "failed");
            assert!(state.2);
        } else {
            assert_eq!(state.0, "dead_letter");
            assert!(!state.2);
        }
    }
    let (dead_letter_status, _) = project_event(
        &app,
        failed_event_id,
        "unsupported.event",
        json!({ "attempt": 6 }),
    )
    .await;
    assert_eq!(dead_letter_status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn real_download_and_zip_are_scoped_deduplicated_and_expiring(pool: PgPool) {
    let root = storage_root();
    std::fs::create_dir_all(root.join("wms-attachments"))
        .expect("attachment directory should create");
    let original = b"%PDF-1.4 portal real customer copy";
    std::fs::write(root.join("wms-attachments/report.pdf"), original)
        .expect("real report file should write");
    let state = PortalState::new(pool.clone(), JWT_SECRET, PROJECTION_KEY, root);
    let app = portal_router(state.clone());
    let (customer_id, address_a, _) = seed_customer_and_addresses(&app).await;
    let owner_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let available_order = Uuid::new_v4();
    let duplicate_order = Uuid::new_v4();
    for (id, order_no) in [(available_order, "SO-ZIP-1"), (duplicate_order, "SO-ZIP-2")] {
        project(
            &app,
            "outbound_order.upsert",
            order_payload(
                id,
                customer_id,
                address_a,
                order_no,
                "shipped",
                product_id,
                "BATCH-001",
            ),
        )
        .await;
    }
    let missing_product = Uuid::new_v4();
    let missing_order = Uuid::new_v4();
    project(
        &app,
        "outbound_order.upsert",
        order_payload(
            missing_order,
            customer_id,
            address_a,
            "SO-MISSING",
            "signed",
            missing_product,
            "BATCH-MISSING",
        ),
    )
    .await;
    let report_version_id = Uuid::new_v4();
    project(
        &app,
        "drug_inspection_report.upsert",
        report_payload(
            report_version_id,
            Uuid::new_v4(),
            owner_id,
            product_id,
            1,
            true,
            "available",
            Some("wms-attachments/report.pdf"),
        ),
    )
    .await;
    seed_user(
        &pool,
        customer_id,
        "zip-user",
        "customer_user",
        false,
        &[address_a],
    )
    .await;
    let token = login(&app, "zip-user").await;

    let authorized = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/report-versions/{report_version_id}/download"),
            Some(&token),
            json!({}),
        ))
        .await
        .expect("report download authorization should respond");
    assert_eq!(authorized.status(), StatusCode::OK);
    let authorized = response_json(authorized).await;
    let expected_report_file_name = "真实药品_P-001_BATCH-001_药检单_V1.pdf";
    assert_eq!(authorized["file_name"], expected_report_file_name);
    let expires_at = authorized["expires_at"]
        .as_str()
        .expect("download expiry should exist")
        .parse::<chrono::DateTime<Utc>>()
        .expect("download expiry should parse");
    assert!(expires_at > Utc::now() + Duration::minutes(14));
    assert!(expires_at < Utc::now() + Duration::minutes(16));
    let file_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    authorized["url"]
                        .as_str()
                        .expect("download URL should exist"),
                )
                .body(Body::empty())
                .expect("file request should build"),
        )
        .await
        .expect("file should respond");
    assert_report_download(file_response, original).await;

    let export_response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/exports",
            Some(&token),
            json!({
                "order_ids": [available_order, duplicate_order, missing_order],
                "include_history": false
            }),
        ))
        .await
        .expect("export create should respond");
    let export_status = export_response.status();
    let export_body = response_json(export_response).await;
    assert_eq!(
        export_status,
        StatusCode::OK,
        "export response: {export_body}"
    );
    let export_id = export_body["id"]
        .as_str()
        .expect("export id should exist")
        .to_string();
    assert!(process_next_export(&state)
        .await
        .expect("export worker should succeed"));
    let exports = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/exports",
            Some(&token),
            json!({}),
        ))
        .await
        .expect("exports should respond");
    let exports = response_json(exports).await;
    assert_eq!(exports[0]["status"], "completed");
    assert_eq!(exports[0]["report_file_count"], 1);
    assert_eq!(exports[0]["missing_count"], 1);
    let export_expiry = exports[0]["expires_at"]
        .as_str()
        .expect("export expiry should exist")
        .parse::<chrono::DateTime<Utc>>()
        .expect("export expiry should parse");
    assert!(export_expiry > Utc::now() + Duration::days(6));

    let export_download = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/exports/{export_id}/download"),
            Some(&token),
            json!({}),
        ))
        .await
        .expect("export download authorization should respond");
    let export_download = response_json(export_download).await;
    let zip_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    export_download["url"]
                        .as_str()
                        .expect("ZIP URL should exist"),
                )
                .body(Body::empty())
                .expect("ZIP request should build"),
        )
        .await
        .expect("ZIP should respond");
    let zip_bytes = to_bytes(zip_response.into_body(), usize::MAX)
        .await
        .expect("ZIP should read");
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes)).expect("real ZIP should open");
    let report_entries = archive
        .file_names()
        .filter(|name| name.starts_with("reports/"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        report_entries,
        vec![format!("reports/{expected_report_file_name}")]
    );
    let mut manifest = String::new();
    archive
        .by_name("药检单清单.csv")
        .expect("manifest should exist")
        .read_to_string(&mut manifest)
        .expect("manifest should read");
    assert!(manifest.contains("SO-MISSING"));
    assert!(manifest.contains("资料暂缺"));

    sqlx::query(
        "UPDATE portal_report_versions
         SET customer_copy_size = $2
         WHERE id = $1",
    )
    .bind(report_version_id)
    .bind(wms_customer_portal_api::export::MAX_EXPORT_BYTES + 1)
    .execute(&pool)
    .await
    .expect("oversize metadata should update");
    let oversized = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/exports",
            Some(&token),
            json!({ "order_ids": [available_order], "include_history": false }),
        ))
        .await
        .expect("oversize export should respond");
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);

    sqlx::query(
        "UPDATE portal_report_versions
         SET customer_copy_size = 24
         WHERE id = $1",
    )
    .bind(report_version_id)
    .execute(&pool)
    .await
    .expect("oversize metadata should reset");
    sqlx::query(
        "INSERT INTO portal_report_versions (
            id, report_id, owner_id, product_id, batch_no, version_number,
            report_no, status, is_current, customer_copy_status,
            customer_copy_storage_key, customer_copy_file_name,
            customer_copy_size, customer_copy_hash, confirmed_at, updated_at
         )
         SELECT gen_random_uuid(), gen_random_uuid(), $1, $2, 'BATCH-001', 1,
                'REPORT-LIMIT-' || series, 'confirmed', TRUE, 'available',
                'wms-attachments/report.pdf', 'report.pdf', 24,
                'hash-limit-' || series, now(), now()
           FROM generate_series(1, 200) AS series",
    )
    .bind(owner_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .expect("file-count boundary data should seed");
    let too_many = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/exports",
            Some(&token),
            json!({ "order_ids": [available_order], "include_history": false }),
        ))
        .await
        .expect("file-count export should respond");
    assert_eq!(too_many.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(too_many).await["message"]
        .as_str()
        .is_some_and(|message| message.contains("最多 200 份")));
}

#[sqlx::test(migrations = "./migrations")]
async fn customer_admin_can_update_account_scope_history_and_status(pool: PgPool) {
    let app = portal_router(PortalState::new(
        pool.clone(),
        JWT_SECRET,
        PROJECTION_KEY,
        storage_root(),
    ));
    let (customer_id, address_id, _) = seed_customer_and_addresses(&app).await;
    seed_user(
        &pool,
        customer_id,
        "account-admin",
        "customer_admin",
        false,
        &[],
    )
    .await;
    let admin_token = login(&app, "account-admin").await;
    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            Some(&admin_token),
            json!({
                "username": "managed-user",
                "display_name": "受管账号",
                "password": PASSWORD,
                "role": "customer_user",
                "can_view_report_history": false,
                "address_ids": [address_id]
            }),
        ))
        .await
        .expect("create user should respond");
    assert_eq!(created.status(), StatusCode::OK);
    let created = response_json(created).await;
    let user_id = created["id"]
        .as_str()
        .expect("created user id should exist");
    let updated = app
        .clone()
        .oneshot(json_request(
            "PUT",
            &format!("/api/v1/users/{user_id}"),
            Some(&admin_token),
            json!({
                "display_name": "受管账号（停用）",
                "role": "customer_user",
                "status": "disabled",
                "can_view_report_history": true,
                "address_ids": [address_id]
            }),
        ))
        .await
        .expect("update user should respond");
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(response_json(updated).await["status"], "disabled");
    let denied = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            None,
            json!({ "username": "managed-user", "password": PASSWORD }),
        ))
        .await
        .expect("disabled login should respond");
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
}
