"""Wave 4/5/6 runtime evidence record 脚本输出与校验测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _assert_recorder_writes_valid_evidence(tmp_path, recorder, validator, filename, args):
    output = tmp_path / filename

    assert recorder.main(["--output", str(output), *args]) == 0

    ok, message = validator.validate_one(output, allow_example_refs=False)

    assert ok is True
    assert "内容有效" in message


def test_record_wave5_hardware_evidence_writes_valid_evidence(tmp_path):
    """Wave 5 硬件记录脚本生成的 evidence 必须能被 validator 接受。"""
    import record_wave5_hardware_evidence as recorder
    import validate_wave5_hardware_evidence as validator

    _assert_recorder_writes_valid_evidence(tmp_path, recorder, validator, "wave-5-hardware-evidence.json", [
        "--environment", "staging",
        "--station-code", "PK-STAGING-01",
        "--scale-device-ref", "asset://wms-staging/hardware/scale-01",
        "--bluetooth-printer-ref", "asset://wms-staging/hardware/bluetooth-printer-01",
        "--waybill-printer-ref", "asset://wms-staging/hardware/waybill-printer-01",
        "--calibration-record-ref", "s3://wms-staging-evidence/wave5/hardware/calibration.pdf",
        "--scale-reading-log-ref", "ci/staging/wave5-hardware-scale/123",
        "--bluetooth-print-log-ref", "ci/staging/wave5-hardware-bluetooth-print/123",
        "--waybill-print-log-ref", "ci/staging/wave5-hardware-waybill-print/123",
        "--audit-event-query-ref", "ci/staging/wave5-hardware-audit/123",
        "--scale-readings-recorded", "1",
        "--bluetooth-labels-printed", "1",
        "--waybills-printed", "1",
        "--hardware-connected",
        "--print-artifacts-reviewed",
        "--audit-event-verified",
    ])


def test_record_wave4_external_dependency_evidence_writes_valid_evidence(tmp_path):
    """Wave 4 外部依赖记录脚本生成的 evidence 必须能被 validator 接受。"""
    import record_wave4_external_dependencies as recorder
    import validate_wave4_external_dependencies as validator

    _assert_recorder_writes_valid_evidence(tmp_path, recorder, validator, "wave-4-external-dependencies.json", [
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
    ])


def test_record_wave5_tms_evidence_writes_valid_evidence(tmp_path):
    """Wave 5 TMS 记录脚本生成的 evidence 必须能被 validator 接受。"""
    import record_wave5_tms_evidence as recorder
    import validate_wave5_tms_evidence as validator

    _assert_recorder_writes_valid_evidence(tmp_path, recorder, validator, "wave-5-tms-evidence.json", [
        "--environment", "staging",
        "--tms-system-ref", "partner://wms-staging/tms/vendor-a",
        "--dispatch-push-log-ref", "ci/staging/wave5-tms-dispatch-push/123",
        "--callback-log-ref", "ci/staging/wave5-tms-callback/123",
        "--failure-retry-log-ref", "ci/staging/wave5-tms-failure-retry/123",
        "--audit-event-query-ref", "ci/staging/wave5-tms-audit/123",
        "--credential-ref", "vault://wms/staging/tms/vendor-a",
        "--dispatches-received", "1",
        "--callbacks-received", "1",
        "--failed-callbacks-exercised", "1",
        "--retry-succeeded",
        "--audit-event-verified",
    ])


def test_record_wave6_deploy_evidence_writes_valid_evidence(tmp_path):
    """Wave 6 灰度发布记录脚本生成的 evidence 必须能被 validator 接受。"""
    import record_wave6_deploy_evidence as recorder
    import validate_wave6_deploy_evidence as validator

    _assert_recorder_writes_valid_evidence(tmp_path, recorder, validator, "wave-6-deploy-evidence.json", [
        "--environment", "staging",
        "--deployment-mode", "kubernetes",
        "--release-version", "wms-api-20260604.1",
        "--release-plan-ref", "s3://wms-staging-evidence/wave6/deploy/release-plan.md",
        "--artifact-ref", "registry://wms-staging/api@sha256:abcdef",
        "--canary-config-ref", "gitlab/staging/wave6-canary-config/123",
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
    ])


def test_record_wave6_deploy_evidence_rejects_full_release_before_write(tmp_path):
    """Wave 6 灰度发布记录脚本不能写入全量直发 evidence。"""
    import record_wave6_deploy_evidence as recorder

    output = tmp_path / "wave-6-deploy-evidence.json"

    assert recorder.main([
        "--output", str(output),
        "--environment", "staging",
        "--deployment-mode", "kubernetes",
        "--release-version", "wms-api-20260604.1",
        "--release-plan-ref", "s3://wms-staging-evidence/wave6/deploy/release-plan.md",
        "--artifact-ref", "registry://wms-staging/api@sha256:abcdef",
        "--canary-config-ref", "gitlab/staging/wave6-canary-config/123",
        "--smoke-gate-ref", "ci/staging/wave6-smoke-gate/123",
        "--observability-dashboard-ref", "grafana/staging/wave6-release/123",
        "--rollback-drill-log-ref", "ci/staging/wave6-rollback-drill/123",
        "--approval-record-ref", "ticket://staging-release-approval/WMS-20260604",
        "--audit-event-query-ref", "ci/staging/wave6-deploy-audit/123",
        "--canary-stages-exercised", "1",
        "--smoke-checks-passed", "1",
        "--rollback-drills-exercised", "1",
        "--canary-used",
        "--rollback-verified",
        "--audit-event-verified",
        "--dual-approval-recorded",
    ]) == 1

    assert not output.exists()
