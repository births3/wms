#[tokio::test]
async fn h3_resilience_limits_user_and_api_key_independently() {
    let state = wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
        global_qps: 100,
        global_burst: 100,
        user_qps: 1,
        user_burst: 1,
        api_key_qps: 1,
        api_key_burst: 1,
        retry_after_seconds: 1,
        circuit_failures: 10,
        circuit_open_seconds: 30,
    });
    let app = Router::new()
        .route("/limited", get(healthz))
        .layer(from_fn_with_state(
            state,
            wms_api::resilience::resilience_middleware,
        ));
    let owner_id = Uuid::new_v4();
    let user_one = bearer_token(owner_id);
    let user_two = bearer_token(owner_id);

    let first_user_one = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("authorization", format!("Bearer {user_one}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first_user_one.status(), StatusCode::OK);

    let second_user_one = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("authorization", format!("Bearer {user_one}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(second_user_one.status(), StatusCode::TOO_MANY_REQUESTS);

    let first_user_two = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("authorization", format!("Bearer {user_two}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first_user_two.status(), StatusCode::OK);

    let first_api_key = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("x-wms-api-key", "external-key-a")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(first_api_key.status(), StatusCode::OK);

    let second_api_key = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("x-wms-api-key", "external-key-a")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(second_api_key.status(), StatusCode::TOO_MANY_REQUESTS);

    let other_api_key = app
        .oneshot(
            Request::builder()
                .uri("/limited")
                .header("x-wms-api-key", "external-key-b")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(other_api_key.status(), StatusCode::OK);
}

#[tokio::test]
async fn h3_resilience_metrics_expose_rate_limit_and_degraded_counters() {
    async fn failing() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let state = wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
        global_qps: 1,
        global_burst: 1,
        user_qps: 100,
        user_burst: 100,
        api_key_qps: 100,
        api_key_burst: 100,
        retry_after_seconds: 1,
        circuit_failures: 1,
        circuit_open_seconds: 30,
    });
    let app = Router::new()
        .route("/limited", get(healthz))
        .route("/dependency", get(failing))
        .route("/metrics", get(wms_api::resilience::resilience_metrics))
        .with_state(state.clone())
        .layer(from_fn_with_state(
            state.clone(),
            wms_api::resilience::resilience_middleware,
        ));

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    let limited = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/limited")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

    state.reset_rate_limit_for_test();
    let failed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let degraded = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(degraded.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        degraded
            .headers()
            .get("x-wms-degraded-response")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );

    let metrics = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(metrics.status(), StatusCode::OK);
    let body = to_bytes(metrics.into_body(), usize::MAX)
        .await
        .expect("metrics body should read");
    let body = String::from_utf8(body.to_vec()).expect("metrics should be utf8");
    assert!(body.contains("wms_h3_rate_limit_rejected_total 1"));
    assert!(body.contains("wms_h3_circuit_opened_total 1"));
    assert!(body.contains("wms_h3_degraded_responses_total 1"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h3_resilience_rejections_write_h2_audit_for_bearer_actor(pool: PgPool) {
    let state = wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
        global_qps: 1,
        global_burst: 1,
        user_qps: 100,
        user_burst: 100,
        api_key_qps: 100,
        api_key_burst: 100,
        retry_after_seconds: 1,
        circuit_failures: 10,
        circuit_open_seconds: 30,
    })
    .with_audit_pool(pool.clone());
    let app = Router::new()
        .route("/limited", get(healthz))
        .layer(from_fn_with_state(
            state,
            wms_api::resilience::resilience_middleware,
        ));
    let owner_id = Uuid::new_v4();
    let token = bearer_token(owner_id);

    for expected in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), expected);
    }

    let row: (i64, Option<String>, Option<String>) = sqlx::query_as(
        r#"
            SELECT COUNT(*), MIN(action), MIN(actor_name)
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
            "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit row should query");
    assert_eq!(row.0, 1);
    assert_eq!(row.1.as_deref(), Some("h3.rate_limited"));
    assert_eq!(row.2.as_deref(), Some("audit-reader"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h3_resilience_rejections_write_h2_audit_for_api_key(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let state = wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
        global_qps: 100,
        global_burst: 100,
        user_qps: 100,
        user_burst: 100,
        api_key_qps: 1,
        api_key_burst: 1,
        retry_after_seconds: 1,
        circuit_failures: 10,
        circuit_open_seconds: 30,
    })
    .with_api_key_audit_owner_id(owner_id)
    .with_audit_pool(pool.clone());
    let app = Router::new()
        .route("/limited", get(healthz))
        .layer(from_fn_with_state(
            state,
            wms_api::resilience::resilience_middleware,
        ));

    for expected in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/limited")
                    .header("x-wms-api-key", "external-key-a")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), expected);
    }

    let row: (i64, Option<String>, Option<String>) = sqlx::query_as(
        r#"
            SELECT COUNT(*), MIN(action), MIN(actor_name)
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
            "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit row should query");
    assert_eq!(row.0, 1);
    assert_eq!(row.1.as_deref(), Some("h3.rate_limited"));
    assert!(row
        .2
        .as_deref()
        .is_some_and(|actor_name| actor_name.starts_with("api-key:")));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h3_resilience_circuit_events_write_h2_audit(pool: PgPool) {
    async fn failing() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let state = wms_api::resilience::ResilienceState::new(wms_api::resilience::ResilienceConfig {
        global_qps: 100,
        global_burst: 100,
        user_qps: 100,
        user_burst: 100,
        api_key_qps: 100,
        api_key_burst: 100,
        retry_after_seconds: 1,
        circuit_failures: 1,
        circuit_open_seconds: 30,
    })
    .with_audit_pool(pool.clone());
    let app = Router::new()
        .route("/dependency", get(failing))
        .layer(from_fn_with_state(
            state,
            wms_api::resilience::resilience_middleware,
        ));
    let owner_id = Uuid::new_v4();
    let token = bearer_token(owner_id);

    let failed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let degraded = app
        .oneshot(
            Request::builder()
                .uri("/dependency")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(degraded.status(), StatusCode::SERVICE_UNAVAILABLE);

    let actions: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT action
              FROM audit_event
             WHERE owner_id = $1
               AND module = 'H3'
               AND resource_type = 'api_resilience'
             ORDER BY action
            "#,
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("audit actions should query");
    assert_eq!(
        actions,
        vec![
            "h3.circuit_degraded".to_string(),
            "h3.circuit_opened".to_string()
        ]
    );
}

#[tokio::test]
async fn h3_docs_routes_follow_environment_mode() {
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/wms")
        .expect("lazy pool should not connect during docs mode test");
    let app = with_env_lock(|| {
        std::env::set_var("WMS_API_DOCS_MODE", "production");
        let app = app(
            config_center_state(),
            AuthAppState::new(pool.clone()),
            Wave3AppState::default(),
            Wave4AppState::with_postgres(pool.clone()),
            Wave5AppState::with_postgres(pool.clone()),
            ExpressAppState::with_postgres(pool.clone()),
            AuditQueryState { pool: pool.clone() },
            MasterDataAppState::default(),
            SystemDictionaryAppState::with_postgres(pool),
        );
        std::env::remove_var("WMS_API_DOCS_MODE");
        app
    });

    let swagger = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api-docs")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(swagger.status(), StatusCode::NOT_FOUND);

    let blocked_redoc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/redoc")
                .header("x-forwarded-for", "8.8.8.8")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(blocked_redoc.status(), StatusCode::FORBIDDEN);

    let redoc_without_forwarded_ip = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/redoc")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(redoc_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

    let metrics_without_forwarded_ip = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(metrics_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

    let openapi_without_forwarded_ip = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(openapi_without_forwarded_ip.status(), StatusCode::FORBIDDEN);

    let redoc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/redoc")
                .header("x-forwarded-for", "10.0.0.8")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(redoc.status(), StatusCode::OK);
    let body = to_bytes(redoc.into_body(), usize::MAX)
        .await
        .expect("redoc body should read");
    let body = String::from_utf8(body.to_vec()).expect("redoc should be utf8");
    assert!(body.contains("redoc"));

    let metrics = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("x-forwarded-for", "10.0.0.8")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(metrics.status(), StatusCode::OK);
}
