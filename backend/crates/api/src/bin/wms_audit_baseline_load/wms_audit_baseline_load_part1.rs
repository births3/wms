use std::{
    collections::BTreeMap,
    env,
    error::Error,
    io::{self, Write},
    net::{IpAddr, ToSocketAddrs},
    path::{Path, PathBuf},
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
            io::Error::other(format!(
                "failed to connect PostgreSQL for baseline load: {error:?}"
            ))
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
    validate_dev_database_url_with_resolver(database_url, execute, resolve_host_ips)?;
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

fn validate_summary_output(path: &Path) -> Result<(), io::Error> {
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
