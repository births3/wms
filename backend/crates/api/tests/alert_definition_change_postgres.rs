use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use std::collections::BTreeMap;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    alert_definition_handlers::{alert_definition_router, AlertDefinitionAppState},
    alert_definition_repository::{AlertDefinitionRepositoryError, PgAlertDefinitionRepository},
    alert_definition_service::{AlertDefinitionService, AlertDefinitionServiceError},
    auth::AuthContext,
    quality_liaison::{PgQualityLiaisonRepository, QualityLiaisonError},
};
use wms_domain::{
    AlertDefinitionChangeOperation, AlertDefinitionDraft, AlertDefinitionListResponse,
    ErrorResponse, QualityLiaisonApprovalCallbackRequest, QualityLiaisonOrder,
    SubmitAlertDefinitionChangeRequest, UpsertQualityLiaisonTypeRequest,
};

fn ctx(owner_id: Uuid, user_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "alert-definition-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_owner_and_users(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '告警定义测试货主')",
    )
    .bind(owner_id)
    .bind(format!("AL-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("alert definition owner should seed");
    for (user_id, label) in [(manager_id, "manager"), (approver_id, "approver")] {
        sqlx::query(
            "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $2, 'test-hash', 'active')",
        )
        .bind(user_id)
        .bind(format!("al-{label}-{}", &user_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("alert definition actor should seed");
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
        )
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("alert definition owner binding should seed");
    }
    (owner_id, manager_id, approver_id)
}

async fn configure_approval(pool: &PgPool, owner_id: Uuid, manager_id: Uuid, approver_id: Uuid) {
    PgQualityLiaisonRepository::new(pool.clone())
        .upsert_type(
            &ctx(owner_id, manager_id, &["mql.quality-liaison.config"]),
            UpsertQualityLiaisonTypeRequest {
                type_code: "alert_definition_change".to_string(),
                type_name: "告警定义变更".to_string(),
                approval_template_id: "ww-alert-definition-change".to_string(),
                approver_user_id: approver_id,
                timeout_seconds: 4 * 60 * 60,
                enabled: true,
            },
            Utc::now(),
            "alert-definition-approval-type",
        )
        .await
        .expect("alert definition approval type should configure");
}

async fn configure_notification_channel(pool: &PgPool, owner_id: Uuid, manager_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO h4_notification_configs (
            id, owner_id, event_type, enabled, template, recipient_rule,
            channels, created_by, updated_by
        ) VALUES (
            $1, $2, 'business.inventory.changed', TRUE,
            '库存低于安全阈值：{{product_code}}',
            '{"roles":["warehouse_manager"]}'::jsonb,
            ARRAY['wechat']::text[], $3, $3
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("alert definition H4 channel should configure");
}

fn draft() -> AlertDefinitionDraft {
    AlertDefinitionDraft {
        alert_code: "inventory.low".to_string(),
        name: "库存低于安全阈值".to_string(),
        event_type: "business.inventory.changed".to_string(),
        condition_expression:
            "{\"field\":\"quantity\",\"op\":\"lt\",\"value_field\":\"safety_stock\"}".to_string(),
        default_severity: "warning".to_string(),
        recipient_roles: vec!["warehouse_manager".to_string()],
        escalation_ref: None,
        silence_period_seconds: 300,
        is_disable_allowed: true,
        message_template: "库存低于安全阈值：{{product_code}}".to_string(),
        message_templates: BTreeMap::from([
            (
                "zh-CN".to_string(),
                "库存低于安全阈值：{{product_code}}".to_string(),
            ),
            (
                "en-US".to_string(),
                "Inventory below safety stock: {{product_code}}".to_string(),
            ),
        ]),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_condition_is_rejected_before_creating_liaison(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    let app = alert_definition_router(AlertDefinitionAppState::with_postgres(pool.clone()));
    let mut invalid = draft();
    invalid.condition_expression = "quantity < 10".to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", "alert-definition-invalid-condition")
                .extension(ctx(owner_id, manager_id, &["hal.alert-definition.write"]))
                .body(Body::from(
                    serde_json::to_vec(&SubmitAlertDefinitionChangeRequest {
                        operation: AlertDefinitionChangeOperation::Upsert,
                        definition_id: None,
                        expected_version: None,
                        definition: Some(invalid),
                        enabled: None,
                    })
                    .expect("invalid condition request should serialize"),
                ))
                .expect("invalid condition request should build"),
        )
        .await
        .expect("invalid condition request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("invalid condition response should read"),
    )
    .expect("invalid condition error should deserialize");
    assert_eq!(error.code, "HAL_CONDITION_INVALID");
    let liaison_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM quality_liaison_orders WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("quality liaison count should query");
    assert_eq!(liaison_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_h4_channel_is_rejected_before_creating_liaison(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    let app = alert_definition_router(AlertDefinitionAppState::with_postgres(pool.clone()));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", "alert-definition-missing-channel")
                .extension(ctx(owner_id, manager_id, &["hal.alert-definition.write"]))
                .body(Body::from(
                    serde_json::to_vec(&SubmitAlertDefinitionChangeRequest {
                        operation: AlertDefinitionChangeOperation::Upsert,
                        definition_id: None,
                        expected_version: None,
                        definition: Some(draft()),
                        enabled: None,
                    })
                    .expect("missing channel request should serialize"),
                ))
                .expect("missing channel request should build"),
        )
        .await
        .expect("missing channel request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("missing channel response should read"),
    )
    .expect("missing channel error should deserialize");
    assert_eq!(error.code, "HAL_CHANNEL_NOT_FOUND");
    let liaison_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM quality_liaison_orders WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("quality liaison count should query");
    assert_eq!(liaison_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_escalation_rule_is_rejected_before_creating_liaison(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    configure_notification_channel(&pool, owner_id, manager_id).await;
    let mut definition = draft();
    definition.escalation_ref = Some("missing-rule".to_string());
    let error = AlertDefinitionService::new(pool.clone())
        .submit_change(
            &ctx(owner_id, manager_id, &["hal.alert-definition.write"]),
            SubmitAlertDefinitionChangeRequest {
                operation: AlertDefinitionChangeOperation::Upsert,
                definition_id: None,
                expected_version: None,
                definition: Some(definition),
                enabled: None,
            },
            Utc::now(),
            "alert-definition-missing-escalation",
        )
        .await
        .expect_err("missing escalation rule should reject before approval");
    assert_eq!(
        error,
        AlertDefinitionServiceError::Definition(
            AlertDefinitionRepositoryError::EscalationRuleNotFound
        )
    );
    let liaison_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM quality_liaison_orders WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("liaison count should query");
    assert_eq!(liaison_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn approved_business_definition_can_be_edited_disabled_and_deleted(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    configure_notification_channel(&pool, owner_id, manager_id).await;
    let manager = ctx(
        owner_id,
        manager_id,
        &["hal.alert-definition.read", "hal.alert-definition.write"],
    );
    let approver = ctx(owner_id, approver_id, &["mql.quality-liaison.approve"]);
    let app = alert_definition_router(AlertDefinitionAppState::with_postgres(pool.clone()));
    let repo = PgAlertDefinitionRepository::new(pool.clone());

    let mut create_draft = draft();
    create_draft.condition_expression.clear();
    let create = submit_change(
        &app,
        &manager,
        SubmitAlertDefinitionChangeRequest {
            operation: AlertDefinitionChangeOperation::Upsert,
            definition_id: None,
            expected_version: None,
            definition: Some(create_draft),
            enabled: None,
        },
        "alert-definition-lifecycle-create",
    )
    .await;
    approve_change(&pool, &approver, create.id, "alert-lifecycle-create").await;
    let created = repo
        .list(owner_id, &Default::default())
        .await
        .expect("created definition should list")
        .into_iter()
        .find(|row| row.alert_code == "inventory.low")
        .expect("created definition should exist");
    assert_eq!(created.condition_expression, "{}");
    assert_eq!(created.silence_period_seconds, 5 * 60);
    assert_eq!(
        created.message_templates.get("en-US").map(String::as_str),
        Some("Inventory below safety stock: {{product_code}}")
    );

    let mut edited_draft = draft();
    edited_draft.name = "库存低于补货阈值".to_string();
    let edit = submit_change(
        &app,
        &manager,
        SubmitAlertDefinitionChangeRequest {
            operation: AlertDefinitionChangeOperation::Upsert,
            definition_id: Some(created.id),
            expected_version: Some(created.version),
            definition: Some(edited_draft),
            enabled: None,
        },
        "alert-definition-lifecycle-edit",
    )
    .await;
    approve_change(&pool, &approver, edit.id, "alert-lifecycle-edit").await;
    let edited = repo
        .get(owner_id, created.id)
        .await
        .expect("edited definition should read");
    assert_eq!(edited.name, "库存低于补货阈值");
    assert_eq!(edited.version, 2);

    let disable = submit_change(
        &app,
        &manager,
        SubmitAlertDefinitionChangeRequest {
            operation: AlertDefinitionChangeOperation::SetEnabled,
            definition_id: Some(edited.id),
            expected_version: Some(edited.version),
            definition: None,
            enabled: Some(false),
        },
        "alert-definition-lifecycle-disable",
    )
    .await;
    approve_change(&pool, &approver, disable.id, "alert-lifecycle-disable").await;
    let disabled = repo
        .get(owner_id, created.id)
        .await
        .expect("disabled definition should read");
    assert!(!disabled.enabled);
    assert_eq!(disabled.version, 3);

    let delete = submit_change(
        &app,
        &manager,
        SubmitAlertDefinitionChangeRequest {
            operation: AlertDefinitionChangeOperation::Delete,
            definition_id: Some(disabled.id),
            expected_version: Some(disabled.version),
            definition: None,
            enabled: None,
        },
        "alert-definition-lifecycle-delete",
    )
    .await;
    approve_change(&pool, &approver, delete.id, "alert-lifecycle-delete").await;
    assert!(matches!(
        repo.get(owner_id, created.id).await,
        Err(AlertDefinitionRepositoryError::NotFound)
    ));
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H-AL' AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(created.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("definition lifecycle audit should query");
    assert_eq!(audit_count, 4);
}

async fn submit_change(
    app: &axum::Router,
    actor: &AuthContext,
    request: SubmitAlertDefinitionChangeRequest,
    idempotency_key: &str,
) -> QualityLiaisonOrder {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", idempotency_key)
                .extension(actor.clone())
                .body(Body::from(
                    serde_json::to_vec(&request).expect("change request should serialize"),
                ))
                .expect("change request should build"),
        )
        .await
        .expect("change request should respond");
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("change response should read"),
    )
    .expect("change response should deserialize")
}

async fn approve_change(
    pool: &PgPool,
    approver: &AuthContext,
    liaison_id: Uuid,
    external_approval_id: &str,
) {
    PgQualityLiaisonRepository::new(pool.clone())
        .apply_approval_callback(
            approver,
            liaison_id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "生命周期审批通过".to_string(),
                external_approval_id: external_approval_id.to_string(),
            },
            Utc::now(),
            &format!("{external_approval_id}-idempotency"),
        )
        .await
        .expect("alert definition lifecycle approval should apply");
}

#[sqlx::test(migrations = "../../migrations")]
async fn approved_change_is_idempotent_owner_scoped_and_audited(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    configure_notification_channel(&pool, owner_id, manager_id).await;
    let manager = ctx(
        owner_id,
        manager_id,
        &["hal.alert-definition.read", "hal.alert-definition.write"],
    );
    let app = alert_definition_router(AlertDefinitionAppState::with_postgres(pool.clone()));
    let body = serde_json::to_vec(&SubmitAlertDefinitionChangeRequest {
        operation: AlertDefinitionChangeOperation::Upsert,
        definition_id: None,
        expected_version: None,
        definition: Some(draft()),
        enabled: None,
    })
    .expect("alert definition change should serialize");

    let submit = || {
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", "alert-definition-create")
                .extension(manager.clone())
                .body(Body::from(body.clone()))
                .expect("alert definition change request should build"),
        )
    };
    let first_response = submit()
        .await
        .expect("alert definition change should respond");
    assert_eq!(first_response.status(), StatusCode::OK);
    let first: QualityLiaisonOrder = serde_json::from_slice(
        &to_bytes(first_response.into_body(), usize::MAX)
            .await
            .expect("alert definition liaison body should read"),
    )
    .expect("alert definition liaison should deserialize");
    assert_eq!(first.status, "pending_approval");
    let before_approval: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM alert_definitions WHERE owner_id = $1 AND alert_code = 'inventory.low'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("pending alert definition should query");
    assert_eq!(
        before_approval, 0,
        "pending change must not mutate runtime config"
    );

    let replay_response = submit()
        .await
        .expect("alert definition change replay should respond");
    assert_eq!(replay_response.status(), StatusCode::OK);
    let replay: QualityLiaisonOrder = serde_json::from_slice(
        &to_bytes(replay_response.into_body(), usize::MAX)
            .await
            .expect("alert definition replay body should read"),
    )
    .expect("alert definition replay should deserialize");
    assert_eq!(replay.id, first.id);

    PgQualityLiaisonRepository::new(pool.clone())
        .apply_approval_callback(
            &ctx(owner_id, approver_id, &["mql.quality-liaison.approve"]),
            first.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "同意新增业务告警".to_string(),
                external_approval_id: "ww-alert-definition-001".to_string(),
            },
            Utc::now(),
            "alert-definition-approval",
        )
        .await
        .expect("approved alert definition should apply");

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alert-definitions?keyword=inventory.low&limit=50")
                .extension(manager.clone())
                .body(Body::empty())
                .expect("alert definition list request should build"),
        )
        .await
        .expect("alert definition list should respond");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list: AlertDefinitionListResponse = serde_json::from_slice(
        &to_bytes(list_response.into_body(), usize::MAX)
            .await
            .expect("alert definition list body should read"),
    )
    .expect("alert definition list should deserialize");
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].alert_code, "inventory.low");
    assert!(list.data[0].enabled);
    assert_eq!(list.data[0].version, 1);

    let cross_owner = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/alert-definitions/{}", list.data[0].id))
                .extension(AuthContext {
                    owner_id: Uuid::new_v4(),
                    ..manager.clone()
                })
                .body(Body::empty())
                .expect("cross-owner alert definition request should build"),
        )
        .await
        .expect("cross-owner alert definition should respond");
    assert_eq!(cross_owner.status(), StatusCode::NOT_FOUND);

    let forbidden = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", "alert-definition-forbidden")
                .extension(ctx(owner_id, manager_id, &["hal.alert-definition.read"]))
                .body(Body::from(body))
                .expect("forbidden alert definition request should build"),
        )
        .await
        .expect("forbidden alert definition request should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H-AL' AND action = 'upsert_alert_definition'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("alert definition audit should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn forced_alert_disable_rolls_back_mql_and_h4_approval(pool: PgPool) {
    let (owner_id, manager_id, approver_id) = seed_owner_and_users(&pool).await;
    configure_approval(&pool, owner_id, manager_id, approver_id).await;
    let forced: (Uuid, i64) = sqlx::query_as(
        "SELECT id, version FROM alert_definitions WHERE owner_id = $1 AND alert_code = 'cold_chain_break_received'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("forced alert should seed");
    let app = alert_definition_router(AlertDefinitionAppState::with_postgres(pool.clone()));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alert-definitions/change-requests")
                .header("content-type", "application/json")
                .header("idempotency-key", "forced-alert-disable")
                .extension(ctx(owner_id, manager_id, &["hal.alert-definition.write"]))
                .body(Body::from(
                    serde_json::to_vec(&SubmitAlertDefinitionChangeRequest {
                        operation: AlertDefinitionChangeOperation::SetEnabled,
                        definition_id: Some(forced.0),
                        expected_version: Some(forced.1),
                        definition: None,
                        enabled: Some(false),
                    })
                    .expect("forced alert disable should serialize"),
                ))
                .expect("forced alert disable request should build"),
        )
        .await
        .expect("forced alert disable should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let liaison: QualityLiaisonOrder = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("forced alert liaison body should read"),
    )
    .expect("forced alert liaison should deserialize");

    let error = PgQualityLiaisonRepository::new(pool.clone())
        .apply_approval_callback(
            &ctx(owner_id, approver_id, &["mql.quality-liaison.approve"]),
            liaison.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "尝试停用强制告警".to_string(),
                external_approval_id: "ww-alert-definition-forced".to_string(),
            },
            Utc::now(),
            "forced-alert-disable-approval",
        )
        .await
        .expect_err("GSP forced alert must not be disabled");
    assert_eq!(error, QualityLiaisonError::BusinessActionInvalid);
    let state: (bool, String, String) = sqlx::query_as(
        "SELECT alert.enabled, liaison.status, approval.status FROM alert_definitions alert JOIN quality_liaison_orders liaison ON liaison.owner_id = alert.owner_id JOIN h4_approval_records approval ON approval.id = liaison.approval_record_id WHERE alert.id = $1 AND liaison.id = $2",
    )
    .bind(forced.0)
    .bind(liaison.id)
    .fetch_one(&pool)
    .await
    .expect("forced alert rollback state should query");
    assert_eq!(
        state,
        (true, "pending_approval".to_string(), "pending".to_string())
    );
}
