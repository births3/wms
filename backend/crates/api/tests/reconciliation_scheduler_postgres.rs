use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension, Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    reconciliation::{PgReconciliationRepository, RunReconciliationRequest},
    reconciliation_handlers::{reconciliation_router, ReconciliationAppState},
    reconciliation_query::{ClaimReconciliationRequest, ReconciliationItemQuery},
};

fn ctx(owner_id: Uuid, user_id: Uuid, permission: &str) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "rc-scheduler-test".into(),
        permissions: vec![permission.into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_due_owner(pool: &PgPool) -> (Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1, $2, 'RC 调度租约货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-LEASE-{}", &owner_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed reconciliation claim owner");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, 'RC 调度服务账号', 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("rc-lease-{}", &user_id.simple().to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed reconciliation claim actor");
    sqlx::query(
        "INSERT INTO h8_erp_connectors
         (id, owner_id, connector_code, connector_name, directions, message_types,
          channel_mode, api_base_url, status)
         VALUES ($1,$2,'rc-lease','RC 库存快照','{outbound}','{inventory_snapshot}',
                 'rest','https://erp.example.test','active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed owner-wide inventory snapshot connector");
    sqlx::query(
        "INSERT INTO reconciliation_rules (owner_id, interval_hours, enabled, updated_by)
         VALUES ($1, 6, TRUE, $2)",
    )
    .bind(owner_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed reconciliation schedule rule");
    (owner_id, user_id)
}

fn app(pool: &PgPool, auth: AuthContext) -> Router {
    reconciliation_router(ReconciliationAppState::with_postgres(pool.clone()))
        .layer(Extension(auth))
}

async fn post_json(
    app: Router,
    uri: &str,
    idempotency_key: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("idempotency-key", idempotency_key)
                .body(Body::from(body.to_string()))
                .expect("build reconciliation scheduler request"),
        )
        .await
        .expect("call reconciliation scheduler route");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read reconciliation scheduler response");
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn claim(app: Router, worker_id: &str, idempotency_key: &str) -> (StatusCode, Value) {
    post_json(
        app,
        "/api/v1/reconciliation/claims",
        idempotency_key,
        json!({"worker_id": worker_id, "lease_seconds": 120}),
    )
    .await
}

#[sqlx::test(migrations = "../../migrations")]
async fn service_workers_claim_once_renew_and_take_over_expired_lease(pool: PgPool) {
    let (owner_id, user_id) = seed_due_owner(&pool).await;
    let denied = app(&pool, ctx(owner_id, user_id, "rc.reconciliation.execute"));
    let (status, _) = claim(denied, "human-worker", "rc-claim-denied").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let service = app(&pool, ctx(owner_id, user_id, "rc.reconciliation.ingest"));
    let (left, right) = tokio::join!(
        claim(service.clone(), "worker-a", "rc-claim-a"),
        claim(service.clone(), "worker-b", "rc-claim-b"),
    );
    assert_eq!(left.0, StatusCode::OK);
    assert_eq!(right.0, StatusCode::OK);
    let claims = [left.1["claim"].clone(), right.1["claim"].clone()]
        .into_iter()
        .filter(|value| !value.is_null())
        .collect::<Vec<_>>();
    assert_eq!(claims.len(), 1, "only one worker may claim the due window");
    let first = &claims[0];
    let claim_id = first["id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .expect("claim response id");
    let token = first["claim_token"].as_str().expect("claim response token");
    let worker_id = first["worker_id"].as_str().expect("claim response worker");
    assert_eq!(first["attempt_no"], 1);
    let claim_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event
          WHERE owner_id=$1 AND action='claim_reconciliation_window'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load reconciliation claim audit");
    assert_eq!(claim_audits, 1);

    let (_, replay) = claim(
        service.clone(),
        worker_id,
        if worker_id == "worker-a" {
            "rc-claim-a"
        } else {
            "rc-claim-b"
        },
    )
    .await;
    assert_eq!(replay["claim"]["claim_token"], token);

    let renew_body = json!({
        "claim_token": token,
        "worker_id": worker_id,
        "lease_seconds": 180
    });
    let renew_path = format!("/api/v1/reconciliation/claims/{claim_id}/renew");
    let (renew_status, renewed) = post_json(
        service.clone(),
        &renew_path,
        "rc-renew-1",
        renew_body.clone(),
    )
    .await;
    assert_eq!(renew_status, StatusCode::OK);
    assert_eq!(renewed["status"], "active");
    let (renew_replay_status, renew_replay) =
        post_json(service.clone(), &renew_path, "rc-renew-1", renew_body).await;
    assert_eq!(renew_replay_status, StatusCode::OK);
    assert_eq!(renew_replay, renewed);
    let renew_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event
          WHERE owner_id=$1 AND action='renew_reconciliation_claim'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load reconciliation renewal audit");
    assert_eq!(renew_audits, 1);
    let (_, blocked) = claim(service.clone(), "worker-c", "rc-claim-c-blocked").await;
    assert!(blocked["claim"].is_null());

    sqlx::query(
        "UPDATE reconciliation_schedule_claims
            SET lease_expires_at = now() - INTERVAL '1 second'
          WHERE id = $1",
    )
    .bind(claim_id)
    .execute(&pool)
    .await
    .expect("expire first reconciliation claim");
    let (takeover_status, takeover) =
        claim(service.clone(), "worker-c", "rc-claim-c-takeover").await;
    assert_eq!(takeover_status, StatusCode::OK);
    assert_eq!(takeover["claim"]["attempt_no"], 2);
    assert_ne!(takeover["claim"]["claim_token"], token);
    let (expired_fail_status, _) = post_json(
        service,
        &format!("/api/v1/reconciliation/claims/{claim_id}/failed"),
        "rc-expired-claim-must-not-change",
        json!({
            "claim_token": token,
            "stage": "pull",
            "error_code": "erp_pull_failed"
        }),
    )
    .await;
    assert_eq!(expired_fail_status, StatusCode::CONFLICT);

    let (expired_count, active_count, expiry_audits, expiry_notifications): (i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT
            (SELECT COUNT(*) FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND status='expired'),
            (SELECT COUNT(*) FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND status='active'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id=$1 AND action='expire_reconciliation_claim'),
            (SELECT COUNT(*) FROM h4_notification_records
              WHERE owner_id=$1 AND event_type='rc.reconciliation.lease_expired')",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("load reconciliation lease takeover evidence");
    assert_eq!(
        (
            expired_count,
            active_count,
            expiry_audits,
            expiry_notifications
        ),
        (1, 1, 1, 1)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn successful_run_and_claim_completion_commit_or_roll_back_together(pool: PgPool) {
    let (owner_id, user_id) = seed_due_owner(&pool).await;
    let service = app(&pool, ctx(owner_id, user_id, "rc.reconciliation.ingest"));
    let (_, claimed) = claim(service.clone(), "worker-atomic", "rc-claim-atomic").await;
    let claim_id = claimed["claim"]["id"].as_str().expect("atomic claim id");
    let claim_token = claimed["claim"]["claim_token"]
        .as_str()
        .expect("atomic claim token");
    let window_key = claimed["claim"]["window_key"]
        .as_str()
        .expect("atomic claim window");

    sqlx::query(
        "CREATE FUNCTION reject_completed_reconciliation_claim() RETURNS TRIGGER
         LANGUAGE plpgsql AS $$
         BEGIN
             IF NEW.status = 'completed' THEN
                 RAISE EXCEPTION 'reject claim completion for atomicity test';
             END IF;
             RETURN NEW;
         END
         $$",
    )
    .execute(&pool)
    .await
    .expect("create reconciliation claim rejection function");
    sqlx::query(
        "CREATE TRIGGER reject_completed_reconciliation_claim
         BEFORE UPDATE ON reconciliation_schedule_claims
         FOR EACH ROW EXECUTE FUNCTION reject_completed_reconciliation_claim()",
    )
    .execute(&pool)
    .await
    .expect("create reconciliation claim rejection trigger");

    let run_body = json!({
        "claim_id": claim_id,
        "claim_token": claim_token,
        "window_key": window_key,
        "snapshot_at": Utc::now(),
        "items": []
    });
    let mut invalid_token_body = run_body.clone();
    invalid_token_body["claim_token"] = json!(Uuid::new_v4());
    let (invalid_status, _) = post_json(
        service.clone(),
        "/api/v1/reconciliation/runs",
        "rc-run-invalid-token",
        invalid_token_body,
    )
    .await;
    assert_eq!(invalid_status, StatusCode::UNPROCESSABLE_ENTITY);

    let (failed_status, _) = post_json(
        service.clone(),
        "/api/v1/reconciliation/runs",
        "rc-run-atomic",
        run_body.clone(),
    )
    .await;
    assert_eq!(failed_status, StatusCode::INTERNAL_SERVER_ERROR);
    let (runs, active_claims): (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM reconciliation_runs WHERE owner_id=$1),
            (SELECT COUNT(*) FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND status='active')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load rolled back reconciliation run and claim");
    assert_eq!((runs, active_claims), (0, 1));

    sqlx::query(
        "DROP TRIGGER reject_completed_reconciliation_claim ON reconciliation_schedule_claims",
    )
    .execute(&pool)
    .await
    .expect("drop reconciliation claim rejection trigger");
    sqlx::query("DROP FUNCTION reject_completed_reconciliation_claim()")
        .execute(&pool)
        .await
        .expect("drop reconciliation claim rejection function");
    let (success_status, run) = post_json(
        service,
        "/api/v1/reconciliation/runs",
        "rc-run-atomic",
        run_body,
    )
    .await;
    assert_eq!(success_status, StatusCode::OK);
    let run_id = run["id"]
        .as_str()
        .expect("successful reconciliation run id");
    let (claim_status, linked_run_id): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT status, run_id FROM reconciliation_schedule_claims
          WHERE id=$1",
    )
    .bind(Uuid::parse_str(claim_id).expect("parse reconciliation claim id"))
    .fetch_one(&pool)
    .await
    .expect("load completed reconciliation claim");
    assert_eq!(claim_status, "completed");
    assert_eq!(
        linked_run_id,
        Some(Uuid::parse_str(run_id).expect("parse reconciliation run id"))
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn failed_claim_is_idempotent_audited_notified_and_retryable(pool: PgPool) {
    let (owner_id, user_id) = seed_due_owner(&pool).await;
    let service = app(&pool, ctx(owner_id, user_id, "rc.reconciliation.ingest"));
    let (_, claimed) = claim(service.clone(), "worker-failed", "rc-claim-failed").await;
    let claim_id = claimed["claim"]["id"].as_str().expect("failed claim id");
    let claim_token = claimed["claim"]["claim_token"]
        .as_str()
        .expect("failed claim token");
    let (unknown_status, _) = post_json(
        service.clone(),
        &format!("/api/v1/reconciliation/claims/{claim_id}/failed"),
        "rc-fail-claim-unknown-code",
        json!({
            "claim_token": claim_token,
            "stage": "pull",
            "error_code": "vendor_timeout"
        }),
    )
    .await;
    assert_eq!(unknown_status, StatusCode::UNPROCESSABLE_ENTITY);
    let (mismatch_status, _) = post_json(
        service.clone(),
        &format!("/api/v1/reconciliation/claims/{claim_id}/failed"),
        "rc-fail-claim-mismatched-code",
        json!({
            "claim_token": claim_token,
            "stage": "pull",
            "error_code": "snapshot_submit_failed"
        }),
    )
    .await;
    assert_eq!(mismatch_status, StatusCode::UNPROCESSABLE_ENTITY);
    let (still_active, premature_audits, premature_notifications): (i64, i64, i64) =
        sqlx::query_as(
            "SELECT
                (SELECT COUNT(*) FROM reconciliation_schedule_claims
                  WHERE owner_id=$1 AND id=$2 AND status='active'),
                (SELECT COUNT(*) FROM audit_event
                  WHERE owner_id=$1 AND action='fail_reconciliation_claim'),
                (SELECT COUNT(*) FROM h4_notification_records
                  WHERE owner_id=$1 AND event_type='rc.reconciliation.worker_failed')",
        )
        .bind(owner_id)
        .bind(Uuid::parse_str(claim_id).expect("parse failed claim id"))
        .fetch_one(&pool)
        .await
        .expect("load rejected failure report evidence");
    assert_eq!(
        (still_active, premature_audits, premature_notifications),
        (1, 0, 0)
    );

    let body = json!({
        "claim_token": claim_token,
        "stage": "pull",
        "error_code": "erp_pull_failed"
    });
    for _ in 0..2 {
        let (status, failed) = post_json(
            service.clone(),
            &format!("/api/v1/reconciliation/claims/{claim_id}/failed"),
            "rc-fail-claim-1",
            body.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(failed["status"], "failed");
    }
    let (failed_count, audits, notifications): (i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND status='failed'
                AND failure_stage='pull' AND failure_code='erp_pull_failed'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id=$1 AND action='fail_reconciliation_claim'),
            (SELECT COUNT(*) FROM h4_notification_records
              WHERE owner_id=$1 AND event_type='rc.reconciliation.worker_failed')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load failed reconciliation claim evidence");
    assert_eq!((failed_count, audits, notifications), (1, 1, 1));

    let (retry_status, retry) = claim(service, "worker-retry", "rc-claim-after-failure").await;
    assert_eq!(retry_status, StatusCode::OK);
    assert_eq!(retry["claim"]["attempt_no"], 2);
    assert_eq!(retry["claim"]["window_key"], claimed["claim"]["window_key"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn database_rejects_illegal_claim_failure_pairs_and_status_shapes(pool: PgPool) {
    let (owner_id, _) = seed_due_owner(&pool).await;
    let now = Utc::now();
    let invalid_pair = sqlx::query(
        "INSERT INTO reconciliation_schedule_claims
         (id, owner_id, window_key, claim_token, worker_id, attempt_no, status,
          lease_expires_at, failure_stage, failure_code, claimed_at, updated_at, failed_at)
         VALUES ($1,$2,'invalid-pair',$3,'db-invariant-test',1,'failed',$4,
                 'pull','snapshot_submit_failed',$5,$5,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .bind(now + chrono::Duration::minutes(2))
    .bind(now)
    .execute(&pool)
    .await;
    let invalid_status = sqlx::query(
        "INSERT INTO reconciliation_schedule_claims
         (id, owner_id, window_key, claim_token, worker_id, attempt_no, status,
          lease_expires_at, claimed_at, updated_at)
         VALUES ($1,$2,'invalid-completed',$3,'db-invariant-test',1,'completed',$4,$5,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .bind(now + chrono::Duration::minutes(2))
    .bind(now)
    .execute(&pool)
    .await;

    assert!(
        invalid_pair.is_err(),
        "database must reject mismatched pair"
    );
    assert!(
        invalid_status.is_err(),
        "completed claim must require run and completion timestamp"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn scheduled_query_returns_only_due_owner_with_active_inventory_route(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let actor = ctx(owner_id, Uuid::new_v4(), "rc.reconciliation.ingest");
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'RC 调度货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-DUE-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, 'RC 调度人', 'test-hash', 'active')",
    )
    .bind(actor.user_id)
    .bind(format!(
        "rc-due-{}",
        &actor.user_id.simple().to_string()[..8]
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO h8_erp_connectors
         (id, owner_id, connector_code, connector_name, directions, message_types,
          channel_mode, api_base_url, status)
         VALUES ($1,$2,'rc-due','RC 库存快照','{outbound}','{inventory_snapshot}',
                 'rest','https://erp.example.test','active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO reconciliation_rules (owner_id, interval_hours, enabled, updated_by)
         VALUES ($1, 6, TRUE, $2)",
    )
    .bind(owner_id)
    .bind(actor.user_id)
    .execute(&pool)
    .await
    .unwrap();

    let repository = PgReconciliationRepository::new(pool.clone());
    let claimed = repository
        .claim_due_window(
            &actor,
            ClaimReconciliationRequest {
                worker_id: "worker-due".into(),
                lease_seconds: 120,
            },
            Utc::now(),
            "rc-due-claim",
        )
        .await
        .unwrap();
    let claim = claimed.value.claim.expect("due owner should be claimed");
    assert_eq!(claim.owner_id, owner_id);
    assert!(claim.window_key.starts_with("scheduled:"));

    let request = RunReconciliationRequest {
        claim_id: claim.id,
        claim_token: claim.claim_token,
        window_key: claim.window_key,
        snapshot_at: Utc::now(),
        items: vec![],
    };
    repository
        .run(&actor, request, Utc::now(), "rc-due-run")
        .await
        .unwrap();
    assert!(repository
        .claim_due_window(
            &actor,
            ClaimReconciliationRequest {
                worker_id: "worker-not-due".into(),
                lease_seconds: 120,
            },
            Utc::now(),
            "rc-not-due-claim",
        )
        .await
        .unwrap()
        .value
        .claim
        .is_none());

    let response = repository
        .list_items(
            &actor,
            ReconciliationItemQuery {
                difference_type: Some("matched,wms_more".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(response.page.count, 0);

    let scoped_owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1, $2, 'RC 仓级连接货主')",
    )
    .bind(scoped_owner_id)
    .bind(format!(
        "RC-SCOPED-{}",
        &scoped_owner_id.simple().to_string()[..8]
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO h8_erp_connectors
         (id, owner_id, connector_code, connector_name, warehouse_ids, directions, message_types,
          channel_mode, api_base_url, status)
         VALUES ($1,$2,'rc-scoped','RC 仓级库存快照',$3,'{outbound}','{inventory_snapshot}',
                 'rest','https://erp.example.test','active')",
    )
    .bind(Uuid::new_v4())
    .bind(scoped_owner_id)
    .bind(vec![Uuid::new_v4()])
    .execute(&pool)
    .await
    .unwrap();
    let scoped_actor = ctx(scoped_owner_id, actor.user_id, "rc.reconciliation.ingest");
    assert!(repository
        .claim_due_window(
            &scoped_actor,
            ClaimReconciliationRequest {
                worker_id: "worker-scoped".into(),
                lease_seconds: 120,
            },
            Utc::now(),
            "rc-scoped-claim",
        )
        .await
        .unwrap()
        .value
        .claim
        .is_none());
}
