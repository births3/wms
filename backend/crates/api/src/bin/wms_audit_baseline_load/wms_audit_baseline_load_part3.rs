fn day_for_batch(start_date: NaiveDate, days: i64, batch_index: i64) -> NaiveDate {
    start_date + Duration::days(batch_index % days)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BatchWindow {
    day: NaiveDate,
    sequence_offset: i64,
    rows: i64,
}

fn next_batch_window(
    start_date: NaiveDate,
    days: i64,
    batch_size: i64,
    current_rows: i64,
    inserted_total: i64,
    rows_remaining: i64,
) -> BatchWindow {
    let sequence_offset = current_rows + inserted_total;
    let batch_index = sequence_offset / batch_size;
    let rows_already_in_batch = sequence_offset % batch_size;
    let batch_capacity = batch_size - rows_already_in_batch;
    BatchWindow {
        day: day_for_batch(start_date, days, batch_index),
        sequence_offset,
        rows: rows_remaining.min(batch_capacity),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DayState {
    day: NaiveDate,
    last_self_hash: Option<String>,
}

impl DayState {
    #[allow(clippy::type_complexity)]
    fn build_requests(
        &self,
        args: &BaselineLoadArgs,
        sequence_offset: i64,
        rows: i64,
    ) -> Result<
        (
            Vec<(AuditWriteRequest, String, Option<String>)>,
            Option<String>,
        ),
        io::Error,
    > {
        let requests = baseline_requests(
            args,
            self.day,
            sequence_offset,
            rows,
            self.last_self_hash.as_deref(),
        )?;
        let committed_hash = requests.last().map(|row| row.1.clone());
        Ok((requests, committed_hash))
    }

    fn mark_committed(&mut self, committed_hash: Option<String>) {
        if let Some(hash) = committed_hash {
            self.last_self_hash = Some(hash);
        }
    }
}

fn baseline_requests(
    args: &BaselineLoadArgs,
    day: NaiveDate,
    global_offset: i64,
    rows: i64,
    initial_prev_hash: Option<&str>,
) -> Result<Vec<(AuditWriteRequest, String, Option<String>)>, io::Error> {
    let actor_id = fixed_uuid("00000000-0000-4000-8000-000000000001")?;
    let owner_id = fixed_uuid("00000000-0000-4000-8000-000000000002")?;
    let mut prev_hash = initial_prev_hash.map(str::to_string);
    let mut requests = Vec::with_capacity(rows as usize);
    for idx in 0..rows {
        let sequence = global_offset + idx + 1;
        let occurred_at = timestamp_for(day, sequence)?;
        let request = AuditWriteRequest {
            occurred_at,
            actor_id,
            actor_name: BASELINE_ACTOR_NAME.to_string(),
            owner_id,
            jti: format!("baseline:{}:{sequence}", args.run_id),
            action: BASELINE_ACTION.to_string(),
            module: "W6.A".to_string(),
            resource_type: "audit_baseline_material".to_string(),
            resource_id: format!("{}:{sequence}", args.run_id),
            diff: None,
            request_id: Some(deterministic_request_uuid(sequence)?),
            ip: None,
            user_agent: Some("wms-audit-baseline-load".to_string()),
        };
        let self_hash = request.compute_self_hash(prev_hash.as_deref());
        requests.push((request, self_hash.clone(), prev_hash.clone()));
        prev_hash = Some(self_hash);
    }
    Ok(requests)
}

async fn insert_requests(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    requests: Vec<(AuditWriteRequest, String, Option<String>)>,
) -> Result<(), io::Error> {
    if requests.is_empty() {
        return Ok(());
    }
    let mut builder = QueryBuilder::new(
        r#"
        INSERT INTO audit_event (
            occurred_at,
            actor_id,
            actor_name,
            owner_id,
            jti,
            action,
            module,
            resource_type,
            resource_id,
            diff,
            request_id,
            ip,
            user_agent,
            prev_hash,
            self_hash
        )
        "#,
    );
    push_insert_request_values(&mut builder, requests);
    builder
        .build()
        .execute(&mut **tx)
        .await
        .map_err(database_error)?;
    Ok(())
}

fn push_insert_request_values(
    builder: &mut QueryBuilder<'_, Postgres>,
    requests: Vec<(AuditWriteRequest, String, Option<String>)>,
) {
    builder.push_values(requests, |mut row, (request, self_hash, prev_hash)| {
        row.push_bind(request.occurred_at)
            .push_bind(request.actor_id)
            .push_bind(request.actor_name)
            .push_bind(request.owner_id)
            .push_bind(request.jti)
            .push_bind(request.action)
            .push_bind(request.module)
            .push_bind(request.resource_type)
            .push_bind(request.resource_id)
            .push_bind(Option::<serde_json::Value>::None)
            .push_bind(request.request_id)
            .push_bind(request.ip)
            .push_unseparated("::inet")
            .push_bind(request.user_agent)
            .push_bind(prev_hash)
            .push_bind(self_hash);
    });
}

fn audit_event_day_lock_key(day: NaiveDate) -> String {
    format!("audit_event:{day}")
}

fn timestamp_for(day: NaiveDate, sequence: i64) -> Result<DateTime<Utc>, io::Error> {
    let base = day
        .and_hms_micro_opt(0, 0, 0, 0)
        .ok_or_else(|| invalid("invalid baseline date"))?;
    let micros = sequence.rem_euclid(86_400_000_000);
    Ok(Utc.from_utc_datetime(&(base + Duration::microseconds(micros))))
}

fn deterministic_request_uuid(sequence: i64) -> Result<Uuid, io::Error> {
    let suffix = format!("{:012x}", sequence.max(0));
    fixed_uuid(&format!("00000000-0000-4000-8000-{suffix}"))
}

fn fixed_uuid(value: &str) -> Result<Uuid, io::Error> {
    Uuid::parse_str(value).map_err(|error| invalid(format!("invalid fixed UUID: {error}")))
}

fn write_summary(
    path: &PathBuf,
    plan: &BaselinePlan,
    database_facts: &DatabaseFacts,
    inserted_rows: i64,
    force_summary: bool,
) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = summary_payload(plan, database_facts, inserted_rows);
    let serialized = serde_json::to_string_pretty(&payload)? + "\n";
    if force_summary {
        std::fs::write(path, serialized)
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(serialized.as_bytes())
    }
}

fn summary_payload(
    plan: &BaselinePlan,
    database_facts: &DatabaseFacts,
    inserted_rows: i64,
) -> serde_json::Value {
    json!({
        "artifact_type": "baseline_loader_material",
        "not_runtime_evidence": true,
        "cannot_close_gate": "W6.A",
        "writes_evidence_json": plan.writes_evidence_json,
        "environment": "dev",
        "run_id": plan.run_id,
        "current_rows_before": plan.current_rows,
        "target_total_rows": plan.target_total_rows,
        "planned_rows_to_insert": plan.rows_to_insert,
        "planned_batches": plan.planned_batches,
        "inserted_rows": inserted_rows,
        "start_date": plan.start_date,
        "end_date": plan.end_date,
        "days": plan.days,
        "batch_size": plan.batch_size,
        "rows_per_day": plan.rows_per_day.iter().map(|daily| json!({
            "date": daily.date,
            "planned_rows": daily.planned_rows,
        })).collect::<Vec<_>>(),
        "execute_requires_external_capacity_confirmation": plan.execute && plan.rows_to_insert > 0,
        "database_facts_before": {
            "database_name": database_facts.database_name,
            "current_schema": database_facts.current_schema,
            "server_version": database_facts.server_version,
            "audit_event_partition_count": database_facts.audit_event_partition_count,
            "database_size_bytes": database_facts.database_size_bytes_before,
            "target_partition_months_required": database_facts.target_partition_months_required,
            "target_partition_months_existing": database_facts.target_partition_months_existing,
            "target_partition_months_missing": database_facts.target_partition_months_missing,
            "target_partition_months_missing_count": database_facts.target_partition_months_missing.len(),
            "target_dates_without_partition_coverage": database_facts.target_dates_without_partition_coverage,
            "target_dates_without_partition_coverage_count": database_facts.target_dates_without_partition_coverage.len(),
        },
        "execute": plan.execute,
    })
}

fn database_error(error: sqlx::Error) -> io::Error {
    io::Error::other(format!("{error:?}"))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn print_usage() {
    println!(
        "Usage: wms-audit-baseline-load --target-total-rows N [--database-url URL] [--start-date YYYY-MM-DD] [--days N] [--batch-size N] [--run-id ID] [--summary-output artifacts/dev/wave1/h2/file.json] [--force-summary] [--execute --i-understand-this-is-not-evidence]"
    );
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, Utc};
    use serde_json::json;
    use sqlx::{Execute, Postgres, QueryBuilder};
    use std::{
        env, fs,
        io::ErrorKind,
        net::{IpAddr, Ipv4Addr},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        allowed_execute_dev_hosts, build_plan, database_boundary_value, database_host,
        dates_without_partition_coverage, day_for_batch, missing_partition_months,
        next_batch_window, parse_args, push_insert_request_values, required_partition_months,
        summary_payload, validate_dev_database_url, validate_dev_database_url_with_resolver,
        validate_execution_guards, validate_summary_output, write_summary, BaselineLoadArgs,
        BaselinePlan, DatabaseFacts, DayState, DEV_DB_HOST_ALLOWLIST_ENV,
    };

    fn valid_args(execute: bool, confirmed_not_evidence: bool) -> BaselineLoadArgs {
        BaselineLoadArgs {
            database_url: "postgres://wms@pg-dev.wms.internal:5432/wms_dev".to_string(),
            target_total_rows: 100,
            start_date: Utc::now().date_naive() - Duration::days(8),
            days: 7,
            batch_size: 10,
            run_id: "TEST-RUN".to_string(),
            summary_output: PathBuf::from("artifacts/dev/wave1/h2/baseline-loader-TEST.json"),
            force_summary: false,
            execute,
            confirmed_not_evidence,
        }
    }

    #[test]
    fn parse_args_accepts_boolean_execute_guards() {
        let args = parse_args(
            [
                "--target-total-rows".to_string(),
                "100".to_string(),
                "--execute".to_string(),
                "--i-understand-this-is-not-evidence".to_string(),
            ]
            .into_iter(),
        )
        .expect("args should parse");

        assert_eq!(
            args.get("target-total-rows").map(String::as_str),
            Some("100")
        );
        assert_eq!(args.get("execute").map(String::as_str), Some("true"));
        assert_eq!(
            args.get("i-understand-this-is-not-evidence")
                .map(String::as_str),
            Some("true"),
        );
    }

    #[test]
    fn parse_args_accepts_force_summary_guard() {
        let args = parse_args(
            [
                "--target-total-rows".to_string(),
                "100".to_string(),
                "--force-summary".to_string(),
            ]
            .into_iter(),
        )
        .expect("args should parse");

        assert_eq!(args.get("force-summary").map(String::as_str), Some("true"));
    }

    #[test]
    fn execute_requires_explicit_not_evidence_confirmation() {
        let args = valid_args(true, false);

        let err = validate_execution_guards(&args).expect_err("execute must be guarded");

        assert!(err.to_string().contains("not-evidence"));
    }

    #[test]
    fn dry_run_alias_is_rejected_only_for_execute() {
        assert!(validate_dev_database_url(
            "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
            false
        )
        .is_ok());
        assert!(validate_dev_database_url(
            "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
            true
        )
        .is_err());
    }

    #[test]
    fn database_url_rejects_non_dev_or_forbidden_boundaries() {
        assert!(
            validate_dev_database_url("postgres://wms@pg-staging.wms.internal/wms_dev", false)
                .is_err()
        );
        assert!(validate_dev_database_url("postgres://wms@localhost:5432/wms_dev", false).is_err());
        assert!(
            validate_dev_database_url("postgres://wms@pg-dev.wms.internal/wms_dev", false).is_ok()
        );
        assert!(validate_dev_database_url("postgres://wms@10.0.0.8:5432/wms_dev", false).is_err());
    }

    #[test]
    fn database_url_uses_host_not_path_for_dev_boundary() {
        assert!(validate_dev_database_url("postgres://wms@10.0.0.8:5432/wms_dev", false).is_err());
        assert!(
            validate_dev_database_url("postgres://wms@pg-main.wms.internal/wms_dev", false)
                .is_err()
        );
    }

    #[test]
    fn execute_database_url_must_match_dev_host_allowlist() {
        with_env_lock(|| {
            env::remove_var(DEV_DB_HOST_ALLOWLIST_ENV);

            assert!(validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8))]),
            )
            .is_ok());
            assert!(validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev-alt.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8))]),
            )
            .is_err());

            env::set_var(DEV_DB_HOST_ALLOWLIST_ENV, "pg-dev-alt.wms.internal");
            assert!(validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev-alt.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8))]),
            )
            .is_ok());
            assert!(validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8))]),
            )
            .is_err());
        });
    }

    #[test]
    fn dev_host_allowlist_rejects_dry_run_alias_and_raw_ip() {
        with_env_lock(|| {
            env::set_var(DEV_DB_HOST_ALLOWLIST_ENV, "dev-h2.wms.internal");
            assert!(allowed_execute_dev_hosts()
                .expect_err("dry-run alias should be rejected")
                .to_string()
                .contains("dry-run alias"));

            env::set_var(DEV_DB_HOST_ALLOWLIST_ENV, "10.0.0.8");
            assert!(allowed_execute_dev_hosts()
                .expect_err("raw IP should be rejected")
                .to_string()
                .contains("raw IP"));
        });
    }

    #[test]
    fn execute_database_url_rejects_dev_dns_that_resolves_to_loopback() {
        with_env_lock(|| {
            env::remove_var(DEV_DB_HOST_ALLOWLIST_ENV);

            let err = validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]),
            )
            .expect_err("loopback-resolved dev DNS should fail");

            assert!(err.to_string().contains("loopback"));
        });
    }

    #[test]
    fn execute_database_url_accepts_allowlisted_dev_dns_that_resolves_off_loopback() {
        with_env_lock(|| {
            env::remove_var(DEV_DB_HOST_ALLOWLIST_ENV);

            assert!(validate_dev_database_url_with_resolver(
                "postgres://wms@pg-dev.wms.internal/wms_dev",
                true,
                |_| Ok(vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8))]),
            )
            .is_ok());
        });
    }

    #[test]
    fn dry_run_database_url_does_not_require_dns_resolution() {
        assert!(validate_dev_database_url_with_resolver(
            "postgres://wms@dev-h2.wms.internal:15432/wms_dev_h2",
            false,
            |_| Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "resolver should not be required for dry-run",
            )),
        )
        .is_ok());
    }

    #[test]
    fn database_boundary_value_removes_credentials_before_validation() {
        assert_eq!(
            database_boundary_value("postgres://wms:prod-like-secret@pg-dev.wms.internal/wms_dev"),
            "pg-dev.wms.internal/wms_dev",
        );
        assert_eq!(
            database_host("postgres://wms:prod-like-secret@pg-dev.wms.internal:5432/wms_dev")
                .expect("host should parse"),
            "pg-dev.wms.internal",
        );
        assert!(validate_dev_database_url(
            "postgres://wms:prod-like-secret@pg-dev.wms.internal/wms_dev",
            false,
        )
        .is_ok());
    }

    #[test]
    fn summary_output_cannot_look_like_runtime_evidence() {
        assert!(validate_summary_output(&PathBuf::from(
            "artifacts/dev/wave1/h2/baseline-loader.json"
        ))
        .is_ok());
        assert!(validate_summary_output(&PathBuf::from(
            "docs/retros/wave-1-h2-runtime-evidence.json"
        ))
        .is_err());
        assert!(
            validate_summary_output(&PathBuf::from("artifacts/dev/wave1/h2/../escape.json"))
                .is_err()
        );
    }

    #[test]
    fn plan_inserts_only_missing_rows() {
        let args = valid_args(false, false);

        let plan = build_plan(&args, 40);
        let no_op = build_plan(&args, 120);

        assert_eq!(plan.rows_to_insert, 60);
        assert_eq!(no_op.rows_to_insert, 0);
        assert!(!plan.writes_evidence_json);
    }

    #[test]
    fn plan_reports_batches_and_rows_per_day_for_capacity_review() {
        let mut args = valid_args(false, false);
        args.target_total_rows = 60_000_000;
        args.batch_size = 4_000;
        args.days = 7;

        let plan = build_plan(&args, 0);

        assert_eq!(plan.planned_batches, 15_000);
        assert_eq!(plan.end_date, args.start_date + Duration::days(6));
        assert_eq!(plan.rows_per_day.len(), 7);
        assert_eq!(plan.rows_per_day[0].planned_rows, 8_572_000);
        assert_eq!(plan.rows_per_day[5].planned_rows, 8_572_000);
        assert_eq!(plan.rows_per_day[6].planned_rows, 8_568_000);
        assert_eq!(
            plan.rows_per_day
                .iter()
                .map(|daily| daily.planned_rows)
                .sum::<i64>(),
            60_000_000,
        );
    }

    #[test]
    fn plan_resumes_rows_per_day_from_current_batch_position() {
        let mut args = valid_args(false, false);
        args.target_total_rows = 10;
        args.batch_size = 4;
        args.days = 3;

        let plan = build_plan(&args, 5);

        assert_eq!(
            plan.rows_per_day
                .iter()
                .map(|daily| daily.planned_rows)
                .collect::<Vec<_>>(),
            vec![0, 3, 2],
        );
    }

    #[test]
    fn plan_counts_partial_resume_batch_as_its_own_batch() {
        let mut args = valid_args(false, false);
        args.target_total_rows = 13;
        args.batch_size = 4;
        args.days = 3;

        let plan = build_plan(&args, 5);

        assert_eq!(plan.rows_to_insert, 8);
        assert_eq!(plan.planned_batches, 3);
    }

    #[test]
    fn batch_size_is_capped_under_postgres_parameter_limit() {
        let mut args = valid_args(false, false);
        args.batch_size = 4_001;

        let err = args.validate().expect_err("oversized batch should fail");

        assert!(err.to_string().contains("--batch-size"));
    }

    #[test]
    fn baseline_window_must_be_fully_before_today() {
        let mut args = valid_args(false, false);
        args.start_date = Utc::now().date_naive();
        args.days = 1;

        let err = args.validate().expect_err("today should not be loadable");

        assert!(err.to_string().contains("fully before today"));
    }

    #[test]
    fn batch_date_rotates_by_batch_index_not_inserted_row_count() {
        let start_date = Utc::now().date_naive() - Duration::days(8);
        let days: Vec<_> = (0..4)
            .map(|batch_index| day_for_batch(start_date, 7, batch_index))
            .collect();

        assert_eq!(days[0], start_date);
        assert_eq!(days[1], start_date + Duration::days(1));
        assert_eq!(days[2], start_date + Duration::days(2));
        assert_eq!(days[3], start_date + Duration::days(3));
    }

    #[test]
    fn next_batch_window_resumes_sequence_and_completes_partial_batch() {
        let start_date = Utc::now().date_naive() - Duration::days(8);

        let window = next_batch_window(start_date, 3, 4, 5, 0, 5);
        let second_window = next_batch_window(start_date, 3, 4, 5, window.rows, 2);

        assert_eq!(window.day, start_date + Duration::days(1));
        assert_eq!(window.sequence_offset, 5);
        assert_eq!(window.rows, 3);
        assert_eq!(second_window.day, start_date + Duration::days(2));
        assert_eq!(second_window.sequence_offset, 8);
        assert_eq!(second_window.rows, 2);
    }

    #[test]
    fn baseline_requests_can_resume_sequence_without_reusing_ids_or_timestamps() {
        let args = valid_args(false, false);
        let day = Utc::now().date_naive() - Duration::days(8);

        let requests = super::baseline_requests(&args, day, 5, 2, None)
            .expect("baseline requests should build from resume sequence");

        assert_eq!(requests[0].0.jti, "baseline:TEST-RUN:6");
        assert_eq!(requests[0].0.resource_id, "TEST-RUN:6");
        assert_eq!(
            requests[0].0.request_id,
            Some(super::deterministic_request_uuid(6).expect("uuid should build"))
        );
        assert_eq!(
            requests[0].0.occurred_at,
            super::timestamp_for(day, 6).expect("timestamp should build")
        );
        assert_eq!(requests[1].0.jti, "baseline:TEST-RUN:7");
    }

    #[test]
    fn day_state_uses_committed_hash_for_next_batch_without_cross_day_leakage() {
        let args = valid_args(false, false);
        let day = Utc::now().date_naive() - Duration::days(8);
        let mut day_state = DayState {
            day,
            last_self_hash: None,
        };
        let other_day_state = DayState {
            day: day + Duration::days(1),
            last_self_hash: None,
        };

        let (first_batch, committed_hash) = day_state
            .build_requests(&args, 0, 2)
            .expect("first batch should build");

        assert_eq!(first_batch[0].2, None);
        assert_eq!(day_state.last_self_hash, None);

        day_state.mark_committed(committed_hash.clone());
        let (second_batch, _) = day_state
            .build_requests(&args, 2, 1)
            .expect("second batch should build");
        let (other_day_batch, _) = other_day_state
            .build_requests(&args, 2, 1)
            .expect("other day batch should build");

        assert_eq!(second_batch[0].2, committed_hash);
        assert_eq!(other_day_batch[0].2, None);
    }

    #[test]
    fn baseline_insert_casts_ip_bind_to_inet() {
        let args = valid_args(false, false);
        let day = Utc::now().date_naive() - Duration::days(8);
        let requests = super::baseline_requests(&args, day, 0, 1, None)
            .expect("baseline request should build");
        let mut builder = QueryBuilder::<Postgres>::new(
            r#"
            INSERT INTO audit_event (
                occurred_at,
                actor_id,
                actor_name,
                owner_id,
                jti,
                action,
                module,
                resource_type,
                resource_id,
                diff,
                request_id,
                ip,
                user_agent,
                prev_hash,
                self_hash
            )
            "#,
        );

        push_insert_request_values(&mut builder, requests);
        let query = builder.build();

        assert!(
            query.sql().contains(", $12::inet, $13"),
            "baseline batch insert must cast ip bind to match audit_event.ip inet column: {}",
            query.sql()
        );
    }

    #[test]
    fn summary_output_refuses_to_overwrite_without_force_summary() {
        let path = unique_temp_summary_path();
        let plan = sample_plan();
        let database_facts = sample_database_facts();
        write_summary(&path, &plan, &database_facts, 0, false)
            .expect("first write should create summary");

        let err = write_summary(&path, &plan, &database_facts, 0, false)
            .expect_err("second write should refuse overwrite");
        assert_eq!(err.kind(), ErrorKind::AlreadyExists);

        fs::remove_file(path).ok();
    }

    #[test]
    fn force_summary_allows_overwrite() {
        let path = unique_temp_summary_path();
        let plan = sample_plan();
        let database_facts = sample_database_facts();
        write_summary(&path, &plan, &database_facts, 0, false)
            .expect("first write should create summary");
        write_summary(&path, &plan, &database_facts, 0, true)
            .expect("force-summary should overwrite");

        fs::remove_file(path).ok();
    }

    #[test]
    fn summary_payload_includes_database_facts_before_load() {
        let payload = summary_payload(&sample_plan(), &sample_database_facts(), 0);

        assert_eq!(
            payload["database_facts_before"]["database_name"],
            "wms_dev_h2"
        );
        assert_eq!(
            payload["database_facts_before"]["audit_event_partition_count"],
            7
        );
        assert_eq!(
            payload["database_facts_before"]["database_size_bytes"],
            9_673_751
        );
        assert_eq!(
            payload["database_facts_before"]["target_partition_months_required"],
            json!(["2026-06-01", "2026-07-01"])
        );
        assert_eq!(
            payload["database_facts_before"]["target_partition_months_existing"],
            json!(["2026-06-01"])
        );
        assert_eq!(
            payload["database_facts_before"]["target_partition_months_missing"],
            json!(["2026-07-01"])
        );
        assert_eq!(
            payload["database_facts_before"]["target_partition_months_missing_count"],
            1
        );
        assert_eq!(
            payload["database_facts_before"]["target_dates_without_partition_coverage"],
            json!(["2026-07-01", "2026-07-02"])
        );
        assert_eq!(
            payload["database_facts_before"]["target_dates_without_partition_coverage_count"],
            2
        );
    }

    #[test]
    fn required_partition_months_returns_distinct_months_for_target_window() {
        let start_date = NaiveDate::from_ymd_opt(2026, 6, 30).expect("valid date");

        let months = required_partition_months(start_date, 4);

        assert_eq!(
            months,
            vec![
                NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date"),
                NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
            ],
        );
    }

    #[test]
    fn missing_partition_months_returns_required_months_without_child_tables() {
        let required = vec![
            NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date"),
            NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
        ];
        let existing = vec![NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date")];

        let missing = missing_partition_months(&required, &existing);

        assert_eq!(
            missing,
            vec![NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date")],
        );
    }

    #[test]
    fn dates_without_partition_coverage_returns_target_days_in_missing_months() {
        let start_date = NaiveDate::from_ymd_opt(2026, 6, 30).expect("valid date");
        let missing_months = vec![NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date")];

        let uncovered = dates_without_partition_coverage(start_date, 4, &missing_months);

        assert_eq!(
            uncovered,
            vec![
                NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
                NaiveDate::from_ymd_opt(2026, 7, 2).expect("valid date"),
                NaiveDate::from_ymd_opt(2026, 7, 3).expect("valid date"),
            ],
        );
    }

    fn sample_plan() -> BaselinePlan {
        build_plan(&valid_args(false, false), 0)
    }

    fn sample_database_facts() -> DatabaseFacts {
        DatabaseFacts {
            database_name: "wms_dev_h2".to_string(),
            current_schema: "public".to_string(),
            server_version: "16.0".to_string(),
            audit_event_partition_count: 7,
            database_size_bytes_before: 9_673_751,
            target_partition_months_required: vec![
                NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date"),
                NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
            ],
            target_partition_months_existing: vec![
                NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date")
            ],
            target_partition_months_missing: vec![
                NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date")
            ],
            target_dates_without_partition_coverage: vec![
                NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
                NaiveDate::from_ymd_opt(2026, 7, 2).expect("valid date"),
            ],
        }
    }

    fn unique_temp_summary_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("wms-audit-baseline-summary-{nanos}.json"))
    }

    fn with_env_lock(test: impl FnOnce()) {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
        let previous = env::var(DEV_DB_HOST_ALLOWLIST_ENV).ok();
        test();
        match previous {
            Some(value) => env::set_var(DEV_DB_HOST_ALLOWLIST_ENV, value),
            None => env::remove_var(DEV_DB_HOST_ALLOWLIST_ENV),
        }
    }
}
