#[sqlx::test(migrations = "../../migrations")]
async fn temperature_excursion_disposition_rejects_unaffected_batch_without_side_effects(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 9, 30, 0)
        .single()
        .expect("valid time");
    let affected_batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    let unrelated_batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    seed_temperature_excursion(
        &pool,
        owner_id,
        "TEMP-EXT-W4-002",
        vec![affected_batch_id],
        now,
    )
    .await;

    let result = repo
        .dispose_temperature_excursion_and_quarantine_batches(
            &ctx,
            "TEMP-EXT-W4-002",
            vec![unrelated_batch_id],
            now,
            None,
        )
        .await
        .expect_err("unaffected batch should be rejected");

    assert!(matches!(
        result,
        Wave4RepositoryError::BatchNotAffected(id) if id == unrelated_batch_id
    ));
    let counts: (i64, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1),
            (SELECT status FROM temperature_excursion_events
              WHERE owner_id = $1 AND external_event_id = 'TEMP-EXT-W4-002')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (0, "pending_disposition".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn temperature_excursion_disposition_audit_event_is_append_only_and_hash_chain_seals(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 11, 0, 0)
        .single()
        .expect("valid time");
    let batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    let event_id =
        seed_temperature_excursion(&pool, owner_id, "TEMP-EXT-W4-003", vec![batch_id], now).await;
    let mut audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "dispose_temperature_excursion",
        "M5",
        "temperature_excursion",
        event_id.to_string(),
        None,
    );
    audit.occurred_at = now;

    repo.dispose_temperature_excursion_and_quarantine_batches(
        &ctx,
        "TEMP-EXT-W4-003",
        vec![batch_id],
        now,
        Some(audit),
    )
    .await
    .expect("temperature excursion disposition should commit");

    let update_result =
        sqlx::query("UPDATE audit_event SET action = 'tampered' WHERE owner_id = $1")
            .bind(owner_id)
            .execute(&pool)
            .await;
    assert!(
        update_result.is_err(),
        "audit_event append_only invariant must reject UPDATE"
    );

    let delete_result = sqlx::query("DELETE FROM audit_event WHERE owner_id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await;
    assert!(
        delete_result.is_err(),
        "audit_event append_only invariant must reject DELETE"
    );

    let seal = seal_audit_chain(&pool, now.date_naive(), now)
        .await
        .expect("Wave 4 audit hash chain should seal");
    assert_eq!(seal.seal_date, now.date_naive());
}
