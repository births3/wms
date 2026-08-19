"""治理脚本公共库：check-data.toml 配置加载

用途：把治理脚本中的硬编码数据（豁免列表、引用清单等）加载为 Python 数据。

接口：
- load_appendix_references()        → list[(appendix, defined_in, expected_in[])]
- load_approval_source_exemptions() → set[story_id]

详细规则：见 governance/check-data.toml 文件头注释
"""
from __future__ import annotations

from pathlib import Path
from typing import NamedTuple

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
CHECK_DATA_TOML = REPO_ROOT / "governance" / "check-data.toml"


def _load_toml() -> dict:
    if not CHECK_DATA_TOML.exists():
        return {}
    text = CHECK_DATA_TOML.read_text(encoding="utf-8")
    try:
        import tomllib
        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        return tomli.loads(text)


class AppendixRef(NamedTuple):
    appendix: str
    defined_in: str
    expected_in: list[str]


def load_appendix_references() -> list[AppendixRef]:
    """加载附录跨模块引用规则。"""
    data = _load_toml()
    refs: list[AppendixRef] = []
    for r in data.get("appendix_references", []):
        refs.append(AppendixRef(
            appendix=r["appendix"],
            defined_in=r["defined_in"],
            expected_in=list(r.get("expected_in", [])),
        ))
    return refs


def load_approval_source_exemptions() -> set[str]:
    """加载审批源链路豁免故事 ID 列表。"""
    data = _load_toml()
    return {r["story_id"] for r in data.get("approval_source_exemptions", [])}
