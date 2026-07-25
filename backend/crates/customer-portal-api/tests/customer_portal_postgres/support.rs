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
