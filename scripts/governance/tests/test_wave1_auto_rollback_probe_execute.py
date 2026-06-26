"""Wave 1 auto rollback probe 执行路径治理测试。"""
import os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_auto_rollback_probe_test_helpers import (
    run_auto_rollback_probe,
    write_fake_bin,
)


def _run_probe(args: list[str], bin_dir: Path):
    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}:{env['PATH']}"
    return run_auto_rollback_probe(args, env=env)


@pytest.mark.parametrize(
    ("environment", "target", "expected"),
    [
        ("dev", "k8s", "kubectl rollout undo deployment/wms-api --context wms-dev --namespace wms-dev"),
        ("staging", "docker-compose", "docker WMS_VERSION=previous-staging-sha args=compose"),
    ],
)
def test_wave1_auto_rollback_probe_enters_execute_path_on_real_signal_failure(
    tmp_path, monkeypatch, environment, target, expected
):
    """真实 HTTP signal 失败时，probe 才能进入 rollback --execute 路径。"""
    import json

    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    command_log = tmp_path / "rollback-command.log"
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"
    write_fake_bin(bin_dir, "curl", "#!/usr/bin/env bash\nexit 7\n")
    write_fake_bin(
        bin_dir,
        "kubectl",
        f"#!/usr/bin/env bash\nprintf 'kubectl %s\\n' \"$*\" >> {command_log!s}\n",
    )
    write_fake_bin(
        bin_dir,
        "docker",
        f"#!/usr/bin/env bash\nprintf 'docker WMS_VERSION=%s args=%s\\n' \"${{WMS_VERSION:-}}\" \"$*\" >> {command_log!s}\n",
    )

    args = [
        "--environment",
        environment,
        "--target",
        target,
        "--smoke-url",
        f"https://smoke.{environment}.wms.internal/wms/healthz",
        "--curl-max-time",
        "1",
        "--evidence-file",
        str(evidence_file),
        "--rollback-log-ref",
        f"s3://wms-{environment}-evidence/wave1/rollback.log",
        "--external-log-ref",
        f"s3://wms-{environment}-evidence/wave1/smoke-alert.log",
    ]
    if target == "k8s":
        args += ["--context", f"wms-{environment}", "--namespace", f"wms-{environment}"]
    else:
        compose_dir = tmp_path / f"wms-{environment}"
        compose_dir.mkdir()
        compose_file = compose_dir / "compose.yml"
        compose_file.write_text("services: {}\n", encoding="utf-8")
        compose_env_file = compose_dir / "staging.env"
        compose_env_file.write_text("WMS_STAGING_API_PORT=18080\n", encoding="utf-8")
        args += [
            "--previous-version",
            f"previous-{environment}-sha",
            "--compose-file",
            str(compose_file),
            "--compose-env-file",
            str(compose_env_file),
        ]

    result = _run_probe(args, bin_dir)

    assert result.returncode == 0
    assert f"environment={environment} target={target}" in result.stdout
    assert "runtime signal failed; invoking rollback" in result.stdout
    command_output = command_log.read_text(encoding="utf-8")
    assert expected in command_output
    if target == "docker-compose":
        assert "--env-file" in command_output
        assert " -f " in command_output
        assert "up -d --no-build" in command_output
    evidence = json.loads(evidence_file.read_text(encoding="utf-8"))
    assert evidence["environment"] == environment
    assert evidence["signal_type"] == "http"
    assert evidence["signal_url"] == f"https://smoke.{environment}.wms.internal/wms/healthz"
    assert evidence["rollback_triggered"] is True
    assert evidence["rollback_exit_code"] == 0
    import report_wave1_completion as report

    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)
    ok, message = report.valid_w1d_runtime_evidence()
    assert ok is True
    assert "自动回滚证据" in message


def test_wave1_auto_rollback_probe_requires_force_to_overwrite_existing_evidence(tmp_path):
    """自动回滚 probe 不能静默覆盖已有真实 evidence。"""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    command_log = tmp_path / "rollback-command.log"
    evidence_file = tmp_path / "docs" / "retros" / "wave-1-runtime-evidence.json"
    evidence_file.parent.mkdir(parents=True)
    evidence_file.write_text('{"existing": true}\n', encoding="utf-8")

    write_fake_bin(bin_dir, "curl", "#!/usr/bin/env bash\nexit 7\n")
    write_fake_bin(
        bin_dir,
        "kubectl",
        f"#!/usr/bin/env bash\nprintf 'kubectl %s\\n' \"$*\" >> {command_log!s}\n",
    )

    args = [
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
        "--curl-max-time",
        "1",
        "--evidence-file",
        str(evidence_file),
        "--rollback-log-ref",
        "s3://wms-dev-evidence/wave1/rollback.log",
        "--external-log-ref",
        "s3://wms-dev-evidence/wave1/smoke-alert.log",
    ]

    result = _run_probe(args, bin_dir)

    assert result.returncode == 2
    assert "already exists; pass --force to overwrite" in result.stderr
    assert evidence_file.read_text(encoding="utf-8") == '{"existing": true}\n'

    force_result = _run_probe([*args, "--force"], bin_dir)

    assert force_result.returncode == 0
    assert '"existing": true' not in evidence_file.read_text(encoding="utf-8")
