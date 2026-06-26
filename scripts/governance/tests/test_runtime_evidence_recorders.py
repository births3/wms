"""Wave runtime evidence record 脚本输出、校验与覆盖保护测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_record_wave_runtime_evidence_requires_force_to_overwrite_existing_files(tmp_path):
    """所有 Wave 6 收口 record 脚本都不能静默覆盖已有真实 evidence。"""
    import importlib

    cases = [
        (
            "record_wave2_runtime_evidence",
            "wave-2-runtime-evidence.json",
            [
                "--environment", "staging",
                "--service-url", "https://wms-staging.internal",
                "--migrated-count", "1",
                "--reconcile-matched", "1",
                "--smoke-log-ref", "ci/staging/wave2-feature-flags-smoke/123",
                "--reconcile-log-ref", "ci/staging/wave2-feature-flags-reconcile/123",
                "--archive-ref", "s3://wms-staging-audit/feature-flags/feature_flags.toml",
            ],
        ),
        (
            "record_wave3_pda_runtime_evidence",
            "wave-3-pda-runtime-evidence.json",
            [
                "--environment", "staging",
                "--pda-model", "Honeywell EDA52",
                "--android-version", "Android 11",
                "--scan-input-method", "physical-scan-key-intent",
                "--pda-stack-candidate", "react-native",
                "--pda-device-ref", "asset://wms-staging/pda/honeywell-eda52-01",
                "--spike005-result-ref", "s3://wms-staging-evidence/wave3/pda/spike-005-runtime-20260604.md",
                "--m2-scan-log-ref", "ci/staging/wave3-pda-m2-scan/123",
                "--m3-scan-log-ref", "ci/staging/wave3-pda-m3-scan/123",
                "--offline-replay-log-ref", "ci/staging/wave3-pda-offline-replay/123",
                "--idempotency-replay-log-ref", "ci/staging/wave3-pda-idempotency-replay/123",
                "--audit-event-query-ref", "ci/staging/wave3-pda-audit-event/123",
                "--l7-run-ref", "ci/staging/wave3-pda-l7/123",
                "--usability-review-ref", "s3://wms-staging-evidence/wave3/pda/usability-review.md",
                "--barcode-samples-scanned", "50",
                "--m2-operations-exercised", "1",
                "--m3-operations-exercised", "1",
                "--offline-replays-exercised", "50",
                "--idempotency-replays-exercised", "50",
                "--real-pda-used",
                "--physical-scan-key-verified",
                "--dev-or-staging-service-verified",
                "--audit-event-verified",
                "--l7-review-completed",
                "--usability-review-completed",
            ],
        ),
        (
            "record_wave4_external_dependencies",
            "wave-4-external-dependencies.json",
            [
                "--environment", "staging",
                "--api-doc-ref", "s3://wms-staging-evidence/wave4/masangfangxin/api-doc.pdf",
                "--auth-doc-ref", "s3://wms-staging-evidence/wave4/masangfangxin/auth-doc.pdf",
                "--error-code-doc-ref", "s3://wms-staging-evidence/wave4/masangfangxin/error-codes.pdf",
                "--rate-limit-doc-ref", "s3://wms-staging-evidence/wave4/masangfangxin/rate-limit.pdf",
                "--credential-ref", "vault://wms/staging/masangfangxin",
                "--success-report-log-ref", "ci/staging/wave4-traceability-success/123",
                "--failure-retry-log-ref", "ci/staging/wave4-traceability-retry/123",
                "--audit-event-query-ref", "ci/staging/wave4-traceability-audit/123",
                "--reported-events", "1",
                "--failed-events-exercised", "1",
                "--pending-replay-queue-verified",
            ],
        ),
        (
            "record_wave5_hardware_evidence",
            "wave-5-hardware-evidence.json",
            [
                "--environment", "staging",
                "--station-code", "PK-STAGING-01",
                "--scale-device-ref", "asset://wms-staging/hardware/scale-01",
                "--bluetooth-printer-ref", "asset://wms-staging/hardware/bluetooth-printer-01",
                "--waybill-printer-ref", "asset://wms-staging/hardware/waybill-printer-01",
                "--calibration-record-ref", "s3://wms-staging-evidence/wave5/hardware/calibration.pdf",
                "--scale-reading-log-ref", "ci/staging/wave5-scale-reading/123",
                "--bluetooth-print-log-ref", "ci/staging/wave5-bluetooth-print/123",
                "--waybill-print-log-ref", "ci/staging/wave5-waybill-print/123",
                "--audit-event-query-ref", "ci/staging/wave5-hardware-audit/123",
                "--scale-readings-recorded", "1",
                "--bluetooth-labels-printed", "1",
                "--waybills-printed", "1",
                "--hardware-connected",
                "--print-artifacts-reviewed",
                "--audit-event-verified",
            ],
        ),
        (
            "record_wave5_tms_evidence",
            "wave-5-tms-evidence.json",
            [
                "--environment", "staging",
                "--tms-system-ref", "vendor://wms-staging/tms/vendor-a",
                "--dispatch-push-log-ref", "ci/staging/wave5-tms-dispatch/123",
                "--callback-log-ref", "ci/staging/wave5-tms-callback/123",
                "--failure-retry-log-ref", "ci/staging/wave5-tms-retry/123",
                "--audit-event-query-ref", "ci/staging/wave5-tms-audit/123",
                "--credential-ref", "vault://wms/staging/tms/vendor-a",
                "--dispatches-received", "1",
                "--callbacks-received", "1",
                "--failed-callbacks-exercised", "1",
                "--retry-succeeded",
                "--audit-event-verified",
            ],
        ),
        (
            "record_wave6_deploy_evidence",
            "wave-6-deploy-evidence.json",
            [
                "--environment", "staging",
                "--deployment-mode", "kubernetes",
                "--release-version", "wms-staging-20260604.1",
                "--release-plan-ref", "ticket://staging-release-plan/WMS-20260604",
                "--artifact-ref", "registry://wms-staging/wms-api:20260604.1",
                "--canary-config-ref", "git://wms/deploy/staging/canary-20260604.yaml",
                "--smoke-gate-ref", "ci/staging/wave6-smoke-gate/123",
                "--observability-dashboard-ref", "grafana/staging/wave6-release/123",
                "--rollback-drill-log-ref", "ci/staging/wave6-rollback-drill/123",
                "--approval-record-ref", "ticket://staging-release-approval/WMS-20260604",
                "--audit-event-query-ref", "ci/staging/wave6-deploy-audit/123",
                "--canary-stages-exercised", "1",
                "--smoke-checks-passed", "1",
                "--rollback-drills-exercised", "1",
                "--canary-used",
                "--full-release-blocked",
                "--rollback-verified",
                "--audit-event-verified",
                "--dual-approval-recorded",
            ],
        ),
    ]

    for module_name, filename, args in cases:
        recorder = importlib.import_module(module_name)
        output = tmp_path / filename
        output.write_text("{}", encoding="utf-8")
        command = ["--output", str(output), *args]

        assert recorder.main(command) == 1, module_name
        assert output.read_text(encoding="utf-8") == "{}"
        assert recorder.main([*command, "--force"]) == 0, module_name
        assert json.loads(output.read_text(encoding="utf-8"))["environment"] == "staging"


