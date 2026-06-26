use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::audit::{AuditDiff, AuditWriteRequest};

const BLOCKED_REF_TOKENS: [&str; 11] = [
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
const PLACEHOLDER_TOKENS: [&str; 7] = ["yyyy", "<", ">", "todo", "tbd", "待填", "待确认"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployAuditInput {
    pub module: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub environment: String,
    pub deployment_mode: String,
    pub release_version: String,
    pub release_plan_ref: String,
    pub artifact_ref: String,
    pub canary_config_ref: String,
    pub smoke_gate_ref: String,
    pub observability_dashboard_ref: String,
    pub rollback_drill_log_ref: String,
    pub approval_record_ref: String,
    pub canary_stages_exercised: i64,
    pub smoke_checks_passed: i64,
    pub rollback_drills_exercised: i64,
}

pub fn validate_deploy_audit_input(input: &DeployAuditInput) -> Result<(), String> {
    if input.environment.trim().to_ascii_lowercase() != "staging" {
        return Err(
            "environment must be staging; W6.H deploy audit cannot be written for dev/local/prod"
                .to_string(),
        );
    }

    if !matches!(
        input.deployment_mode.as_str(),
        "docker-compose" | "kubernetes"
    ) {
        return Err("deployment_mode must be docker-compose or kubernetes".to_string());
    }

    if input.release_version.trim().is_empty() {
        return Err("release_version is required".to_string());
    }

    let refs = [
        ("release_plan_ref", input.release_plan_ref.as_str()),
        ("artifact_ref", input.artifact_ref.as_str()),
        ("canary_config_ref", input.canary_config_ref.as_str()),
        ("smoke_gate_ref", input.smoke_gate_ref.as_str()),
        (
            "observability_dashboard_ref",
            input.observability_dashboard_ref.as_str(),
        ),
        (
            "rollback_drill_log_ref",
            input.rollback_drill_log_ref.as_str(),
        ),
        ("approval_record_ref", input.approval_record_ref.as_str()),
    ];
    for (name, value) in refs {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(format!("{name} is required"));
        }
        let lowered = trimmed.to_ascii_lowercase();
        if PLACEHOLDER_TOKENS
            .iter()
            .any(|token| lowered.contains(token))
        {
            return Err(format!("{name} contains placeholder token"));
        }
        if BLOCKED_REF_TOKENS
            .iter()
            .any(|token| lowered.contains(token))
        {
            return Err(format!("{name} contains blocked boundary token"));
        }
        if !has_environment_token(trimmed, "staging") {
            return Err(format!("{name} evidence refs must contain staging"));
        }
    }

    if input.canary_stages_exercised < 1
        || input.smoke_checks_passed < 1
        || input.rollback_drills_exercised < 1
    {
        return Err(
            "canary_stages_exercised, smoke_checks_passed and rollback_drills_exercised counts must be >= 1"
                .to_string(),
        );
    }

    Ok(())
}

pub fn build_deploy_audit_request(
    input: DeployAuditInput,
    actor_id: Uuid,
    actor_name: String,
    owner_id: Uuid,
    jti: String,
    occurred_at: DateTime<Utc>,
) -> AuditWriteRequest {
    let after = json!({
        "environment": input.environment,
        "deployment_mode": input.deployment_mode,
        "release_version": input.release_version,
        "release_plan_ref": input.release_plan_ref,
        "artifact_ref": input.artifact_ref,
        "canary_config_ref": input.canary_config_ref,
        "smoke_gate_ref": input.smoke_gate_ref,
        "observability_dashboard_ref": input.observability_dashboard_ref,
        "rollback_drill_log_ref": input.rollback_drill_log_ref,
        "approval_record_ref": input.approval_record_ref,
        "canary_stages_exercised": input.canary_stages_exercised,
        "smoke_checks_passed": input.smoke_checks_passed,
        "rollback_drills_exercised": input.rollback_drills_exercised,
    });
    let module = input.module;
    let action = input.action;
    let resource_type = input.resource_type;
    let resource_id = input.resource_id;
    let diff = AuditDiff {
        before: serde_json::Value::Null,
        after,
        changed_keys: vec![
            "approval_record_ref".to_string(),
            "artifact_ref".to_string(),
            "canary_config_ref".to_string(),
            "canary_stages_exercised".to_string(),
            "deployment_mode".to_string(),
            "environment".to_string(),
            "observability_dashboard_ref".to_string(),
            "release_plan_ref".to_string(),
            "release_version".to_string(),
            "rollback_drill_log_ref".to_string(),
            "rollback_drills_exercised".to_string(),
            "smoke_checks_passed".to_string(),
            "smoke_gate_ref".to_string(),
        ],
    };

    AuditWriteRequest {
        occurred_at,
        actor_id,
        actor_name,
        owner_id,
        jti,
        action,
        module,
        resource_type,
        resource_id,
        diff: Some(diff),
        request_id: None,
        ip: None,
        user_agent: Some("wms-deploy-audit".to_string()),
    }
}

fn has_environment_token(value: &str, environment: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
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

    use super::{build_deploy_audit_request, validate_deploy_audit_input, DeployAuditInput};

    #[test]
    fn build_deploy_audit_request_maps_wave6_release_to_audit_event() {
        let input = DeployAuditInput {
            module: "W6.H".to_string(),
            action: "deploy.gray_release.recorded".to_string(),
            resource_type: "deployment_release".to_string(),
            resource_id: "W6H:staging:wms-api-20260608.1".to_string(),
            environment: "staging".to_string(),
            deployment_mode: "docker-compose".to_string(),
            release_version: "wms-api-20260608.1".to_string(),
            release_plan_ref: "ticket://staging-release-plan/W6H-20260608".to_string(),
            artifact_ref: "registry://wms-staging/wms-api@sha256:abcdef".to_string(),
            canary_config_ref: "git/staging/wave6-canary-config/123".to_string(),
            smoke_gate_ref: "ci/staging/wave6-smoke-gate/123".to_string(),
            observability_dashboard_ref: "grafana/staging/wave6-release/123".to_string(),
            rollback_drill_log_ref: "ci/staging/wave6-rollback-drill/123".to_string(),
            approval_record_ref: "ticket://staging-release-approval/W6H-20260608".to_string(),
            canary_stages_exercised: 1,
            smoke_checks_passed: 1,
            rollback_drills_exercised: 1,
        };
        let actor_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let occurred_at = Utc::now();

        let req = build_deploy_audit_request(
            input,
            actor_id,
            "release-operator".to_string(),
            owner_id,
            "deploy-jti-1".to_string(),
            occurred_at,
        );

        assert_eq!(req.module, "W6.H");
        assert_eq!(req.action, "deploy.gray_release.recorded");
        assert_eq!(req.resource_type, "deployment_release");
        assert_eq!(req.resource_id, "W6H:staging:wms-api-20260608.1");
        assert_eq!(req.actor_id, actor_id);
        assert_eq!(req.owner_id, owner_id);
        assert_eq!(req.jti, "deploy-jti-1");

        let diff = req.diff.expect("deploy audit should include release refs");
        assert_eq!(diff.before, serde_json::Value::Null);
        assert_eq!(
            diff.changed_keys,
            vec![
                "approval_record_ref",
                "artifact_ref",
                "canary_config_ref",
                "canary_stages_exercised",
                "deployment_mode",
                "environment",
                "observability_dashboard_ref",
                "release_plan_ref",
                "release_version",
                "rollback_drill_log_ref",
                "rollback_drills_exercised",
                "smoke_checks_passed",
                "smoke_gate_ref",
            ],
        );
        assert_eq!(diff.after["environment"], "staging");
        assert_eq!(
            diff.after["artifact_ref"],
            "registry://wms-staging/wms-api@sha256:abcdef"
        );
    }

    #[test]
    fn validate_deploy_audit_input_rejects_non_staging_environment() {
        let mut input = valid_input();
        input.environment = "dev".to_string();
        input.release_plan_ref = "ticket://wms-dev-release-plan/W6H-20260608".to_string();

        let err = validate_deploy_audit_input(&input).expect_err("dev must not close W6.H");

        assert!(err.contains("environment must be staging"));
    }

    #[test]
    fn validate_deploy_audit_input_rejects_blocked_or_mismatched_refs() {
        let mut input = valid_input();
        input.artifact_ref = "registry://wms-release/wms-api@sha256:abcdef".to_string();

        let err = validate_deploy_audit_input(&input).expect_err("refs without staging must fail");

        assert!(err.contains("evidence refs must contain staging"));

        input = valid_input();
        input.smoke_gate_ref = "ci/dev/wave6-smoke/123".to_string();

        let err = validate_deploy_audit_input(&input).expect_err("dev refs must fail");

        assert!(err.contains("blocked boundary"));

        input = valid_input();
        input.release_plan_ref = "s3://wms-staging-example/wave6/release-plan.md".to_string();

        let err = validate_deploy_audit_input(&input).expect_err("example refs must fail");

        assert!(err.contains("blocked boundary"));
    }

    #[test]
    fn validate_deploy_audit_input_rejects_invalid_deployment_mode_and_counts() {
        let mut input = valid_input();
        input.deployment_mode = "manual".to_string();

        let err = validate_deploy_audit_input(&input).expect_err("mode must fail");

        assert!(err.contains("deployment_mode"));

        input = valid_input();
        input.canary_stages_exercised = 0;

        let err = validate_deploy_audit_input(&input).expect_err("count must fail");

        assert!(err.contains("counts must be >= 1"));
    }

    fn valid_input() -> DeployAuditInput {
        DeployAuditInput {
            module: "W6.H".to_string(),
            action: "deploy.gray_release.recorded".to_string(),
            resource_type: "deployment_release".to_string(),
            resource_id: "W6H:staging:wms-api-20260608.1".to_string(),
            environment: "staging".to_string(),
            deployment_mode: "docker-compose".to_string(),
            release_version: "wms-api-20260608.1".to_string(),
            release_plan_ref: "ticket://wms-staging-release-plan/W6H-20260608".to_string(),
            artifact_ref: "registry://wms-staging/wms-api@sha256:abcdef".to_string(),
            canary_config_ref: "git/staging/wave6-canary-config/123".to_string(),
            smoke_gate_ref: "ci/staging/wave6-smoke-gate/123".to_string(),
            observability_dashboard_ref: "grafana/staging/wave6-release/123".to_string(),
            rollback_drill_log_ref: "ci/staging/wave6-rollback-drill/123".to_string(),
            approval_record_ref: "ticket://staging-release-approval/W6H-20260608".to_string(),
            canary_stages_exercised: 1,
            smoke_checks_passed: 1,
            rollback_drills_exercised: 1,
        }
    }
}
