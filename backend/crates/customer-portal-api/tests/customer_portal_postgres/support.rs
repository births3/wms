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
