use std::{collections::BTreeMap, env, error::Error, io};

use chrono::Utc;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;
use wms_api::{
    audit::append_event,
    deploy_audit::{build_deploy_audit_request, validate_deploy_audit_input, DeployAuditInput},
};

const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const BLOCKED_DB_URL_TOKENS: [&str; 11] = [
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "local",
    "dev",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "example",
];
const FROM_ENV_ARG_MAP: [(&str, &str); 21] = [
    ("WAVE_6_ENVIRONMENT", "environment"),
    ("WAVE_6_DEPLOYMENT_MODE", "deployment-mode"),
    ("WAVE_6_DEPLOY_MODULE", "module"),
    ("WAVE_6_DEPLOY_ACTION", "action"),
    ("WAVE_6_DEPLOY_RESOURCE_TYPE", "resource-type"),
    ("WAVE_6_DEPLOY_RESOURCE_ID", "resource-id"),
    ("WAVE_6_RELEASE_VERSION", "release-version"),
    ("WAVE_6_RELEASE_PLAN_REF", "release-plan-ref"),
    ("WAVE_6_ARTIFACT_REF", "artifact-ref"),
    ("WAVE_6_CANARY_CONFIG_REF", "canary-config-ref"),
    ("WAVE_6_SMOKE_GATE_REF", "smoke-gate-ref"),
    (
        "WAVE_6_OBSERVABILITY_DASHBOARD_REF",
        "observability-dashboard-ref",
    ),
    ("WAVE_6_ROLLBACK_DRILL_LOG_REF", "rollback-drill-log-ref"),
    ("WAVE_6_APPROVAL_RECORD_REF", "approval-record-ref"),
    ("WAVE_6_CANARY_STAGES_EXERCISED", "canary-stages-exercised"),
    ("WAVE_6_SMOKE_CHECKS_PASSED", "smoke-checks-passed"),
    (
        "WAVE_6_ROLLBACK_DRILLS_EXERCISED",
        "rollback-drills-exercised",
    ),
    ("WAVE_6_DEPLOY_ACTOR_ID", "actor-id"),
    ("WAVE_6_DEPLOY_ACTOR_NAME", "actor-name"),
    ("WAVE_6_DEPLOY_OWNER_ID", "owner-id"),
    ("WAVE_6_DEPLOY_JTI", "jti"),
];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = parse_args(env::args().skip(1))?;
    if bool_flag(&args, "help") {
        print_usage();
        return Ok(());
    }
    if bool_flag(&args, "from-env") {
        let missing = apply_from_env(&mut args, |name| env::var(name).ok());
        if !missing.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "missing Wave 6 deploy audit environment variables: {}",
                    missing.join(", "),
                ),
            )
            .into());
        }
    }
    let input = DeployAuditInput {
        module: required(&args, "module")?,
        action: required(&args, "action")?,
        resource_type: required(&args, "resource-type")?,
        resource_id: required(&args, "resource-id")?,
        environment: required(&args, "environment")?,
        deployment_mode: required(&args, "deployment-mode")?,
        release_version: required(&args, "release-version")?,
        release_plan_ref: required(&args, "release-plan-ref")?,
        artifact_ref: required(&args, "artifact-ref")?,
        canary_config_ref: required(&args, "canary-config-ref")?,
        smoke_gate_ref: required(&args, "smoke-gate-ref")?,
        observability_dashboard_ref: required(&args, "observability-dashboard-ref")?,
        rollback_drill_log_ref: required(&args, "rollback-drill-log-ref")?,
        approval_record_ref: required(&args, "approval-record-ref")?,
        canary_stages_exercised: positive_i64(&args, "canary-stages-exercised")?,
        smoke_checks_passed: positive_i64(&args, "smoke-checks-passed")?,
        rollback_drills_exercised: positive_i64(&args, "rollback-drills-exercised")?,
    };
    validate_deploy_audit_input(&input).map_err(|message| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid Wave 6 deploy audit input: {message}"),
        )
    })?;
    let environment = input.environment.clone();
    let module = input.module.clone();
    let action = input.action.clone();
    let mut request = build_deploy_audit_request(
        input,
        uuid_arg(&args, "actor-id")?,
        required(&args, "actor-name")?,
        uuid_arg(&args, "owner-id")?,
        required(&args, "jti")?,
        Utc::now(),
    );
    request.request_id = optional_uuid_arg(&args, "request-id")?;
    request.ip = optional(&args, "ip");

    if bool_flag(&args, "check-only") {
        println!(
            "{}",
            serde_json::to_string_pretty(&preview_payload(&environment, &module, &action))?
        );
        return Ok(());
    }

    let database_url = database_url()?;
    validate_staging_database_url(&database_url)?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .map_err(|error| {
            io::Error::other(format!(
                "failed to connect PostgreSQL for deploy audit: {error:?}"
            ))
        })?;

    ensure_current_audit_partition(&pool).await?;

    let inserted = append_event(&pool, &request)
        .await
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    let audit_event_query_ref = audit_event_query_ref(&environment, &inserted);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "audit_event_query_ref": audit_event_query_ref,
            "audit_event_id": inserted.id,
            "occurred_at": inserted.occurred_at,
            "module": inserted.module,
            "action": inserted.action,
            "resource_type": inserted.resource_type,
            "resource_id": inserted.resource_id,
            "owner_id": inserted.owner_id,
            "self_hash": inserted.self_hash,
        }))?
    );

    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    match env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => Ok(value),
        None => env::var(WMS_DB_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
                )
            }),
    }
}

fn validate_staging_database_url(database_url: &str) -> Result<(), io::Error> {
    let trimmed = database_url.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL is required",
        ));
    }

    let lowered = trimmed.to_ascii_lowercase();
    if !lowered.starts_with("postgres://") && !lowered.starts_with("postgresql://") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL must use postgres:// or postgresql://",
        ));
    }

    let authority_and_path = lowered
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(lowered.as_str());
    let host_port_path = authority_and_path
        .rsplit_once('@')
        .map(|(_, rest)| rest)
        .unwrap_or(authority_and_path);
    let host = host_port_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    if host.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL host is required",
        ));
    }

    if host.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL must use a staging DNS name, not a raw IP",
        ));
    }

    if BLOCKED_DB_URL_TOKENS
        .iter()
        .any(|token| host_port_path.contains(token))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL contains blocked boundary token",
        ));
    }

    if !has_environment_token(host_port_path, "staging") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "database URL must contain staging boundary token",
        ));
    }

    Ok(())
}

async fn ensure_current_audit_partition(pool: &PgPool) -> Result<(), io::Error> {
    sqlx::query("SELECT create_current_partition(CURRENT_DATE)")
        .execute(pool)
        .await
        .map_err(|error| {
            io::Error::other(format!(
                "failed to ensure current audit partition: {error:?}"
            ))
        })?;
    Ok(())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<BTreeMap<String, String>, io::Error> {
    let mut parsed = BTreeMap::new();
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        if matches!(flag.as_str(), "-h" | "--help") {
            parsed.insert("help".to_string(), "true".to_string());
            continue;
        }
        if !flag.starts_with("--") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unexpected argument {flag}; expected --name value"),
            ));
        }
        let key = flag.trim_start_matches("--").to_string();
        if matches!(key.as_str(), "check-only" | "from-env") {
            parsed.insert(key, "true".to_string());
            continue;
        }
        let value = args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing value for --{key}"),
            )
        })?;
        if value.starts_with("--") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing value for --{key}"),
            ));
        }
        parsed.insert(key, value);
    }
    Ok(parsed)
}

fn print_usage() {
    println!("Usage: wms-deploy-audit --from-env --check-only");
    println!(
        "Or: wms-deploy-audit --check-only --environment staging --deployment-mode docker-compose --module W6.H --action deploy.gray_release.recorded --resource-type deployment_release --resource-id <release-id> --release-version <version> --release-plan-ref <ref> --artifact-ref <ref> --canary-config-ref <ref> --smoke-gate-ref <ref> --observability-dashboard-ref <ref> --rollback-drill-log-ref <ref> --approval-record-ref <ref> --canary-stages-exercised <n> --smoke-checks-passed <n> --rollback-drills-exercised <n> --actor-id <uuid> --actor-name <name> --owner-id <uuid> --jti <deploy-run-id>"
    );
    println!("Writes audit_event only when --check-only is omitted and DATABASE_URL/WMS_DB_URL points to a real staging PostgreSQL DNS.");
}

fn apply_from_env<F>(args: &mut BTreeMap<String, String>, lookup: F) -> Vec<&'static str>
where
    F: Fn(&str) -> Option<String>,
{
    let mut missing = Vec::new();
    for (env_name, arg_name) in FROM_ENV_ARG_MAP {
        match lookup(env_name).filter(|value| !value.trim().is_empty()) {
            Some(value) => {
                args.insert(arg_name.to_string(), value);
            }
            None => missing.push(env_name),
        }
    }
    missing
}

fn bool_flag(args: &BTreeMap<String, String>, key: &str) -> bool {
    args.get(key).map(|value| value == "true").unwrap_or(false)
}

fn required(args: &BTreeMap<String, String>, key: &str) -> Result<String, io::Error> {
    args.get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("--{key} is required")))
}

fn optional(args: &BTreeMap<String, String>, key: &str) -> Option<String> {
    args.get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

fn positive_i64(args: &BTreeMap<String, String>, key: &str) -> Result<i64, io::Error> {
    let raw = required(args, key)?;
    let value = raw.parse::<i64>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("--{key} must be an integer: {error}"),
        )
    })?;
    if value < 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("--{key} must be >= 1"),
        ));
    }
    Ok(value)
}

fn uuid_arg(args: &BTreeMap<String, String>, key: &str) -> Result<Uuid, io::Error> {
    let raw = required(args, key)?;
    Uuid::parse_str(&raw).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("--{key} must be a UUID: {error}"),
        )
    })
}

fn optional_uuid_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<Uuid>, io::Error> {
    optional(args, key)
        .map(|raw| {
            Uuid::parse_str(&raw).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("--{key} must be a UUID: {error}"),
                )
            })
        })
        .transpose()
}

fn audit_event_query_ref(environment: &str, event: &wms_api::audit::AuditEventRecord) -> String {
    format!(
        "postgres://wms-{}/audit_event/{}/{}/{}",
        ref_component(environment),
        ref_component(&event.module),
        ref_component(&event.resource_id),
        event.id,
    )
}

fn ref_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn preview_payload(environment: &str, module: &str, action: &str) -> serde_json::Value {
    json!({
        "ok": true,
        "check_only": true,
        "writes_audit_event": false,
        "writes_runtime_evidence": false,
        "closes_gate": false,
        "environment": environment,
        "module": module,
        "action": action,
        "next": "remove --check-only only after real staging refs, H1 actor/owner, approval and release window are confirmed",
    })
}

fn has_environment_token(value: &str, environment: &str) -> bool {
    let bytes = value.as_bytes();
    let needle = environment.as_bytes();
    bytes
        .windows(needle.len())
        .enumerate()
        .any(|(index, window)| {
            if window != needle {
                return false;
            }
            let before = index
                .checked_sub(1)
                .and_then(|previous| bytes.get(previous))
                .copied();
            let after = bytes.get(index + needle.len()).copied();
            !is_ascii_alnum(before) && !is_ascii_alnum(after)
        })
}

fn is_ascii_alnum(value: Option<u8>) -> bool {
    value
        .map(|byte| byte.is_ascii_alphanumeric())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use wms_api::audit::{AuditEventRecord, AuditWriteRequest};

    use super::{
        apply_from_env, audit_event_query_ref, bool_flag, parse_args, positive_i64,
        preview_payload, required, validate_staging_database_url,
    };

    #[test]
    fn parse_args_rejects_missing_values() {
        let err = parse_args(["--module".to_string()].into_iter())
            .expect_err("missing value should fail");

        assert!(err.to_string().contains("missing value for --module"));
    }

    #[test]
    fn parse_args_accepts_help_without_value() {
        let args = parse_args(["--help".to_string()].into_iter()).expect("help should parse");

        assert!(bool_flag(&args, "help"));
    }

    #[test]
    fn required_and_positive_values_are_validated() {
        let args = parse_args(
            [
                "--module".to_string(),
                "W6.H".to_string(),
                "--smoke-checks-passed".to_string(),
                "1".to_string(),
            ]
            .into_iter(),
        )
        .expect("args should parse");

        assert_eq!(required(&args, "module").expect("module required"), "W6.H");
        assert_eq!(
            positive_i64(&args, "smoke-checks-passed").expect("positive count"),
            1,
        );
        assert!(required(&args, "action").is_err());
    }

    #[test]
    fn bool_flag_detects_check_only_without_value() {
        let args = parse_args(
            [
                "--module".to_string(),
                "W6.H".to_string(),
                "--check-only".to_string(),
            ]
            .into_iter(),
        )
        .expect("args should parse");

        assert!(bool_flag(&args, "check-only"));
        assert_eq!(required(&args, "module").expect("module required"), "W6.H");
    }

    #[test]
    fn parse_args_accepts_from_env_without_value() {
        let args = parse_args(["--from-env".to_string(), "--check-only".to_string()].into_iter())
            .expect("from-env should parse as a flag");

        assert!(bool_flag(&args, "from-env"));
        assert!(bool_flag(&args, "check-only"));
    }

    #[test]
    fn apply_from_env_fills_wave6_deploy_audit_args() {
        let mut args =
            parse_args(["--from-env".to_string()].into_iter()).expect("from-env should parse");
        let missing = apply_from_env(&mut args, |name| {
            match name {
                "WAVE_6_ENVIRONMENT" => Some("staging"),
                "WAVE_6_DEPLOYMENT_MODE" => Some("docker-compose"),
                "WAVE_6_DEPLOY_MODULE" => Some("W6.H"),
                "WAVE_6_DEPLOY_ACTION" => Some("deploy.gray_release.recorded"),
                "WAVE_6_DEPLOY_RESOURCE_TYPE" => Some("deployment_release"),
                "WAVE_6_DEPLOY_RESOURCE_ID" => Some("W6H:staging:wms-api-20260608.1"),
                "WAVE_6_RELEASE_VERSION" => Some("wms-api-20260608.1"),
                "WAVE_6_RELEASE_PLAN_REF" => Some("ticket://wms-staging-release-plan/W6H"),
                "WAVE_6_ARTIFACT_REF" => Some("registry://wms-staging/wms-api@sha256:abc"),
                "WAVE_6_CANARY_CONFIG_REF" => Some("git/staging/wave6-canary"),
                "WAVE_6_SMOKE_GATE_REF" => Some("ci/staging/wave6-smoke"),
                "WAVE_6_OBSERVABILITY_DASHBOARD_REF" => Some("grafana/staging/wave6"),
                "WAVE_6_ROLLBACK_DRILL_LOG_REF" => Some("ci/staging/wave6-rollback"),
                "WAVE_6_APPROVAL_RECORD_REF" => Some("ticket://wms-staging-approval/W6H"),
                "WAVE_6_CANARY_STAGES_EXERCISED" => Some("1"),
                "WAVE_6_SMOKE_CHECKS_PASSED" => Some("1"),
                "WAVE_6_ROLLBACK_DRILLS_EXERCISED" => Some("1"),
                "WAVE_6_DEPLOY_ACTOR_ID" => Some("11111111-1111-4111-8111-111111111111"),
                "WAVE_6_DEPLOY_ACTOR_NAME" => Some("release-operator"),
                "WAVE_6_DEPLOY_OWNER_ID" => Some("22222222-2222-4222-8222-222222222222"),
                "WAVE_6_DEPLOY_JTI" => Some("deploy-staging-W6H-20260608"),
                _ => None,
            }
            .map(str::to_string)
        });

        assert!(missing.is_empty());
        assert_eq!(
            required(&args, "environment").expect("environment"),
            "staging"
        );
        assert_eq!(required(&args, "module").expect("module"), "W6.H");
        assert_eq!(
            required(&args, "actor-id").expect("actor id"),
            "11111111-1111-4111-8111-111111111111",
        );
        assert_eq!(
            positive_i64(&args, "rollback-drills-exercised").expect("count"),
            1,
        );
    }

    #[test]
    fn apply_from_env_reports_missing_wave6_deploy_audit_vars() {
        let mut args =
            parse_args(["--from-env".to_string()].into_iter()).expect("from-env should parse");
        let missing = apply_from_env(&mut args, |name| {
            (name == "WAVE_6_ENVIRONMENT").then_some("staging".to_string())
        });

        assert!(missing.contains(&"WAVE_6_RELEASE_PLAN_REF"));
        assert!(missing.contains(&"WAVE_6_DEPLOY_ACTOR_ID"));
        assert!(missing.contains(&"WAVE_6_DEPLOY_JTI"));
        assert_eq!(
            required(&args, "environment").expect("environment"),
            "staging"
        );
        assert!(required(&args, "release-plan-ref").is_err());
    }

    #[test]
    fn preview_payload_reports_no_writes() {
        let payload = preview_payload("staging", "W6.H", "deploy.gray_release.recorded");

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["check_only"], true);
        assert_eq!(payload["writes_audit_event"], false);
        assert_eq!(payload["writes_runtime_evidence"], false);
        assert_eq!(payload["closes_gate"], false);
        assert_eq!(payload["environment"], "staging");
        assert_eq!(payload["module"], "W6.H");
        assert_eq!(payload["action"], "deploy.gray_release.recorded");
    }

    #[test]
    fn audit_event_query_ref_contains_environment_and_no_secret() {
        let req = AuditWriteRequest {
            occurred_at: Utc::now(),
            actor_id: Uuid::new_v4(),
            actor_name: "release-bot".to_string(),
            owner_id: Uuid::new_v4(),
            jti: "deploy:staging:wms-api-1:job-1".to_string(),
            action: "deploy.gray_release.recorded".to_string(),
            module: "W6.H".to_string(),
            resource_type: "deployment_release".to_string(),
            resource_id: "W6H:staging:wms-api-20260608.1".to_string(),
            diff: None,
            request_id: None,
            ip: None,
            user_agent: None,
        };
        let event = AuditEventRecord {
            id: 42,
            occurred_at: req.occurred_at,
            actor_id: req.actor_id,
            actor_name: req.actor_name,
            owner_id: req.owner_id,
            jti: req.jti,
            action: req.action,
            module: req.module,
            resource_type: req.resource_type,
            resource_id: req.resource_id,
            diff: req.diff,
            request_id: req.request_id,
            ip: req.ip,
            user_agent: req.user_agent,
            prev_hash: None,
            self_hash: "hash".to_string(),
        };

        assert_eq!(
            audit_event_query_ref("staging", &event),
            "postgres://wms-staging/audit_event/W6.H/W6H:staging:wms-api-20260608.1/42",
        );
    }

    #[test]
    fn validate_staging_database_url_accepts_staging_dns() {
        validate_staging_database_url(
            "postgres://wms_user:secret@pg-staging.wms.internal:5432/wms_staging",
        )
        .expect("staging DNS should be accepted");
        validate_staging_database_url("postgresql://wms_user:secret@db.wms.internal/wms-staging")
            .expect("staging db name should be accepted");
    }

    #[test]
    fn validate_staging_database_url_rejects_local_raw_ip_dev_prod_and_example() {
        for database_url in [
            "postgres://wms:secret@localhost:5432/wms_staging",
            "postgres://wms:secret@127.0.0.1:5432/wms_staging",
            "postgres://wms:secret@192.168.1.10:5432/wms_staging",
            "postgres://wms:secret@pg-dev.wms.internal:5432/wms_dev",
            "postgres://wms:secret@pg-prod.wms.internal:5432/wms_prod",
            "postgres://wms:secret@pg-staging.example:5432/wms_staging",
            "postgres://wms:secret@pgstaging.wms.internal:5432/wms",
        ] {
            let err = validate_staging_database_url(database_url)
                .expect_err("non-staging DB boundary must fail");

            assert!(
                err.to_string().contains("database URL"),
                "unexpected error for {database_url}: {err}",
            );
        }
    }
}
