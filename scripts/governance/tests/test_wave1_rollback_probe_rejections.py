"""Wave 1 rollback 脚本拒绝路径测试。"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_rollback_probe_test_helpers import run_rollback_script


def test_wave1_rollback_script_rejects_execute_without_boundary(tmp_path):
    """真实执行必须显式给出可审计的目标边界。"""
    compose_file = tmp_path / "compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")

    k8s = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "dev",
        "--execute",
    )
    assert k8s.returncode == 2
    assert "--context and --namespace are required" in k8s.stderr

    compose = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--execute",
    )
    assert compose.returncode == 2
    assert "--compose-file is required" in compose.stderr

    missing_compose_file = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(compose_file.with_name("missing.yml")),
        "--execute",
    )
    assert missing_compose_file.returncode == 2
    assert "--compose-file must point to an existing file" in missing_compose_file.stderr


def test_wave1_rollback_script_execute_rejects_environment_boundary_mismatch(tmp_path):
    """真实执行拒绝 environment 与实际执行边界不一致的参数。"""
    prod_compose = tmp_path / "prod-compose.yml"
    prod_compose.write_text("services: {}\n", encoding="utf-8")
    local_compose = tmp_path / "local-compose.yml"
    local_compose.write_text("services: {}\n", encoding="utf-8")
    staging_compose = tmp_path / "staging-compose.yml"
    staging_compose.write_text("services: {}\n", encoding="utf-8")

    k8s_prod_boundary = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "dev",
        "--context",
        "wms-prod",
        "--namespace",
        "prod",
        "--execute",
    )
    assert k8s_prod_boundary.returncode == 2
    assert "must not point to a production boundary" in k8s_prod_boundary.stderr

    k8s_local_boundary = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "dev",
        "--context",
        "wms-local-dev",
        "--namespace",
        "wms-dev",
        "--execute",
    )
    assert k8s_local_boundary.returncode == 2
    assert "must not point to a local boundary" in k8s_local_boundary.stderr

    k8s_wrong_environment = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "dev",
        "--context",
        "wms-staging",
        "--namespace",
        "wms-staging",
        "--execute",
    )
    assert k8s_wrong_environment.returncode == 2
    assert "must include the selected environment token (dev)" in k8s_wrong_environment.stderr

    compose_prod_boundary = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(prod_compose),
        "--execute",
    )
    assert compose_prod_boundary.returncode == 2
    assert "must not point to a production boundary" in compose_prod_boundary.stderr

    compose_local_boundary = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(local_compose),
        "--execute",
    )
    assert compose_local_boundary.returncode == 2
    assert "must not point to a local boundary" in compose_local_boundary.stderr

    compose_wrong_environment = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(staging_compose),
        "--execute",
    )
    assert compose_wrong_environment.returncode == 2
    assert "must include the selected environment token (dev)" in compose_wrong_environment.stderr


def test_wave1_rollback_script_execute_rejects_stub_and_placeholder_boundaries(tmp_path):
    """真实 rollback 执行边界不能使用 mock/stub 或模板占位。"""
    mock_compose = tmp_path / "wms-dev-mock-compose.yml"
    mock_compose.write_text("services: {}\n", encoding="utf-8")
    staging_compose = tmp_path / "wms-staging-compose.yml"
    staging_compose.write_text("services: {}\n", encoding="utf-8")

    k8s_placeholder = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "dev",
        "--context",
        "wms-dev-TBD",
        "--namespace",
        "wms-dev",
        "--execute",
    )
    assert k8s_placeholder.returncode == 2
    assert "template placeholder" in k8s_placeholder.stderr

    compose_mock_boundary = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "dev",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(mock_compose),
        "--execute",
    )
    assert compose_mock_boundary.returncode == 2
    assert "stub/mock/fake" in compose_mock_boundary.stderr

    previous_version_placeholder = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "staging",
        "--previous-version",
        "TBD",
        "--compose-file",
        str(staging_compose),
        "--execute",
    )
    assert previous_version_placeholder.returncode == 2
    assert "template placeholder" in previous_version_placeholder.stderr


@pytest.mark.parametrize(
    ("args", "expected"),
    [
        (["--target", "k8s", "--environment", "prod"], "--environment must be dev or staging"),
        (["--target", "vm", "--environment", "dev"], "--target must be k8s or docker-compose"),
    ],
)
def test_wave1_rollback_script_rejects_invalid_arguments(args, expected):
    """无效环境和目标应在执行前拒绝。"""
    result = run_rollback_script(*args)

    assert result.returncode == 2
    assert expected in result.stderr
