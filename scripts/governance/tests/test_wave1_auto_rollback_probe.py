"""Wave 1 auto rollback probe 治理测试。"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_auto_rollback_probe_test_helpers import run_auto_rollback_probe


def test_wave1_auto_rollback_probe_requires_runtime_signal():
    """没有真实 smoke/Prometheus 信号时，自动回滚 probe 必须拒绝产出证据。"""
    result = run_auto_rollback_probe(
        [
            "--environment",
            "dev",
            "--target",
            "k8s",
            "--context",
            "wms-dev",
            "--namespace",
            "wms-dev",
        ],
    )

    assert result.returncode == 2
    assert "missing runtime evidence" in result.stderr


def test_wave1_auto_rollback_probe_check_only_does_not_call_signal_or_rollback(tmp_path):
    """check-only 只校验边界与引用，不请求 signal、不执行 rollback、不写 evidence。"""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    command_log = tmp_path / "commands.log"
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"

    for command in ["curl", "kubectl", "docker"]:
        stub = bin_dir / command
        stub.write_text(
            f"#!/usr/bin/env bash\nprintf '{command} called %s\\n' \"$*\" >> {command_log}\nexit 7\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)

    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}
    result = run_auto_rollback_probe(
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
            str(evidence_file),
            "--rollback-log-ref",
            "s3://wms-staging-evidence/wave1/rollback.log",
            "--external-log-ref",
            "s3://wms-staging-evidence/wave1/smoke-alert.log",
        ],
        env=env,
    )

    assert result.returncode == 0
    assert "readiness ok environment=staging target=k8s signal=http" in result.stdout
    assert not command_log.exists()
    assert not evidence_file.exists()
