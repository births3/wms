"""Shared fixtures for Wave 4/5/6 runtime evidence validator tests."""
import json
from pathlib import Path


def valid_wave4_external_evidence() -> dict[str, object]:
    return {
        "environment": "staging",
        "platform": "码上放心",
        "api_doc_ref": "s3://wms-staging-evidence/wave4/traceability/api-doc.pdf",
        "auth_doc_ref": "s3://wms-staging-evidence/wave4/traceability/auth.md",
        "error_code_doc_ref": "s3://wms-staging-evidence/wave4/traceability/error-codes.md",
        "rate_limit_doc_ref": "s3://wms-staging-evidence/wave4/traceability/rate-limit.md",
        "credential_ref": "vault://wms/staging/traceability/masxf",
        "success_report_log_ref": "ci/staging/wave4-traceability-success/123",
        "failure_retry_log_ref": "ci/staging/wave4-traceability-retry/123",
        "audit_event_query_ref": "ci/staging/wave4-traceability-audit/123",
        "reported_events": 1,
        "failed_events_exercised": 1,
        "pending_replay_queue_verified": True,
    }


def valid_wave5_hardware_evidence() -> dict[str, object]:
    return {
        "environment": "staging",
        "station_code": "PK-STAGING-01",
        "scale_device_ref": "asset://wms-staging/hardware/scale-01",
        "bluetooth_printer_ref": "asset://wms-staging/hardware/bluetooth-printer-01",
        "waybill_printer_ref": "asset://wms-staging/hardware/waybill-printer-01",
        "calibration_record_ref": "s3://wms-staging-evidence/wave5/hardware/calibration.pdf",
        "scale_reading_log_ref": "ci/staging/wave5-hardware-scale/123",
        "bluetooth_print_log_ref": "ci/staging/wave5-hardware-bluetooth-print/123",
        "waybill_print_log_ref": "ci/staging/wave5-hardware-waybill-print/123",
        "audit_event_query_ref": "ci/staging/wave5-hardware-audit/123",
        "scale_readings_recorded": 1,
        "bluetooth_labels_printed": 1,
        "waybills_printed": 1,
        "hardware_connected": True,
        "print_artifacts_reviewed": True,
        "audit_event_verified": True,
    }


def valid_wave5_tms_evidence() -> dict[str, object]:
    return {
        "environment": "staging",
        "tms_system_ref": "partner://wms-staging/tms/vendor-a",
        "dispatch_push_log_ref": "ci/staging/wave5-tms-dispatch-push/123",
        "callback_log_ref": "ci/staging/wave5-tms-callback/123",
        "failure_retry_log_ref": "ci/staging/wave5-tms-failure-retry/123",
        "audit_event_query_ref": "ci/staging/wave5-tms-audit/123",
        "credential_ref": "vault://wms/staging/tms/vendor-a",
        "dispatches_received": 1,
        "callbacks_received": 1,
        "failed_callbacks_exercised": 1,
        "retry_succeeded": True,
        "audit_event_verified": True,
    }


def valid_wave6_deploy_evidence() -> dict[str, object]:
    return {
        "environment": "staging",
        "deployment_mode": "kubernetes",
        "release_version": "wms-api-20260604.1",
        "release_plan_ref": "s3://wms-staging-evidence/wave6/deploy/release-plan.md",
        "artifact_ref": "registry://wms-staging/api@sha256:abcdef",
        "canary_config_ref": "gitlab/staging/wave6-canary-config/123",
        "smoke_gate_ref": "ci/staging/wave6-smoke-gate/123",
        "observability_dashboard_ref": "grafana/staging/wave6-release/123",
        "rollback_drill_log_ref": "ci/staging/wave6-rollback-drill/123",
        "approval_record_ref": "ticket://staging-release-approval/WMS-20260604",
        "audit_event_query_ref": "ci/staging/wave6-deploy-audit/123",
        "canary_stages_exercised": 1,
        "smoke_checks_passed": 1,
        "rollback_drills_exercised": 1,
        "canary_used": True,
        "full_release_blocked": True,
        "rollback_verified": True,
        "audit_event_verified": True,
        "dual_approval_recorded": True,
    }


def write_evidence(path: Path, evidence: dict[str, object]) -> None:
    path.write_text(json.dumps(evidence), encoding="utf-8")
