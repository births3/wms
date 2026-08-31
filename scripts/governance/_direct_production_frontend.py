"""ADR-0043 直接生产前端工作流的共享治理判定。"""
from __future__ import annotations

from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
ADR_FILE = REPO_ROOT / "docs" / "adr" / "0043-direct-production-frontend-workflow.md"
WEB_ADMIN = REPO_ROOT / "apps" / "web-admin"
QUALITY_MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
UI_BUSINESS_BARREL = REPO_ROOT / "packages" / "ui" / "src" / "business" / "index.ts"


def is_direct_production_frontend() -> bool:
    """项目是否已正式切换到 ADR-0043 的直接生产前端工作流。"""
    if not ADR_FILE.is_file():
        return False
    text = ADR_FILE.read_text(encoding="utf-8")
    return (
        "状态：Accepted" in text
        and "不再新增原型页" in text
        and "新页面直接写入 `apps/web-admin`" in text
    )


def replacement_contract_errors() -> list[str]:
    """验证旧原型契约被生产前端契约真实替代，而非简单跳过治理。"""
    errors: list[str] = []
    if not is_direct_production_frontend():
        errors.append("ADR-0043 未处于 Accepted 的直接生产前端模式")
        return errors
    if not WEB_ADMIN.is_dir():
        errors.append("ADR-0043 要求的生产应用 apps/web-admin 不存在")
    if not QUALITY_MATRIX.is_file():
        errors.append("直接生产前端必须由 governance/quality-matrix.toml 承接故事与证据治理")
    if not UI_BUSINESS_BARREL.is_file():
        errors.append("直接生产前端必须保留 @wms/ui/business 生产组件注册入口")
    return errors
