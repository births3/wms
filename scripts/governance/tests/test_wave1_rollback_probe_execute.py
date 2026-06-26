"""Wave 1 rollback 脚本 execute 路径测试。"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave1_rollback_probe_test_helpers import run_rollback_script


def test_wave1_rollback_script_execute_k8s_uses_explicit_boundary(tmp_path):
    """k8s 执行路径必须把 context/namespace 传给 kubectl。"""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    log_file = tmp_path / "kubectl.log"
    kubectl = bin_dir / "kubectl"
    kubectl.write_text(
        f"#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" > {log_file}\n",
        encoding="utf-8",
    )
    kubectl.chmod(0o755)
    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}

    result = run_rollback_script(
        "--target",
        "k8s",
        "--environment",
        "staging",
        "--context",
        "wms-staging",
        "--namespace",
        "wms-staging",
        "--execute",
        env=env,
    )

    assert result.returncode == 0
    assert "context=wms-staging namespace=wms-staging" in result.stdout
    assert log_file.read_text(encoding="utf-8").strip() == (
        "rollout undo deployment/wms-api --context wms-staging --namespace wms-staging"
    )


def test_wave1_rollback_script_execute_compose_uses_explicit_file(tmp_path):
    """docker-compose 执行路径必须把 compose file 和 WMS_VERSION 传给 docker。"""
    compose_file = tmp_path / "staging-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    compose_env_file = tmp_path / "staging.env"
    compose_env_file.write_text("WMS_STAGING_API_PORT=18080\n", encoding="utf-8")
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    log_file = tmp_path / "docker.log"
    docker = bin_dir / "docker"
    docker.write_text(
        f"#!/usr/bin/env bash\nprintf 'WMS_VERSION=%s args=%s\\n' \"$WMS_VERSION\" \"$*\" > {log_file}\n",
        encoding="utf-8",
    )
    docker.chmod(0o755)
    env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}"}

    result = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "staging",
        "--previous-version",
        "abc123",
        "--compose-file",
        str(compose_file),
        "--compose-env-file",
        str(compose_env_file),
        "--execute",
        env=env,
    )

    assert result.returncode == 0
    assert f"compose_file={compose_file}" in result.stdout
    assert f"compose_env_file={compose_env_file}" in result.stdout
    assert log_file.read_text(encoding="utf-8").strip() == (
        f"WMS_VERSION=abc123 args=compose --env-file {compose_env_file} -f {compose_file} up -d --no-build"
    )


def test_wave1_rollback_script_execute_compose_honors_docker_wrapper(tmp_path):
    """docker-compose 执行路径可通过 WAVE1_DOCKER_BIN 指向受控 Docker wrapper。"""
    compose_file = tmp_path / "staging-compose.yml"
    compose_file.write_text("services: {}\n", encoding="utf-8")
    log_file = tmp_path / "docker-wrapper.log"
    docker_wrapper = tmp_path / "docker-wrapper"
    docker_wrapper.write_text(
        f"#!/usr/bin/env bash\nprintf 'wrapper WMS_VERSION=%s args=%s\\n' \"$WMS_VERSION\" \"$*\" > {log_file}\n",
        encoding="utf-8",
    )
    docker_wrapper.chmod(0o755)
    env = {**os.environ, "WAVE1_DOCKER_BIN": str(docker_wrapper)}

    result = run_rollback_script(
        "--target",
        "docker-compose",
        "--environment",
        "staging",
        "--previous-version",
        "previous-staging-sha",
        "--compose-file",
        str(compose_file),
        "--execute",
        env=env,
    )

    assert result.returncode == 0
    assert log_file.read_text(encoding="utf-8").strip() == (
        f"wrapper WMS_VERSION=previous-staging-sha args=compose -f {compose_file} up -d --no-build"
    )
