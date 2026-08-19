#[test]
fn database_max_connections_defaults_and_rejects_invalid_values() {
    std::env::remove_var(DB_MAX_CONNECTIONS_ENV);
    assert_eq!(
        database_max_connections().expect("default max connections"),
        DEFAULT_DB_MAX_CONNECTIONS
    );

    std::env::set_var(DB_MAX_CONNECTIONS_ENV, "64");
    assert_eq!(
        database_max_connections().expect("configured max connections"),
        64
    );

    std::env::set_var(DB_MAX_CONNECTIONS_ENV, "0");
    assert!(database_max_connections().is_err());

    std::env::set_var(DB_MAX_CONNECTIONS_ENV, "not-a-number");
    assert!(database_max_connections().is_err());

    std::env::remove_var(DB_MAX_CONNECTIONS_ENV);
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_events_query_filters_by_auth_owner(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let actor_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();
    append_event(
        &pool,
        &AuditWriteRequest {
            occurred_at: Utc::now(),
            actor_id,
            actor_name: "owner-a-user".to_string(),
            owner_id,
            jti: "owner-a-jti".to_string(),
            action: "receive".to_string(),
            module: "M2".to_string(),
            resource_type: "receiving_order".to_string(),
            resource_id: "ASN-001".to_string(),
            diff: None,
            request_id: Some(request_id),
            ip: None,
            user_agent: None,
        },
    )
    .await
    .expect("owner audit should insert");
    append_event(
        &pool,
        &AuditWriteRequest {
            occurred_at: Utc::now(),
            actor_id,
            actor_name: "owner-a-user".to_string(),
            owner_id,
            jti: "owner-a-second-jti".to_string(),
            action: "putaway".to_string(),
            module: "M2".to_string(),
            resource_type: "putaway".to_string(),
            resource_id: "PUT-001".to_string(),
            diff: None,
            request_id: None,
            ip: None,
            user_agent: None,
        },
    )
    .await
    .expect("second owner audit should insert");
    append_event(
        &pool,
        &AuditWriteRequest {
            occurred_at: Utc::now(),
            actor_id: Uuid::new_v4(),
            actor_name: "other-owner-user".to_string(),
            owner_id: other_owner_id,
            jti: "other-owner-jti".to_string(),
            action: "receive".to_string(),
            module: "M2".to_string(),
            resource_type: "receiving_order".to_string(),
            resource_id: "ASN-002".to_string(),
            diff: None,
            request_id: None,
            ip: None,
            user_agent: None,
        },
    )
    .await
    .expect("other owner audit should insert");
    let app = app(
        config_center_state(),
        AuthAppState::new(pool.clone()),
        Wave3AppState::default(),
        Wave4AppState::with_postgres(pool.clone()),
        Wave5AppState::with_postgres(pool.clone()),
        ExpressAppState::with_postgres(pool.clone()),
        AuditQueryState { pool: pool.clone() },
        MasterDataAppState::default(),
        SystemDictionaryAppState::with_postgres(pool.clone()),
        None,
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/events?resource_type=receiving_order&limit=10")
                .header(
                    "authorization",
                    format!("Bearer {}", bearer_token(owner_id)),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let payload: AuditEventListResponse =
        serde_json::from_slice(&body).expect("response should be json");
    assert_eq!(payload.data.len(), 1);
    let event = &payload.data[0];
    assert_eq!(event.owner_id, owner_id);
    assert_eq!(event.actor.actor_id, actor_id);
    assert_eq!(event.actor.jti, "owner-a-jti");
    assert_eq!(event.resource_id, "ASN-001");
    assert_eq!(event.trace_id, request_id.to_string());
    assert_eq!(event.diff, serde_json::json!({}));

    let page_one = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/events?limit=1")
                .header(
                    "authorization",
                    format!("Bearer {}", bearer_token(owner_id)),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(page_one.status(), StatusCode::OK);
    let body = axum::body::to_bytes(page_one.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let page_one: AuditEventListResponse =
        serde_json::from_slice(&body).expect("response should be json");
    assert_eq!(page_one.data.len(), 1);
    let first_page_id = page_one.data[0].id;
    let cursor = page_one
        .next_cursor
        .expect("first page should include next cursor");

    let page_two = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/audit/events?limit=1&cursor={cursor}"))
                .header(
                    "authorization",
                    format!("Bearer {}", bearer_token(owner_id)),
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(page_two.status(), StatusCode::OK);
    let body = axum::body::to_bytes(page_two.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let page_two: AuditEventListResponse =
        serde_json::from_slice(&body).expect("response should be json");
    assert_eq!(page_two.data.len(), 1);
    assert_ne!(page_two.data[0].id, first_page_id);
    assert!(page_two.next_cursor.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn auth_login_issues_token_and_me_returns_current_user(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    seed_auth_user(&pool, owner_id, user_id, role_id).await;
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
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
        None,
    )
    .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
        AllowAllRevocationStore,
    ))));

    let login_request = LoginRequest {
        owner_code: "PY_OWNER".to_string(),
        username: "admin".to_string(),
        password: "CorrectHorse1!".to_string(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&login_request).expect("login request should encode"),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let login: LoginResponse =
        serde_json::from_slice(&body).expect("login response should be json");
    assert_eq!(login.token_type, "Bearer");
    assert_eq!(login.user.user_id, user_id);
    assert_eq!(login.user.owner_id, owner_id);
    assert_eq!(login.user.owner_code, "PY_OWNER");
    assert_eq!(login.user.username, "admin");
    assert_eq!(login.user.roles, vec!["audit_reader"]);
    assert_eq!(login.user.permissions, vec!["audit.read"]);

    let me_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/me")
                .header("authorization", format!("Bearer {}", login.access_token))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(me_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(me_response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let current_user: CurrentUser =
        serde_json::from_slice(&body).expect("current user response should be json");
    assert_eq!(current_user.user_id, user_id);
    assert_eq!(current_user.owner_id, owner_id);
    assert_eq!(current_user.owner_code, "PY_OWNER");
    assert_eq!(current_user.roles, vec!["audit_reader"]);
    assert_eq!(current_user.permissions, vec!["audit.read"]);
}
