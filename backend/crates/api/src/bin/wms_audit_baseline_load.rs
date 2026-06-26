use std::{
    collections::BTreeMap,
    env,
    error::Error,
    io::{self, Write},
    net::{IpAddr, ToSocketAddrs},
    path::PathBuf,
};

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde_json::json;
use sqlx::{
    postgres::{PgConnection, PgPoolOptions},
    Connection, PgPool, Postgres, QueryBuilder,
};
use uuid::Uuid;
use wms_api::audit::AuditWriteRequest;

const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const DEV_DB_HOST_ALLOWLIST_ENV: &str = "WMS_DEV_DB_HOST_ALLOWLIST";
const DRY_RUN_ALIAS: &str = "dev-h2.wms.internal";
const DEFAULT_EXECUTE_DEV_DB_HOST: &str = "pg-dev.wms.internal";
const CONFIRM_FLAG: &str = "i-understand-this-is-not-evidence";
const FORCE_SUMMARY_FLAG: &str = "force-summary";
const MAX_BATCH_SIZE: i64 = 4_000;
const DEFAULT_BATCH_SIZE: i64 = 1_000;
const DEFAULT_DAYS: i64 = 7;
const DEFAULT_OUTPUT: &str = "artifacts/dev/wave1/h2/baseline-loader-summary.json";
const BASELINE_ACTOR_NAME: &str = "wave1-h2-baseline-loader";
const BASELINE_ACTION: &str = "baseline.synthetic_event.prepared";

#[derive(Debug, Clone, Eq, PartialEq)]
struct BaselineLoadArgs {
    database_url: String,
    target_total_rows: i64,
    start_date: NaiveDate,
    days: i64,
    batch_size: i64,
    run_id: String,
    summary_output: PathBuf,
    force_summary: bool,
    execute: bool,
    confirmed_not_evidence: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BaselinePlan {
    current_rows: i64,
    target_total_rows: i64,
    rows_to_insert: i64,
    days: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
    batch_size: i64,
    planned_batches: i64,
    rows_per_day: Vec<DailyRowPlan>,
    run_id: String,
    execute: bool,
    writes_evidence_json: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DailyRowPlan {
    date: NaiveDate,
    planned_rows: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DatabaseFacts {
    database_name: String,
    current_schema: String,
    server_version: String,
    audit_event_partition_count: i64,
    database_size_bytes_before: i64,
    target_partition_months_required: Vec<NaiveDate>,
    target_partition_months_existing: Vec<NaiveDate>,
    target_partition_months_missing: Vec<NaiveDate>,
    target_dates_without_partition_coverage: Vec<NaiveDate>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_usage();
        return Ok(());
    }
    let args = BaselineLoadArgs::parse(raw_args.into_iter())?;
    validate_execution_guards(&args)?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&args.database_url)
        .await
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to connect PostgreSQL for baseline load: {error:?}"),
            )
        })?;

    ensure_baseline_schema(&pool).await?;
    let current_rows = count_audit_rows(&pool).await?;
    let plan = build_plan(&args, current_rows);
    let database_facts = collect_database_facts(&pool, &plan).await?;

    let inserted_rows = if args.execute && plan.rows_to_insert > 0 {
        ensure_target_days_unsealed(&pool, args.start_date, args.days).await?;
        ensure_partitions(&pool, args.start_date, args.days).await?;
        load_baseline_rows(&pool, &args, &plan).await?
    } else {
        0
    };

    write_summary(
        &args.summary_output,
        &plan,
        &database_facts,
        inserted_rows,
        args.force_summary,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary_payload(&plan, &database_facts, inserted_rows))?
    );
    Ok(())
}

impl BaselineLoadArgs {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, io::Error> {
        let raw = parse_args(args)?;
        let database_url = raw
            .get("database-url")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .or_else(|| database_url().ok())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--database-url or DATABASE_URL/WMS_DB_URL is required",
                )
            })?;
        let target_total_rows = required_i64(&raw, "target-total-rows")?;
        let start_date = optional_date(&raw, "start-date")?
            .unwrap_or_else(|| Utc::now().date_naive() - Duration::days(DEFAULT_DAYS));
        let days = optional_i64(&raw, "days")?.unwrap_or(DEFAULT_DAYS);
        let batch_size = optional_i64(&raw, "batch-size")?.unwrap_or(DEFAULT_BATCH_SIZE);
        let run_id = raw
            .get("run-id")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| Utc::now().format("%Y%m%dT%H%M%SZ").to_string());
        let summary_output = raw
            .get("summary-output")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT));
        let force_summary = raw.contains_key(FORCE_SUMMARY_FLAG);
        let execute = raw.contains_key("execute");
        let confirmed_not_evidence = raw.contains_key(CONFIRM_FLAG);

        let parsed = Self {
            database_url,
            target_total_rows,
            start_date,
            days,
            batch_size,
            run_id,
            summary_output,
            force_summary,
            execute,
            confirmed_not_evidence,
        };
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> Result<(), io::Error> {
        if self.target_total_rows < 1 {
            return Err(invalid("--target-total-rows must be >= 1"));
        }
        if self.days < 1 {
            return Err(invalid("--days must be >= 1"));
        }
        let last_date = self
            .start_date
            .checked_add_signed(Duration::days(self.days - 1))
            .ok_or_else(|| invalid("baseline date range overflow"))?;
        if last_date >= Utc::now().date_naive() {
            return Err(invalid(
                "baseline date range must be fully before today so seal cron can close completed days",
            ));
        }
        if self.batch_size < 1 || self.batch_size > MAX_BATCH_SIZE {
            return Err(invalid(format!(
                "--batch-size must be 1..={MAX_BATCH_SIZE}"
            )));
        }
        validate_dev_database_url(&self.database_url, self.execute)?;
        validate_summary_output(&self.summary_output)?;
        Ok(())
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<BTreeMap<String, String>, io::Error> {
    let mut parsed = BTreeMap::new();
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        if !flag.starts_with("--") {
            return Err(invalid(format!(
                "unexpected argument {flag}; expected --name value"
            )));
        }
        let key = flag.trim_start_matches("--").to_string();
        if matches!(key.as_str(), "execute" | CONFIRM_FLAG | FORCE_SUMMARY_FLAG) {
            parsed.insert(key, "true".to_string());
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| invalid(format!("missing value for --{key}")))?;
        if value.starts_with("--") {
            return Err(invalid(format!("missing value for --{key}")));
        }
        parsed.insert(key, value);
    }
    Ok(parsed)
}

fn database_url() -> Result<String, io::Error> {
    env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var(WMS_DB_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| {
            invalid(format!(
                "{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"
            ))
        })
}

fn required_i64(args: &BTreeMap<String, String>, key: &str) -> Result<i64, io::Error> {
    let raw = args
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid(format!("--{key} is required")))?;
    parse_i64(key, raw)
}

fn optional_i64(args: &BTreeMap<String, String>, key: &str) -> Result<Option<i64>, io::Error> {
    args.get(key)
        .filter(|value| !value.trim().is_empty())
        .map(|raw| parse_i64(key, raw))
        .transpose()
}

fn parse_i64(key: &str, raw: &str) -> Result<i64, io::Error> {
    raw.parse::<i64>()
        .map_err(|error| invalid(format!("--{key} must be an integer: {error}")))
}

fn optional_date(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<NaiveDate>, io::Error> {
    args.get(key)
        .filter(|value| !value.trim().is_empty())
        .map(|raw| {
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .map_err(|error| invalid(format!("--{key} must use YYYY-MM-DD: {error}")))
        })
        .transpose()
}

fn validate_execution_guards(args: &BaselineLoadArgs) -> Result<(), io::Error> {
    if args.execute && !args.confirmed_not_evidence {
        return Err(invalid(format!(
            "--execute requires --{CONFIRM_FLAG}; this loader prepares dev baseline material and does not write runtime evidence"
        )));
    }
    Ok(())
}

fn validate_dev_database_url(database_url: &str, execute: bool) -> Result<(), io::Error> {
    validate_dev_database_url_with_resolver(database_url, execute, |host| resolve_host_ips(host))?;
    Ok(())
}

fn validate_dev_database_url_with_resolver(
    database_url: &str,
    execute: bool,
    resolver: impl Fn(&str) -> Result<Vec<IpAddr>, io::Error>,
) -> Result<(), io::Error> {
    let host = database_host(database_url)?;
    let lower = host.to_ascii_lowercase();
    let forbidden = [
        "staging",
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
        "local",
        "prod",
        "production",
        "prodution",
        "mock",
        "fake",
        "stub",
        "example",
    ];
    if !host_contains_dev_token(&lower) {
        return Err(invalid("database-url host must include dev boundary token"));
    }
    if execute && lower.contains(DRY_RUN_ALIAS) {
        return Err(invalid(
            "dev-h2.wms.internal is only allowed for dry-run readiness and cannot be used with --execute",
        ));
    }
    if execute && lower.parse::<IpAddr>().is_ok() {
        return Err(invalid(
            "database-url host must be a dev DNS name, not a raw IP address",
        ));
    }
    if execute && !allowed_execute_dev_hosts()?.contains(&lower) {
        return Err(invalid(format!(
            "database-url host {host} is not in {DEV_DB_HOST_ALLOWLIST_ENV}; add the real dev PostgreSQL DNS before --execute"
        )));
    }
    if execute {
        reject_loopback_resolution(&host, resolver)?;
    }
    for token in forbidden {
        if lower.contains(token) {
            return Err(invalid(format!(
                "database-url must not point to {token} boundary"
            )));
        }
    }
    Ok(())
}

fn resolve_host_ips(host: &str) -> Result<Vec<IpAddr>, io::Error> {
    (host, 0)
        .to_socket_addrs()
        .map(|addrs| addrs.map(|addr| addr.ip()).collect())
}

fn reject_loopback_resolution(
    host: &str,
    resolver: impl Fn(&str) -> Result<Vec<IpAddr>, io::Error>,
) -> Result<(), io::Error> {
    let ips = resolver(host).map_err(|error| {
        invalid(format!(
            "database-url host {host} must resolve before --execute: {error}"
        ))
    })?;
    if ips.is_empty() {
        return Err(invalid(format!(
            "database-url host {host} did not resolve to any IP address"
        )));
    }
    if ips.iter().any(IpAddr::is_loopback) {
        return Err(invalid(format!(
            "database-url host {host} resolves to loopback and cannot be used with --execute"
        )));
    }
    Ok(())
}

fn allowed_execute_dev_hosts() -> Result<Vec<String>, io::Error> {
    let raw = env::var(DEV_DB_HOST_ALLOWLIST_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_EXECUTE_DEV_DB_HOST.to_string());
    let hosts: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(|host| host.to_ascii_lowercase())
        .collect();
    if hosts.is_empty() {
        return Err(invalid(format!(
            "{DEV_DB_HOST_ALLOWLIST_ENV} must contain at least one dev DB host"
        )));
    }
    for host in &hosts {
        if host.parse::<IpAddr>().is_ok() {
            return Err(invalid(format!(
                "{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must be a DNS name, not raw IP"
            )));
        }
        if !host_contains_dev_token(host) {
            return Err(invalid(format!(
                "{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must include dev boundary token"
            )));
        }
        if host.contains(DRY_RUN_ALIAS) {
            return Err(invalid(format!(
                "{DEV_DB_HOST_ALLOWLIST_ENV} must not include dry-run alias {DRY_RUN_ALIAS}"
            )));
        }
    }
    Ok(hosts)
}

fn database_host(database_url: &str) -> Result<String, io::Error> {
    let boundary = database_boundary_value(database_url);
    let host_port = boundary
        .split_once('/')
        .map(|(host_port, _)| host_port)
        .unwrap_or(boundary.as_str());
    let host = host_port
        .strip_prefix('[')
        .and_then(|value| value.split_once(']').map(|(host, _)| host.to_string()))
        .unwrap_or_else(|| {
            host_port
                .split_once(':')
                .map(|(host, _)| host.to_string())
                .unwrap_or_else(|| host_port.to_string())
        });
    if host.trim().is_empty() {
        return Err(invalid("database-url host is required"));
    }
    Ok(host)
}

fn database_boundary_value(database_url: &str) -> String {
    let without_scheme = database_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(database_url);
    without_scheme
        .rsplit_once('@')
        .map(|(_, boundary)| boundary)
        .unwrap_or(without_scheme)
        .to_string()
}

fn host_contains_dev_token(host: &str) -> bool {
    host.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|part| part == "dev")
}

fn validate_summary_output(path: &PathBuf) -> Result<(), io::Error> {
    let normalized = path.to_string_lossy();
    if normalized.contains("docs/retros") || normalized.contains("wave-1-h2-runtime-evidence") {
        return Err(invalid(
            "--summary-output must not point to docs/retros or runtime evidence files",
        ));
    }
    if !normalized.starts_with("artifacts/dev/wave1/h2/") {
        return Err(invalid(
            "--summary-output must be under artifacts/dev/wave1/h2/",
        ));
    }
    if normalized.split('/').any(|part| part == "..") {
        return Err(invalid(
            "--summary-output must not contain .. path segments",
        ));
    }
    Ok(())
}

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
    io::Error::new(io::ErrorKind::Other, format!("{error:?}"))
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
