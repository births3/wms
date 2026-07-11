fn build_plan(args: &BaselineLoadArgs, current_rows: i64) -> BaselinePlan {
    let rows_to_insert = (args.target_total_rows - current_rows).max(0);
    let planned_batches = planned_batch_count(
        args.start_date,
        args.days,
        args.batch_size,
        current_rows,
        rows_to_insert,
    );
    BaselinePlan {
        current_rows,
        target_total_rows: args.target_total_rows,
        rows_to_insert,
        days: args.days,
        start_date: args.start_date,
        end_date: args.start_date + Duration::days(args.days - 1),
        batch_size: args.batch_size,
        planned_batches,
        rows_per_day: planned_rows_per_day(
            args.start_date,
            args.days,
            args.batch_size,
            current_rows,
            rows_to_insert,
        ),
        run_id: args.run_id.clone(),
        execute: args.execute,
        writes_evidence_json: false,
    }
}

fn planned_batch_count(
    start_date: NaiveDate,
    days: i64,
    batch_size: i64,
    current_rows: i64,
    rows_to_insert: i64,
) -> i64 {
    let mut inserted_total = 0;
    let mut batches = 0;
    while inserted_total < rows_to_insert {
        let window = next_batch_window(
            start_date,
            days,
            batch_size,
            current_rows,
            inserted_total,
            rows_to_insert - inserted_total,
        );
        inserted_total += window.rows;
        batches += 1;
    }
    batches
}

fn planned_rows_per_day(
    start_date: NaiveDate,
    days: i64,
    batch_size: i64,
    current_rows: i64,
    rows_to_insert: i64,
) -> Vec<DailyRowPlan> {
    let mut daily = Vec::with_capacity(days as usize);
    for offset in 0..days {
        daily.push(DailyRowPlan {
            date: start_date + Duration::days(offset),
            planned_rows: 0,
        });
    }
    let mut inserted_total = 0;
    while inserted_total < rows_to_insert {
        let window = next_batch_window(
            start_date,
            days,
            batch_size,
            current_rows,
            inserted_total,
            rows_to_insert - inserted_total,
        );
        let day_index = (window.day - start_date).num_days() as usize;
        daily[day_index].planned_rows += window.rows;
        inserted_total += window.rows;
    }
    daily
}

async fn ensure_baseline_schema(pool: &PgPool) -> Result<(), io::Error> {
    let audit_event_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('public.audit_event')::text")
            .fetch_one(pool)
            .await
            .map_err(database_error)?;
    if audit_event_exists.as_deref() != Some("audit_event") {
        return Err(invalid(
            "audit_event table is missing; run migrations first",
        ));
    }

    let partition_fn_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regprocedure('create_audit_partition(date)')::text")
            .fetch_one(pool)
            .await
            .map_err(database_error)?;
    if partition_fn_exists.as_deref() != Some("create_audit_partition(date)") {
        return Err(invalid(
            "create_audit_partition(date) is missing; run migrations first",
        ));
    }
    Ok(())
}

async fn count_audit_rows(pool: &PgPool) -> Result<i64, io::Error> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_event")
        .fetch_one(pool)
        .await
        .map_err(database_error)
}

async fn collect_database_facts(
    pool: &PgPool,
    plan: &BaselinePlan,
) -> Result<DatabaseFacts, io::Error> {
    let (database_name, current_schema, server_version, database_size_bytes_before): (
        String,
        String,
        String,
        i64,
    ) = sqlx::query_as(
        r#"
        SELECT current_database(),
               current_schema(),
               current_setting('server_version'),
               pg_database_size(current_database())::bigint
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(database_error)?;

    let audit_event_partition_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
          FROM pg_inherits
         WHERE inhparent = 'public.audit_event'::regclass
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(database_error)?;

    let target_partition_months_required = required_partition_months(plan.start_date, plan.days);
    let target_partition_months_existing =
        target_existing_partition_months(pool, &target_partition_months_required).await?;
    let target_partition_months_missing = missing_partition_months(
        &target_partition_months_required,
        &target_partition_months_existing,
    );
    let target_dates_without_partition_coverage = dates_without_partition_coverage(
        plan.start_date,
        plan.days,
        &target_partition_months_missing,
    );

    Ok(DatabaseFacts {
        database_name,
        current_schema,
        server_version,
        audit_event_partition_count,
        database_size_bytes_before,
        target_partition_months_required,
        target_partition_months_existing,
        target_partition_months_missing,
        target_dates_without_partition_coverage,
    })
}

async fn target_existing_partition_months(
    pool: &PgPool,
    required_months: &[NaiveDate],
) -> Result<Vec<NaiveDate>, io::Error> {
    let months: Vec<NaiveDate> = sqlx::query_scalar(
        r#"
        SELECT to_date(substring(c.relname from 'audit_event_([0-9]{4}_[0-9]{2})$'), 'YYYY_MM')::date
          FROM pg_inherits i
          JOIN pg_class c ON c.oid = i.inhrelid
         WHERE i.inhparent = 'public.audit_event'::regclass
           AND c.relname ~ '^audit_event_[0-9]{4}_[0-9]{2}$'
         ORDER BY 1
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    Ok(months
        .into_iter()
        .filter(|month| required_months.contains(month))
        .collect())
}

fn required_partition_months(start_date: NaiveDate, days: i64) -> Vec<NaiveDate> {
    let mut months = Vec::new();
    for offset in 0..days {
        let month = month_start(start_date + Duration::days(offset));
        if months.last() != Some(&month) {
            months.push(month);
        }
    }
    months
}

fn missing_partition_months(required: &[NaiveDate], existing: &[NaiveDate]) -> Vec<NaiveDate> {
    required
        .iter()
        .copied()
        .filter(|month| !existing.contains(month))
        .collect()
}

fn dates_without_partition_coverage(
    start_date: NaiveDate,
    days: i64,
    missing_months: &[NaiveDate],
) -> Vec<NaiveDate> {
    let mut uncovered = Vec::new();
    for offset in 0..days {
        let day = start_date + Duration::days(offset);
        if missing_months.contains(&month_start(day)) {
            uncovered.push(day);
        }
    }
    uncovered
}

fn month_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.day0()))
}

async fn ensure_partitions(
    pool: &PgPool,
    start_date: NaiveDate,
    days: i64,
) -> Result<(), io::Error> {
    for offset in 0..days {
        let day = start_date + Duration::days(offset);
        sqlx::query("SELECT create_audit_partition($1)")
            .bind(day)
            .execute(pool)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

async fn ensure_target_days_unsealed(
    pool: &PgPool,
    start_date: NaiveDate,
    days: i64,
) -> Result<(), io::Error> {
    let end_date = start_date
        .checked_add_signed(Duration::days(days))
        .ok_or_else(|| invalid("baseline date range overflow"))?;
    let sealed_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
          FROM audit_chain_seal
         WHERE seal_date >= $1
           AND seal_date < $2
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .map_err(database_error)?;
    if sealed_count > 0 {
        return Err(invalid(
            "baseline date range already contains audit_chain_seal rows; refusing to append audit_event to sealed days",
        ));
    }
    Ok(())
}

async fn load_baseline_rows(
    pool: &PgPool,
    args: &BaselineLoadArgs,
    plan: &BaselinePlan,
) -> Result<i64, io::Error> {
    let mut conn = pool.acquire().await.map_err(database_error)?;
    let load_result = load_baseline_rows_with_session(&mut conn, args, plan).await;
    let unlock_result = release_day_locks(&mut conn, args.start_date, args.days).await;
    if load_result.is_err() || unlock_result.is_err() {
        let _ = conn.close().await;
    }
    match load_result {
        Ok(inserted_rows) => {
            unlock_result?;
            Ok(inserted_rows)
        }
        Err(error) => Err(error),
    }
}

async fn load_baseline_rows_with_session(
    conn: &mut PgConnection,
    args: &BaselineLoadArgs,
    plan: &BaselinePlan,
) -> Result<i64, io::Error> {
    acquire_day_locks(conn, args.start_date, args.days).await?;
    let mut day_states = initialize_day_states(conn, args.start_date, args.days).await?;

    let mut inserted_total = 0;
    while inserted_total < plan.rows_to_insert {
        let window = next_batch_window(
            args.start_date,
            args.days,
            args.batch_size,
            plan.current_rows,
            inserted_total,
            plan.rows_to_insert - inserted_total,
        );
        let day_state = day_states.get_mut(&window.day).ok_or_else(|| {
            invalid(format!(
                "baseline day state missing for {} while loading",
                window.day
            ))
        })?;
        let inserted =
            insert_batch_for_day(conn, args, day_state, window.sequence_offset, window.rows)
                .await?;
        inserted_total += inserted;
    }
    Ok(inserted_total)
}

async fn acquire_day_locks(
    conn: &mut PgConnection,
    start_date: NaiveDate,
    days: i64,
) -> Result<(), io::Error> {
    for offset in 0..days {
        let day = start_date + Duration::days(offset);
        sqlx::query("SELECT pg_advisory_lock(hashtext($1))")
            .bind(audit_event_day_lock_key(day))
            .execute(&mut *conn)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

async fn release_day_locks(
    conn: &mut PgConnection,
    start_date: NaiveDate,
    days: i64,
) -> Result<(), io::Error> {
    for offset in (0..days).rev() {
        let day = start_date + Duration::days(offset);
        let _: bool = sqlx::query_scalar("SELECT pg_advisory_unlock(hashtext($1))")
            .bind(audit_event_day_lock_key(day))
            .fetch_one(&mut *conn)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

async fn initialize_day_states(
    conn: &mut PgConnection,
    start_date: NaiveDate,
    days: i64,
) -> Result<BTreeMap<NaiveDate, DayState>, io::Error> {
    let mut states = BTreeMap::new();
    for offset in 0..days {
        let day = start_date + Duration::days(offset);
        let mut tx = conn.begin().await.map_err(database_error)?;
        ensure_day_unsealed_in_tx(&mut tx, day).await?;
        ensure_day_contains_only_baseline_events(&mut tx, day).await?;
        let last_self_hash = latest_self_hash_for_day_in_tx(&mut tx, day).await?;
        tx.commit().await.map_err(database_error)?;
        states.insert(
            day,
            DayState {
                day,
                last_self_hash,
            },
        );
    }
    Ok(states)
}

async fn insert_batch_for_day(
    conn: &mut PgConnection,
    args: &BaselineLoadArgs,
    day_state: &mut DayState,
    sequence_offset: i64,
    rows: i64,
) -> Result<i64, io::Error> {
    let (requests, committed_hash) = day_state.build_requests(args, sequence_offset, rows)?;
    let mut tx = conn.begin().await.map_err(database_error)?;
    insert_requests(&mut tx, requests).await?;
    tx.commit().await.map_err(database_error)?;
    day_state.mark_committed(committed_hash);
    Ok(rows)
}

async fn latest_self_hash_for_day_in_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    day: NaiveDate,
) -> Result<Option<String>, io::Error> {
    sqlx::query_scalar(
        r#"
        SELECT self_hash
          FROM audit_event
         WHERE occurred_at >= $1::date
           AND occurred_at < $1::date + interval '1 day'
         ORDER BY id DESC
         LIMIT 1
        "#,
    )
    .bind(day)
    .fetch_optional(&mut **tx)
    .await
    .map_err(database_error)
}

async fn ensure_day_unsealed_in_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    day: NaiveDate,
) -> Result<(), io::Error> {
    let sealed_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
          FROM audit_chain_seal
         WHERE seal_date = $1
        "#,
    )
    .bind(day)
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?;
    if sealed_count > 0 {
        return Err(invalid(
            "target day already has audit_chain_seal; refusing to append baseline events",
        ));
    }
    Ok(())
}

async fn ensure_day_contains_only_baseline_events(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    day: NaiveDate,
) -> Result<(), io::Error> {
    let non_baseline_count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
          FROM audit_event
         WHERE occurred_at >= $1::date
           AND occurred_at < $1::date + interval '1 day'
           AND NOT (
             actor_name = $2
             AND action = $3
           )
        "#,
    )
    .bind(day)
    .bind(BASELINE_ACTOR_NAME)
    .bind(BASELINE_ACTION)
    .fetch_one(&mut **tx)
    .await
    .map_err(database_error)?;
    if non_baseline_count > 0 {
        return Err(invalid(
            "target day already contains non-baseline audit_event rows; refusing to mix synthetic baseline with real audit chain",
        ));
    }
    Ok(())
}
