use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m9-rule-test".to_string(),
        permissions: vec!["m9.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn audit(ctx: &AuthContext, now: chrono::DateTime<Utc>) -> AuditWriteRequest {
    let mut request =
        AuditWriteRequest::from_auth_context(ctx, "create_rule", "M9", "billing_rule", "", None);
    request.occurred_at = now;
    request
}

fn rule_request(contract_id: Uuid) -> CreateBillingRuleRequest {
    CreateBillingRuleRequest {
        contract_id,
        charge_item: "storage".to_string(),
        unit: "pallet_day".to_string(),
        unit_price_cents: 125.into(),
        billing_cycle: "monthly".to_string(),
        effective_from: "2026-06-01".to_string(),
        effective_to: "2026-06-30".to_string(),
    }
}

async fn seed_contract(
    repo: &PgWave3Repository,
    ctx: &AuthContext,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let account = repo
        .create_billing_account(
            ctx,
            CreateBillingAccountRequest {
                account_code: format!("M9-RULE-{}", ctx.owner_id.simple()),
                account_name: "M9 rule account".to_string(),
            },
            now,
        )
        .await
        .expect("create billing account");
    repo.create_billing_contract(
        ctx,
        CreateBillingContractRequest {
            account_id: account.id,
            contract_no: format!("M9-RULE-CONTRACT-{}", ctx.owner_id.simple()),
            valid_from: "2026-06-01".to_string(),
            valid_to: "2026-06-30".to_string(),
        },
        now,
    )
    .await
    .expect("create billing contract")
    .id
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_rule_is_idempotent_owner_scoped_and_audited(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let ctx_a = ctx(owner_a);
    let ctx_b = ctx(owner_b);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid test time");
    let contract_id = seed_contract(&repo, &ctx_a, now).await;
    let request = rule_request(contract_id);

    let first = repo
        .create_billing_rule_with_audit(
            &ctx_a,
            request.clone(),
            now,
            "m9-rule-idempotency-key",
            audit(&ctx_a, now),
        )
        .await
        .expect("create billing rule");
    let replay = repo
        .create_billing_rule_with_audit(
            &ctx_a,
            request.clone(),
            now,
            "m9-rule-idempotency-key",
            audit(&ctx_a, now),
        )
        .await
        .expect("replay billing rule");
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);

    let mut changed_request = request.clone();
    changed_request.unit_price_cents = 130.into();
    let conflict = repo
        .create_billing_rule_with_audit(
            &ctx_a,
            changed_request,
            now,
            "m9-rule-idempotency-key",
            audit(&ctx_a, now),
        )
        .await;
    assert!(matches!(
        conflict,
        Err(Wave3RepositoryError::IdempotencyConflict)
    ));

    let cross_owner = repo
        .create_billing_rule(&ctx_b, request, now)
        .await
        .expect_err("a contract must not cross owner boundaries");
    assert!(matches!(cross_owner, Wave3RepositoryError::NotFound));

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM billing_rules WHERE owner_id = $1), (SELECT COUNT(*) FROM billing_rules WHERE owner_id = $2), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'm9-rule-idempotency-key'), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create_rule')",
    )
    .bind(owner_a)
    .bind(owner_b)
    .fetch_one(&pool)
    .await
    .expect("query billing rule evidence");
    assert_eq!(counts, (1, 0, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_rule_validates_fields_and_contract_window(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let repo = PgWave3Repository::new(pool);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid test time");
    let contract_id = seed_contract(&repo, &context, now).await;

    let mut invalid_charge_item = rule_request(contract_id);
    invalid_charge_item.charge_item = "unknown".to_string();
    assert!(matches!(
        repo.create_billing_rule(&context, invalid_charge_item, now)
            .await,
        Err(Wave3RepositoryError::InvalidBillingRuleField)
    ));

    let mut invalid_unit = rule_request(contract_id);
    invalid_unit.unit = "unknown".to_string();
    assert!(matches!(
        repo.create_billing_rule(&context, invalid_unit, now).await,
        Err(Wave3RepositoryError::InvalidBillingRuleField)
    ));

    let mut invalid_cycle = rule_request(contract_id);
    invalid_cycle.billing_cycle = "hourly".to_string();
    assert!(matches!(
        repo.create_billing_rule(&context, invalid_cycle, now).await,
        Err(Wave3RepositoryError::InvalidBillingRuleField)
    ));

    let mut invalid_rate = rule_request(contract_id);
    invalid_rate.unit_price_cents = (-1).into();
    assert!(matches!(
        repo.create_billing_rule(&context, invalid_rate, now).await,
        Err(Wave3RepositoryError::InvalidRate)
    ));

    let mut invalid_window = rule_request(contract_id);
    invalid_window.effective_from = "2026-07-01".to_string();
    invalid_window.effective_to = "2026-06-30".to_string();
    assert!(matches!(
        repo.create_billing_rule(&context, invalid_window, now)
            .await,
        Err(Wave3RepositoryError::InvalidEffectiveWindow)
    ));

    let mut outside_contract = rule_request(contract_id);
    outside_contract.effective_from = "2026-05-01".to_string();
    outside_contract.effective_to = "2026-05-31".to_string();
    assert!(matches!(
        repo.create_billing_rule(&context, outside_contract, now)
            .await,
        Err(Wave3RepositoryError::InvalidEffectiveWindow)
    ));
}
