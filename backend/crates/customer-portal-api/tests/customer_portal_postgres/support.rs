fn storage_root() -> PathBuf {
    std::env::temp_dir().join(format!("wms-customer-portal-{}", Uuid::new_v4()))
}

fn json_request(
    method: &str,
    path: &str,
    token: Option<&str>,
    payload: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    builder
        .body(Body::from(payload.to_string()))
        .expect("portal JSON request should build")
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("portal response should read"),
    )
    .expect("portal response should be JSON")
}

async fn assert_report_download(response: axum::response::Response, expected: &[u8]) {
    assert_eq!(response.status(), StatusCode::OK);
    let content_disposition = response
        .headers()
        .get(axum::http::header::CONTENT_DISPOSITION)
        .expect("content disposition should exist")
        .to_str()
        .expect("content disposition should be ASCII");
    assert!(
        content_disposition.starts_with("attachment; filename=\"report.pdf\"; filename*=UTF-8''")
    );
    assert!(content_disposition.ends_with("_P-001_BATCH-001_%E8%8D%AF%E6%A3%80%E5%8D%95_V1.pdf"));
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("file should read")
            .as_ref(),
        expected
    );
}

async fn project(app: &axum::Router, event_type: &str, payload: Value) -> Value {
    let (status, body) = project_event(app, Uuid::new_v4(), event_type, payload).await;
    assert_eq!(status, StatusCode::OK);
    body
}

async fn project_event(
    app: &axum::Router,
    event_id: Uuid,
    event_type: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/internal/projections")
                .header(CONTENT_TYPE, "application/json")
                .header("X-Projection-Key", PROJECTION_KEY)
                .body(Body::from(
                    json!({
                        "event_id": event_id,
                        "event_type": event_type,
                        "occurred_at": Utc::now(),
                        "payload": payload
                    })
                    .to_string(),
                ))
                .expect("projection request should build"),
        )
        .await
        .expect("projection should respond");
    let status = response.status();
    (status, response_json(response).await)
}

const PASSWORD: &str = "PortalTest1234";

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
        "address_snapshot": { "address_name": format!("{order_no} 收货地址") },
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
