//! Wave 4 M-TC traceability-code reporting boundary.
//!
//! The external "码上放心" adapter is deferred until the platform's formal
//! field names, auth scheme, error codes, and rate limits are confirmed.

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    TraceabilityOutboundReport, TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
};

const BLOCKED_REF_TOKENS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "example",
];
const TRACEABILITY_PLATFORM: &str = "码上放心";
const STATUS_QUEUED: &str = "queued";
const STATUS_REPORTED: &str = "reported";
const STATUS_PENDING_REPLAY: &str = "pending_replay";

#[derive(Clone, Debug, Default)]
pub struct TraceabilityCodeService;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceabilityCodeError {
    EmptyEvents,
    InvalidEvent,
    MissingPlatformEvidenceRef,
    UnsafePlatformEvidenceRef,
    InlineCredential,
    MissingTraceId,
    MissingReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceabilityPlatformConfig {
    pub environment: String,
    pub api_doc_ref: String,
    pub auth_doc_ref: String,
    pub error_code_doc_ref: String,
    pub rate_limit_doc_ref: String,
    pub credential_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceabilityPlatformResponse {
    pub success: bool,
    pub platform_receipt_id: Option<String>,
    pub error_code: Option<String>,
    pub retryable: bool,
    pub trace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceabilityReplayDecision {
    pub status: String,
    pub should_retry: bool,
    pub audit_action: String,
    pub trace_id: String,
    pub platform_receipt_id: Option<String>,
    pub error_code: Option<String>,
}

impl TraceabilityCodeService {
    pub fn traceability_report(
        &self,
        req: TraceabilityOutboundReportRequest,
    ) -> Result<TraceabilityOutboundReport, TraceabilityCodeError> {
        self.traceability_report_at(req, Utc::now())
    }

    pub fn traceability_report_at(
        &self,
        req: TraceabilityOutboundReportRequest,
        now: DateTime<Utc>,
    ) -> Result<TraceabilityOutboundReport, TraceabilityCodeError> {
        if req.events.is_empty() {
            return Err(TraceabilityCodeError::EmptyEvents);
        }
        if req.events.iter().any(invalid_event) {
            return Err(TraceabilityCodeError::InvalidEvent);
        }

        Ok(TraceabilityOutboundReport {
            report_id: Uuid::new_v4(),
            platform: TRACEABILITY_PLATFORM.to_string(),
            status: STATUS_QUEUED.to_string(),
            queued_count: req.events.len() as u32,
            generated_at: now,
            events: req.events,
        })
    }

    pub fn validate_platform_config(
        &self,
        config: &TraceabilityPlatformConfig,
    ) -> Result<(), TraceabilityCodeError> {
        let refs = [
            config.environment.as_str(),
            config.api_doc_ref.as_str(),
            config.auth_doc_ref.as_str(),
            config.error_code_doc_ref.as_str(),
            config.rate_limit_doc_ref.as_str(),
            config.credential_ref.as_str(),
        ];
        if refs.iter().any(|value| value.trim().is_empty()) {
            return Err(TraceabilityCodeError::MissingPlatformEvidenceRef);
        }
        if !matches!(config.environment.as_str(), "dev" | "staging") {
            return Err(TraceabilityCodeError::UnsafePlatformEvidenceRef);
        }
        if refs.iter().any(|value| has_blocked_ref_token(value)) {
            return Err(TraceabilityCodeError::UnsafePlatformEvidenceRef);
        }
        if !config.credential_ref.starts_with("vault://") {
            return Err(TraceabilityCodeError::InlineCredential);
        }
        Ok(())
    }

    pub fn classify_platform_response(
        &self,
        response: TraceabilityPlatformResponse,
    ) -> Result<TraceabilityReplayDecision, TraceabilityCodeError> {
        if response.trace_id.trim().is_empty() {
            return Err(TraceabilityCodeError::MissingTraceId);
        }
        if response.success {
            let receipt = response
                .platform_receipt_id
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .ok_or(TraceabilityCodeError::MissingReceipt)?;
            return Ok(TraceabilityReplayDecision {
                status: STATUS_REPORTED.to_string(),
                should_retry: false,
                audit_action: "traceability.report.success".to_string(),
                trace_id: response.trace_id,
                platform_receipt_id: Some(receipt.to_string()),
                error_code: response.error_code,
            });
        }

        Ok(TraceabilityReplayDecision {
            status: STATUS_PENDING_REPLAY.to_string(),
            should_retry: response.retryable,
            audit_action: if response.retryable {
                "traceability.report.retry_scheduled".to_string()
            } else {
                "traceability.report.pending_replay".to_string()
            },
            trace_id: response.trace_id,
            platform_receipt_id: response.platform_receipt_id,
            error_code: response.error_code,
        })
    }
}

fn invalid_event(event: &TraceabilityStatusChangeEvent) -> bool {
    event.trace_code.trim().is_empty() || event.status_change_type.trim().is_empty()
}

fn has_blocked_ref_token(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    BLOCKED_REF_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent};

    use super::{
        TraceabilityCodeError, TraceabilityCodeService, TraceabilityPlatformConfig,
        TraceabilityPlatformResponse,
    };

    fn outbound_event() -> TraceabilityStatusChangeEvent {
        TraceabilityStatusChangeEvent {
            event_id: Uuid::new_v4(),
            trace_code: "TC-202606-0001".to_string(),
            status_change_type: "已入库→已出库".to_string(),
            occurred_at: Utc
                .with_ymd_and_hms(2026, 6, 5, 14, 0, 0)
                .single()
                .expect("valid time"),
        }
    }

    #[test]
    fn traceability_report_queues_confirmed_status_change_tuple_only() {
        let service = TraceabilityCodeService;
        let report = service
            .traceability_report(TraceabilityOutboundReportRequest {
                events: vec![outbound_event()],
            })
            .expect("valid traceability event should be queued");

        assert_eq!(report.platform, "码上放心");
        assert_eq!(report.status, "queued");
        assert_eq!(report.queued_count, 1);
        assert_eq!(report.events[0].status_change_type, "已入库→已出库");
    }

    #[test]
    fn traceability_report_rejects_empty_event_or_missing_tuple_field() {
        let service = TraceabilityCodeService;
        assert!(matches!(
            service.traceability_report(TraceabilityOutboundReportRequest { events: vec![] }),
            Err(TraceabilityCodeError::EmptyEvents)
        ));

        let mut event = outbound_event();
        event.status_change_type = String::new();
        assert!(matches!(
            service.traceability_report(TraceabilityOutboundReportRequest {
                events: vec![event]
            }),
            Err(TraceabilityCodeError::InvalidEvent)
        ));
    }

    #[test]
    fn platform_config_requires_real_dev_or_staging_refs_and_vault_credential() {
        let service = TraceabilityCodeService;
        let config = TraceabilityPlatformConfig {
            environment: "staging".to_string(),
            api_doc_ref: "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf".to_string(),
            auth_doc_ref: "s3://wms-staging-evidence/wave4/traceability/auth.md".to_string(),
            error_code_doc_ref: "s3://wms-staging-evidence/wave4/traceability/error-codes.md"
                .to_string(),
            rate_limit_doc_ref: "s3://wms-staging-evidence/wave4/traceability/rate-limit.md"
                .to_string(),
            credential_ref: "vault://wms/staging/traceability/masxf".to_string(),
        };

        assert_eq!(service.validate_platform_config(&config), Ok(()));

        let mut unsafe_config = config.clone();
        unsafe_config.api_doc_ref =
            "s3://wms-prod-evidence/wave4/traceability/api-doc.pdf".to_string();
        assert!(matches!(
            service.validate_platform_config(&unsafe_config),
            Err(TraceabilityCodeError::UnsafePlatformEvidenceRef)
        ));

        let mut inline_secret_config = config;
        inline_secret_config.credential_ref = "inline-token".to_string();
        assert!(matches!(
            service.validate_platform_config(&inline_secret_config),
            Err(TraceabilityCodeError::InlineCredential)
        ));
    }

    #[test]
    fn platform_response_success_requires_receipt_and_failure_goes_to_replay() {
        let service = TraceabilityCodeService;
        let success = service
            .classify_platform_response(TraceabilityPlatformResponse {
                success: true,
                platform_receipt_id: Some("MASXF-RCPT-001".to_string()),
                error_code: None,
                retryable: false,
                trace_id: "trace-success-001".to_string(),
            })
            .expect("success with receipt should classify");

        assert_eq!(success.status, "reported");
        assert!(!success.should_retry);
        assert_eq!(success.audit_action, "traceability.report.success");

        assert!(matches!(
            service.classify_platform_response(TraceabilityPlatformResponse {
                success: true,
                platform_receipt_id: None,
                error_code: None,
                retryable: false,
                trace_id: "trace-missing-receipt".to_string(),
            }),
            Err(TraceabilityCodeError::MissingReceipt)
        ));

        let retry = service
            .classify_platform_response(TraceabilityPlatformResponse {
                success: false,
                platform_receipt_id: None,
                error_code: Some("RATE_LIMITED".to_string()),
                retryable: true,
                trace_id: "trace-retry-001".to_string(),
            })
            .expect("retryable failure should classify");

        assert_eq!(retry.status, "pending_replay");
        assert!(retry.should_retry);
        assert_eq!(retry.audit_action, "traceability.report.retry_scheduled");
        assert_eq!(retry.error_code.as_deref(), Some("RATE_LIMITED"));
    }
}
