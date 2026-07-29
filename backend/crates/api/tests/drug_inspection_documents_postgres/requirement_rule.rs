#[sqlx::test(migrations = "../../migrations")]
async fn upsert_drug_inspection_requirement_rule_is_audited_and_idempotent(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let app = drug_inspection_document_router(DrugInspectionDocumentAppState::with_postgres(
        pool.clone(),
    ));
    let request = json!({
        "special_drug_category": "narcotic",
        "missing_behavior": "block",
        "enabled": true
    });
    for attempt in 0..2 {
        let response = app
            .clone()
            .oneshot(json_request(
                "PUT",
                "/api/v1/drug-inspection/requirement-rules/current",
                context(
                    fixture.owner_id,
                    fixture.reviewer_id,
                    &["m-di.requirement-rule.manage"],
                ),
                Some("di-requirement-rule"),
                request.clone(),
            ))
            .await
            .expect("requirement rule upsert should respond");
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "attempt {attempt} should succeed"
        );
    }
    let evidence: (i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM audit_event
             WHERE owner_id = $1
               AND action = 'di.requirement_rule.upsert'),
           (SELECT COUNT(*) FROM idempotency_request
             WHERE owner_id = $1
               AND idempotency_key = 'di-requirement-rule')",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("requirement rule audit and idempotency evidence should query");
    assert_eq!(evidence, (1, 1));
}
