"""Wave 1 rollback 脚本 dry-run 测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_rollback_probe_test_helpers import run_rollback_script


def test_wave1_rollback_script_dry_run_paths():
    """Wave 1 回滚脚本默认只 dry-run 两条 ADR-0016 路径。"""
    k8s = run_rollback_script("--target", "k8s", "--environment", "dev")
    assert k8s.returncode == 0
    assert "dry-run:" in k8s.stdout
    assert "kubectl rollout undo deployment/wms-api" in k8s.stdout

    compose = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "staging",
        "--previous-version",
        "abc123",
    )
    assert compose.returncode == 0
    assert "dry-run:" in compose.stdout
    assert "WMS_VERSION=abc123 docker compose up -d --no-build" in compose.stdout


def test_wave1_rollback_script_requires_previous_version():
    """docker-compose 回滚必须显式给出上一稳定版本。"""
    result = run_rollback_script("--target", "docker-compose", "--environment", "dev")

    assert result.returncode == 2
    assert "--previous-version is required" in result.stderr
