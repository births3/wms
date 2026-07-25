use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    quality_liaison::{PgQualityLiaisonRepository, QualityLiaisonError},
    quality_liaison_handlers::{quality_liaison_router, QualityLiaisonAppState},
};
use wms_domain::{
    CreateQualityLiaisonRequest, QualityLiaisonApprovalCallbackRequest, QualityLiaisonOrder,
    QualityLiaisonTypeConfig, UpsertQualityLiaisonTypeRequest,
};

fn ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "quality-liaison-test".to_string(),
        permissions: vec![
            "mql.quality-liaison.read".to_string(),
            "mql.quality-liaison.write".to_string(),
            "mql.quality-liaison.config".to_string(),
            "mql.quality-liaison.approve".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_actor(pool: &PgPool, owner_id: Uuid, user_id: Uuid, label: &str) {
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("ql-{label}-{}", &user_id.to_string()[..8]))
    .bind(label)
    .execute(pool)
    .await
    .expect("quality liaison actor should seed");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("quality liaison owner binding should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn type_config_create_and_approval_callback_are_atomic_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '质量联系单测试货主')",
    )
    .bind(owner_id)
    .bind(format!("QL-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("quality liaison owner should seed");
    seed_actor(&pool, owner_id, creator_id, "creator").await;
    seed_actor(&pool, owner_id, approver_id, "approver").await;

    let repository = PgQualityLiaisonRepository::new(pool.clone());
    let creator = ctx(owner_id, creator_id);
    let now = Utc::now();
    let invalid_type = repository
        .upsert_type(
            &creator,
            UpsertQualityLiaisonTypeRequest {
                type_code: "_invalid".to_string(),
                type_name: "非法类型".to_string(),
                approval_template_id: "ww-invalid".to_string(),
                approver_user_id: approver_id,
                timeout_seconds: 60,
                enabled: true,
            },
            now,
            "ql-type-invalid",
        )
        .await
        .expect_err("invalid type code should fail before the database constraint");
    assert_eq!(invalid_type, QualityLiaisonError::InvalidRequest);
    let config_request = UpsertQualityLiaisonTypeRequest {
        type_code: "validation_override".to_string(),
        type_name: "校验强制通过".to_string(),
        approval_template_id: "ww-ql-validation-override".to_string(),
        approver_user_id: approver_id,
        timeout_seconds: 4 * 60 * 60,
        enabled: true,
    };
    let config = repository
        .upsert_type(&creator, config_request.clone(), now, "ql-type-config")
        .await
        .expect("quality liaison type should configure");
    assert_eq!(config.value.type_code, "validation_override");
    assert!(!config.replayed);
    let config_replay = repository
        .upsert_type(&creator, config_request, now, "ql-type-config")
        .await
        .expect("same type configuration should replay");
    assert!(config_replay.replayed);
    assert_eq!(config_replay.value.id, config.value.id);

    let create_request = CreateQualityLiaisonRequest {
        type_code: "validation_override".to_string(),
        related_document_type: "outbound_order".to_string(),
        related_document_no: "SO-QL-001".to_string(),
        problem_description: "指定批号库存不足，申请强制通过".to_string(),
        disposition_suggestion: "主管复核后放行".to_string(),
        trigger_source: "M-VR".to_string(),
        business_payload: serde_json::json!({"validation_exception_id":"VR-EX-001"}),
    };
    let created = repository
        .create(&creator, create_request.clone(), now, "ql-create")
        .await
        .expect("quality liaison should create");
    assert_eq!(created.value.status, "pending_approval");
    assert!(created.value.liaison_no.starts_with("QL"));
    let approval_id = created
        .value
        .approval_record_id
        .expect("quality liaison should create H4 approval");
    let h4_approval: (String, String, String, String) = sqlx::query_as(
        "SELECT scenario, business_ref, process_id, status FROM h4_approval_records WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(approval_id)
    .fetch_one(&pool)
    .await
    .expect("H4 approval should persist atomically");
    assert_eq!(h4_approval.0, "quality_liaison");
    assert_eq!(h4_approval.1, created.value.id.to_string());
    assert_eq!(h4_approval.2, "ww-ql-validation-override");
    assert_eq!(h4_approval.3, "pending");

    let create_replay = repository
        .create(&creator, create_request, now, "ql-create")
        .await
        .expect("same create request should replay");
    assert!(create_replay.replayed);
    assert_eq!(create_replay.value.id, created.value.id);

    let approver = ctx(owner_id, approver_id);
    let empty_opinion = repository
        .apply_approval_callback(
            &approver,
            created.value.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: " ".to_string(),
                external_approval_id: "ww-ql-approval-empty".to_string(),
            },
            now,
            "ql-approval-empty",
        )
        .await
        .expect_err("approval opinion is required");
    assert_eq!(empty_opinion, QualityLiaisonError::ApprovalOpinionRequired);

    let callback_request = QualityLiaisonApprovalCallbackRequest {
        conclusion: "rejected".to_string(),
        opinion: "拒绝强制通过，保持原异常状态".to_string(),
        external_approval_id: "ww-ql-approval-001".to_string(),
    };
    let rejected = repository
        .apply_approval_callback(
            &approver,
            created.value.id,
            callback_request.clone(),
            now,
            "ql-approval",
        )
        .await
        .expect("quality liaison rejection should apply");
    assert_eq!(rejected.value.status, "rejected");
    assert_eq!(rejected.value.approved_by, Some(approver_id));
    let callback_replay = repository
        .apply_approval_callback(
            &approver,
            created.value.id,
            callback_request,
            now,
            "ql-approval",
        )
        .await
        .expect("same approval callback should replay");
    assert!(callback_replay.replayed);

    let states: (String, String, String) = sqlx::query_as(
        "SELECT q.status, h.status, h.external_approval_id FROM quality_liaison_orders q JOIN h4_approval_records h ON h.id = q.approval_record_id WHERE q.owner_id = $1 AND q.id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("quality liaison and H4 approval states should query");
    assert_eq!(
        states,
        (
            "rejected".to_string(),
            "rejected".to_string(),
            "ww-ql-approval-001".to_string()
        )
    );

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'M-QL' AND action = ANY($2)",
    )
    .bind(owner_id)
    .bind(vec![
        "upsert_quality_liaison_type",
        "create_quality_liaison",
        "apply_quality_liaison_approval",
    ])
    .fetch_one(&pool)
    .await
    .expect("quality liaison audit evidence should query");
    assert_eq!(
        audit_count, 3,
        "idempotent replays must not duplicate audit"
    );
}

#[path = "quality_liaison_postgres/archive_revision.rs"]
mod archive_revision;

#[sqlx::test(migrations = "../../migrations")]
async fn quality_liaison_api_enforces_permissions_and_designated_approver(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '质量联系单接口测试货主')",
    )
    .bind(owner_id)
    .bind(format!("QL-API-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("quality liaison API owner should seed");
    seed_actor(&pool, owner_id, creator_id, "api-creator").await;
    seed_actor(&pool, owner_id, approver_id, "api-approver").await;
    let creator = ctx(owner_id, creator_id);
    PgQualityLiaisonRepository::new(pool.clone())
        .upsert_type(
            &creator,
            UpsertQualityLiaisonTypeRequest {
                type_code: "validation_override".to_string(),
                type_name: "校验强制通过".to_string(),
                approval_template_id: "ww-ql-validation-override".to_string(),
                approver_user_id: approver_id,
                timeout_seconds: 4 * 60 * 60,
                enabled: true,
            },
            Utc::now(),
            "ql-api-type-config",
        )
        .await
        .expect("quality liaison API type should seed");
    let app = quality_liaison_router(QualityLiaisonAppState::with_postgres(pool));
    let type_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/quality-liaisons/types/validation_override")
                .extension(creator.clone())
                .body(Body::empty())
                .expect("quality liaison type request should build"),
        )
        .await
        .expect("quality liaison type should respond");
    assert_eq!(type_response.status(), StatusCode::OK);
    let type_config: QualityLiaisonTypeConfig = serde_json::from_slice(
        &to_bytes(type_response.into_body(), usize::MAX)
            .await
            .expect("quality liaison type response body should read"),
    )
    .expect("quality liaison type response should deserialize");
    assert_eq!(type_config.type_code, "validation_override");
    let type_forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/quality-liaisons/types/validation_override")
                .extension(AuthContext {
                    permissions: vec!["mql.quality-liaison.read".to_string()],
                    ..creator.clone()
                })
                .body(Body::empty())
                .expect("forbidden quality liaison type request should build"),
        )
        .await
        .expect("forbidden quality liaison type should respond");
    assert_eq!(type_forbidden.status(), StatusCode::FORBIDDEN);
    let cross_owner_type = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/quality-liaisons/types/validation_override")
                .extension(AuthContext {
                    owner_id: Uuid::new_v4(),
                    ..creator.clone()
                })
                .body(Body::empty())
                .expect("cross-owner quality liaison type request should build"),
        )
        .await
        .expect("cross-owner quality liaison type should respond");
    assert_eq!(cross_owner_type.status(), StatusCode::NOT_FOUND);

    let create_body = serde_json::to_vec(&CreateQualityLiaisonRequest {
        type_code: "validation_override".to_string(),
        related_document_type: "outbound_order".to_string(),
        related_document_no: "SO-QL-API-001".to_string(),
        problem_description: "校验异常".to_string(),
        disposition_suggestion: "审批后放行".to_string(),
        trigger_source: "M-VR".to_string(),
        business_payload: serde_json::json!({"validation_exception_id":"VR-API-001"}),
    })
    .expect("quality liaison API request should serialize");
    let read_only = AuthContext {
        permissions: vec!["mql.quality-liaison.read".to_string()],
        ..creator.clone()
    };
    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/quality-liaisons")
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-api-create-forbidden")
                .extension(read_only.clone())
                .body(Body::from(create_body.clone()))
                .expect("forbidden quality liaison request should build"),
        )
        .await
        .expect("quality liaison router should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let created_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/quality-liaisons")
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-api-create")
                .extension(creator.clone())
                .body(Body::from(create_body))
                .expect("quality liaison create request should build"),
        )
        .await
        .expect("quality liaison router should create");
    assert_eq!(created_response.status(), StatusCode::OK);
    let created: QualityLiaisonOrder = serde_json::from_slice(
        &to_bytes(created_response.into_body(), usize::MAX)
            .await
            .expect("quality liaison response body should read"),
    )
    .expect("quality liaison response should deserialize");

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/quality-liaisons/{}", created.id))
                .extension(read_only.clone())
                .body(Body::empty())
                .expect("quality liaison detail request should build"),
        )
        .await
        .expect("quality liaison detail should respond");
    assert_eq!(detail.status(), StatusCode::OK);

    let cross_owner = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/quality-liaisons/{}", created.id))
                .extension(AuthContext {
                    owner_id: Uuid::new_v4(),
                    ..read_only
                })
                .body(Body::empty())
                .expect("cross-owner quality liaison detail request should build"),
        )
        .await
        .expect("cross-owner quality liaison detail should respond");
    assert_eq!(cross_owner.status(), StatusCode::NOT_FOUND);

    let callback_body = serde_json::to_vec(&QualityLiaisonApprovalCallbackRequest {
        conclusion: "approved".to_string(),
        opinion: "同意".to_string(),
        external_approval_id: "ww-ql-api-001".to_string(),
    })
    .expect("quality liaison callback should serialize");
    let forged = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/quality-liaisons/{}/approval-callback",
                    created.id
                ))
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-api-forged-approval")
                .extension(creator)
                .body(Body::from(callback_body.clone()))
                .expect("forged quality liaison callback should build"),
        )
        .await
        .expect("forged quality liaison callback should respond");
    assert_eq!(forged.status(), StatusCode::FORBIDDEN);

    let approved = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/quality-liaisons/{}/approval-callback",
                    created.id
                ))
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-api-approval")
                .extension(ctx(owner_id, approver_id))
                .body(Body::from(callback_body))
                .expect("quality liaison callback should build"),
        )
        .await
        .expect("quality liaison callback should respond");
    assert_eq!(approved.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn approved_destruction_disposition_creates_msa_order_with_quality_source(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '质量销毁测试货主')",
    )
    .bind(owner_id)
    .bind(format!("QL-D-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("destruction owner should seed");
    seed_actor(&pool, owner_id, creator_id, "destruction-creator").await;
    seed_actor(&pool, owner_id, approver_id, "destruction-approver").await;
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '质量销毁测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("QL-D-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("destruction warehouse should seed");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        ) VALUES (
            $1, $2, 'QL-DESTRUCTION-P', 'QL-DESTRUCTION-B', DATE '2026-01-01',
            DATE '2028-01-01', 10, 0, 'unqualified', $3, 'QL-D-LOC', FALSE, $4, $4
        )
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("destruction inventory batch should seed");

    let repository = PgQualityLiaisonRepository::new(pool.clone());
    let creator = ctx(owner_id, creator_id);
    repository
        .upsert_type(
            &creator,
            UpsertQualityLiaisonTypeRequest {
                type_code: "inbound_rejection".to_string(),
                type_name: "验收不合格".to_string(),
                approval_template_id: "ww-ql-inbound-rejection".to_string(),
                approver_user_id: approver_id,
                timeout_seconds: 4 * 60 * 60,
                enabled: true,
            },
            Utc::now(),
            "ql-destruction-type",
        )
        .await
        .expect("destruction quality liaison type should configure");
    let liaison = repository
        .create(
            &creator,
            CreateQualityLiaisonRequest {
                type_code: "inbound_rejection".to_string(),
                related_document_type: "asn".to_string(),
                related_document_no: "ASN-QL-DESTRUCTION-001".to_string(),
                problem_description: "验收不合格，批准后销毁".to_string(),
                disposition_suggestion: "销毁".to_string(),
                trigger_source: "M2".to_string(),
                business_payload: serde_json::json!({
                    "action":"create_stock_loss",
                    "warehouse_id":warehouse_id,
                    "batch_id":batch_id,
                    "quantity":2,
                    "reason_code":"destruction"
                }),
            },
            Utc::now(),
            "ql-destruction-create",
        )
        .await
        .expect("destruction quality liaison should create");
    repository
        .apply_approval_callback(
            &ctx(owner_id, approver_id),
            liaison.value.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "同意按销毁流程执行".to_string(),
                external_approval_id: "ww-ql-destruction-001".to_string(),
            },
            Utc::now(),
            "ql-destruction-approval",
        )
        .await
        .expect("approved destruction should create stock loss order");

    let stock_order: (String, String, bool, String) = sqlx::query_as(
        "SELECT reason_code, status, requires_quality_approval, quality_liaison_id FROM stock_adjustment_orders WHERE owner_id = $1 AND adjustment_type = 'loss' AND quality_liaison_id = $2",
    )
    .bind(owner_id)
    .bind(liaison.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("M-QL destruction should create M-SA order");
    assert_eq!(stock_order.0, "destruction");
    assert_eq!(stock_order.1, "pending_execution");
    assert!(stock_order.2);
    assert_eq!(stock_order.3, liaison.value.id.to_string());

    let invalid_liaison = repository
        .create(
            &creator,
            CreateQualityLiaisonRequest {
                type_code: "inbound_rejection".to_string(),
                related_document_type: "asn".to_string(),
                related_document_no: "ASN-QL-DESTRUCTION-002".to_string(),
                problem_description: "销毁数量超过可用库存".to_string(),
                disposition_suggestion: "销毁".to_string(),
                trigger_source: "M2".to_string(),
                business_payload: serde_json::json!({
                    "action":"create_stock_loss",
                    "warehouse_id":warehouse_id,
                    "batch_id":batch_id,
                    "quantity":11,
                    "reason_code":"destruction"
                }),
            },
            Utc::now(),
            "ql-destruction-invalid-create",
        )
        .await
        .expect("invalid destruction liaison should still enter approval");
    let action_error = repository
        .apply_approval_callback(
            &ctx(owner_id, approver_id),
            invalid_liaison.value.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "同意，但执行必须校验库存".to_string(),
                external_approval_id: "ww-ql-destruction-002".to_string(),
            },
            Utc::now(),
            "ql-destruction-invalid-approval",
        )
        .await
        .expect_err("failed M-SA creation must fail the approval transaction");
    assert_eq!(action_error, QualityLiaisonError::BusinessActionInvalid);
    let rolled_back: (String, String, i64) = sqlx::query_as(
        r#"
        SELECT q.status, h.status,
               (SELECT COUNT(*) FROM stock_adjustment_orders s
                 WHERE s.owner_id = q.owner_id
                   AND s.quality_liaison_id = q.id::text)
          FROM quality_liaison_orders q
          JOIN h4_approval_records h ON h.id = q.approval_record_id
         WHERE q.owner_id = $1 AND q.id = $2
        "#,
    )
    .bind(owner_id)
    .bind(invalid_liaison.value.id)
    .fetch_one(&pool)
    .await
    .expect("failed action states should query");
    assert_eq!(
        rolled_back,
        ("pending_approval".to_string(), "pending".to_string(), 0),
        "M-QL and H4 approval states must roll back with the failed M-SA action"
    );
}
