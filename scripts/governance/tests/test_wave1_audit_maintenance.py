"""Wave 1 H2 audit maintenance 脚本入口保护测试。"""
import os
import subprocess
from pathlib import Path


def audit_maintenance_script() -> Path:
    return Path(__file__).resolve().parents[3] / "deploy" / "scripts" / "audit_maintenance.sh"


def run_audit_maintenance(env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(audit_maintenance_script()), "--seal-date", "2026-06-06"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )


def test_audit_maintenance_rejects_forbidden_database_boundary():
    """H2 维护入口不能指向 local/prod/mock/fake/stub/example 边界。"""
    env = {
        **os.environ,
        "DATABASE_URL": "postgres://wms@localhost:5432/wms_dev",
    }

    result = run_audit_maintenance(env)

    assert result.returncode == 2
    assert "DATABASE_URL must not point to" in result.stderr


def test_audit_maintenance_rejects_database_url_template_placeholder():
    """H2 维护入口不能使用模板占位 DATABASE_URL。"""
    env = {
        **os.environ,
        "DATABASE_URL": "postgres://wms@pg-dev.wms.internal:5432/wms_TBD",
    }

    result = run_audit_maintenance(env)

    assert result.returncode == 2
    assert "template placeholder" in result.stderr


def test_audit_maintenance_default_binary_matches_cargo_target():
    """维护脚本默认 binary 必须能由 Cargo 和镜像构建产出。"""
    script = audit_maintenance_script().read_text(encoding="utf-8")
    cargo = Path("backend/crates/api/Cargo.toml").read_text(encoding="utf-8")
    dockerfile = Path("backend/Dockerfile.wms-api").read_text(encoding="utf-8")

    assert 'name = "audit-maintenance"' in cargo
    assert 'AUDIT_MAINTENANCE_BIN:-audit-maintenance' in script
    assert "cargo build --release --bin audit-maintenance" in dockerfile
    assert "cp /app/backend/target/release/audit-maintenance /tmp/audit-maintenance" in dockerfile
    assert "COPY --from=builder /tmp/audit-maintenance /app/audit-maintenance" in dockerfile
