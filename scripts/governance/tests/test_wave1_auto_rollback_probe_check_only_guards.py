"""Wave 1 auto rollback probe check-only 保护边界测试。"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_auto_rollback_probe_test_helpers import run_auto_rollback_probe, write_fake_bin


def _run_probe_check_only(
    tmp_path,
    args: list[str],
    fake_commands: tuple[str, ...],
):
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir(exist_ok=True)
    for command in fake_commands:
        write_fake_bin(bin_dir, command)

    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}
    return run_auto_rollback_probe(args, env=env)


def test_wave1_auto_rollback_probe_check_only_rejects_bad_evidence_refs(tmp_path):
    """check-only 也必须拒绝 prod/stub evidence 引用。"""
    result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "https://smoke.dev.wms.internal/wms/healthz",
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-prod-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-dev-stub-evidence/wave1/smoke-alert.log",
        ],
        ("curl", "kubectl"),
    )

    assert result.returncode == 2
    assert "--rollback-log-ref" in result.stderr


def test_wave1_auto_rollback_probe_check_only_rejects_local_named_boundaries(tmp_path):
    """check-only 必须拒绝 local 命名的信号和 evidence 引用。"""
    result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "https://smoke.local.wms.internal/dev/healthz",
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-dev-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-dev-evidence/wave1/smoke-alert.log",
        ],
        ("curl", "kubectl"),
    )

    assert result.returncode == 2
    assert "local boundary" in result.stderr

    evidence_ref_result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "https://smoke.dev.wms.internal/dev/healthz",
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-local-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-dev-evidence/wave1/smoke-alert.log",
        ],
        ("curl", "kubectl"),
    )

    assert evidence_ref_result.returncode == 2
    assert "local boundary" in evidence_ref_result.stderr


def test_wave1_auto_rollback_probe_local_allow_env_cannot_bypass_boundary(tmp_path):
    """遗留 local 测试开关不能绕过真实 evidence 边界。"""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    write_fake_bin(bin_dir, "curl")
    write_fake_bin(bin_dir, "kubectl")

    env = {
        **os.environ,
        "PATH": f"{bin_dir}:{os.environ['PATH']}",
        "WAVE1_ALLOW_LOCAL_TEST_SIGNAL": "true",
    }
    result = run_auto_rollback_probe(
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "http://localhost:8080/dev/healthz",
        ],
        env=env,
    )

    assert result.returncode == 2
    assert "local boundary" in result.stderr


def test_wave1_auto_rollback_probe_check_only_rejects_template_placeholders(tmp_path):
    """check-only 必须在写 evidence 前拒绝模板占位。"""
    signal_result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
            "--smoke-url",
            "https://smoke.dev.wms.internal/TBD/healthz",
        ],
        ("curl", "kubectl"),
    )

    assert signal_result.returncode == 2
    assert "template placeholder" in signal_result.stderr

    evidence_ref_result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "staging",
            "--target",
            "k8s",
            "--context",
            "wms-staging",
            "--namespace",
            "wms-staging",
            "--smoke-url",
            "https://smoke.staging.wms.internal/wms/healthz",
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-staging-evidence/wave1/TODO-rollback.log",
            "--external-log-ref",
            "s3://wms-staging-evidence/wave1/smoke-alert.log",
        ],
        ("curl", "kubectl"),
    )

    assert evidence_ref_result.returncode == 2
    assert "template placeholder" in evidence_ref_result.stderr


def test_wave1_auto_rollback_probe_check_only_rejects_existing_evidence_without_force(tmp_path):
    """check-only 应提前发现 evidence 已存在，避免采集阶段才失败。"""
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"
    evidence_file.parent.mkdir(parents=True)
    evidence_file.write_text('{"existing": true}\n', encoding="utf-8")

    args = [
        "--check-only",
        "--environment",
        "staging",
        "--target",
        "k8s",
        "--context",
        "wms-staging",
        "--namespace",
        "wms-staging",
        "--smoke-url",
        "https://smoke.staging.wms.internal/wms/healthz",
        "--evidence-file",
        str(evidence_file),
        "--rollback-log-ref",
        "s3://wms-staging-evidence/wave1/rollback.log",
        "--external-log-ref",
        "s3://wms-staging-evidence/wave1/smoke-alert.log",
    ]
    result = _run_probe_check_only(tmp_path, args, ("curl", "kubectl"))

    assert result.returncode == 2
    assert "already exists; pass --force to overwrite" in result.stderr
    assert evidence_file.read_text(encoding="utf-8") == '{"existing": true}\n'

    force_result = _run_probe_check_only(tmp_path, [*args, "--force"], ("curl", "kubectl"))

    assert force_result.returncode == 0
    assert "readiness ok environment=staging target=k8s signal=http" in force_result.stdout
    assert evidence_file.read_text(encoding="utf-8") == '{"existing": true}\n'


def test_wave1_auto_rollback_probe_check_only_rejects_prometheus_without_env_url(tmp_path):
    """probe check-only 必须和最终 evidence validator 一样要求 Prometheus URL 带环境标记。"""
    compose_dir = tmp_path / "wms-dev"
    compose_dir.mkdir()
    compose_file = compose_dir / "docker-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")

    result = _run_probe_check_only(
        tmp_path,
        [
            "--check-only",
            "--environment",
            "dev",
            "--target",
            "docker-compose",
            "--previous-version",
            "previous-dev-sha",
            "--compose-file",
            str(compose_file),
            "--prometheus-url",
            "https://prometheus.wms.internal",
            "--promql",
            'wms_wave1_rollback_signal{environment="dev"}',
            "--evidence-file",
            str(tmp_path / "wave-1-runtime-evidence.json"),
            "--rollback-log-ref",
            "s3://wms-dev-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-dev-evidence/wave1/prometheus-alert.log",
        ],
        ("curl", "docker"),
    )

    assert result.returncode == 2
    assert "Prometheus URL" in result.stderr
