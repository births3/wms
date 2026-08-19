use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{CreateBillingAccountRequest, CreateBillingContractRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m9-contract-test".to_string(),
        permissions: vec!["m9.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn contract_request(account_id: Uuid, contract_no: &str) -> CreateBillingContractRequest {
    CreateBillingContractRequest {
        account_id,
        contract_no: contract_no.to_string(),
        valid_from: "2026-06-01".to_string(),
        valid_to: "2026-06-30".to_string(),
    }
}

fn audit(ctx: &AuthContext, now: chrono::DateTime<Utc>) -> AuditWriteRequest {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        "create_contract",
        "M9",
        "billing_contract",
        "",
        None,
    );
    audit.occurred_at = now;
    audit
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_contract_creation_is_idempotent_and_owner_scoped(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let ctx_a = ctx(owner_a);
    let ctx_b = ctx(owner_b);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid test time");
    let account = repo
        .create_billing_account(
            &ctx_a,
            CreateBillingAccountRequest {
                account_code: "M9-IDEM-ACCOUNT".to_string(),
                account_name: "M9 idempotency account".to_string(),
            },
            now,
        )
        .await
        .expect("create account");
    let request = contract_request(account.id, "M9-CONTRACT-001");

    let first = repo
        .create_billing_contract_with_audit(
            &ctx_a,
            request.clone(),
            now,
            "m9-contract-key",
            audit(&ctx_a, now),
        )
        .await
        .expect("create contract");
    let replay = repo
        .create_billing_contract_with_audit(
            &ctx_a,
            request,
            now,
            "m9-contract-key",
            audit(&ctx_a, now),
        )
        .await
        .expect("replay contract");
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);

    let conflict = repo
        .create_billing_contract_with_audit(
            &ctx_a,
            contract_request(account.id, "M9-CONTRACT-002"),
            now,
            "m9-contract-key",
            audit(&ctx_a, now),
        )
        .await;
    assert!(matches!(
        conflict,
        Err(Wave3RepositoryError::IdempotencyConflict)
    ));

    let cross_owner = repo
        .create_billing_contract_with_audit(
            &ctx_b,
            contract_request(account.id, "M9-CROSS-OWNER"),
            now,
            "m9-cross-owner-key",
            audit(&ctx_b, now),
        )
        .await;
    assert!(matches!(cross_owner, Err(Wave3RepositoryError::NotFound)));

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM billing_contracts WHERE owner_id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create_contract'), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'm9-contract-key'), (SELECT COUNT(*) FROM billing_contracts WHERE owner_id = $2)",
    )
    .bind(owner_a)
    .bind(owner_b)
    .fetch_one(&pool)
    .await
    .expect("query contract idempotency evidence");
    assert_eq!(counts, (1, 1, 1, 0));
}
