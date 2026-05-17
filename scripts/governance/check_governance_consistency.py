#!/usr/bin/env python3
"""check_governance_consistency.py — 治理文档与配置一致性元检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/governance.md §4.6 表格 + governance/gate-rules.toml
输出：人类可读 + --json
退出码：
  0  通过（§4.6 与 gate-rules.toml 中 Wave 计划脚本一致）
  1  发现不一致
  2  脚本自身错误

背景：
  governance.md §4.6 Tier 启动 SOP 列出 Wave 1-5 计划脚本；
  governance/gate-rules.toml 也列出占位规则。
  两处必须保持一致 — 此脚本守护这一约束。

校验项：
  1. §4.6 中提到的每个脚本，应在 gate-rules.toml 中出现
  2. gate-rules.toml 中的占位规则脚本，应在 §4.6 中出现
  3. 同一脚本在两处的 Tier 标注一致

例外：
  - §4.6 标注"CI 全量，非 diff 触发"的（如 perf_baseline / api_compat）
    允许只在 §4.6 出现，不在 gate-rules.toml
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
GOVERNANCE_MD = REPO_ROOT / "docs" / "governance.md"
GATE_RULES = REPO_ROOT / "governance" / "gate-rules.toml"

# §4.6 表格行模式：| Wave N | `script_a.py` / `script_b.py` | T2 | ... |
TABLE_ROW_RE = re.compile(
    r"^\|\s*Wave\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*(T\d+)\s*\|"
)
SCRIPT_RE = re.compile(r"`([a-z_][a-z0-9_]*)\.py`")


@dataclass
class Issue:
    kind: str  # "missing_in_gate_rules" | "missing_in_doc" | "tier_mismatch"
    script: str
    detail: str


def parse_doc_section() -> dict[str, str]:
    """从 governance.md §4.6 解析 {script_name: tier}。

    跳过标记"CI 全量，非 diff 触发"的脚本（这些不需在 gate-rules.toml 出现）。
    """
    if not GOVERNANCE_MD.exists():
        return {}
    text = GOVERNANCE_MD.read_text(encoding="utf-8")

    # 定位 §4.6 段
    start = text.find("### 4.6 Tier 启动 SOP")
    if start == -1:
        return {}
    end = text.find("\n---", start)
    section = text[start:end] if end != -1 else text[start:]

    scripts: dict[str, str] = {}
    for line in section.splitlines():
        m = TABLE_ROW_RE.match(line)
        if not m:
            continue
        # 跳过"CI 全量，非 diff 触发"的行（这些不应在 gate-rules.toml 出现）
        if "非 diff 触发" in line or "CI 全量" in line:
            continue
        wave_num = m.group(1)
        scripts_cell = m.group(2)
        tier = m.group(3)
        for sm in SCRIPT_RE.finditer(scripts_cell):
            scripts[sm.group(1)] = tier
    return scripts


def parse_gate_rules() -> dict[str, str]:
    """从 gate-rules.toml 解析 {check_name: tier}（仅占位规则，跳过已实现脚本）。"""
    if not GATE_RULES.exists():
        return {}
    text = GATE_RULES.read_text(encoding="utf-8")

    try:
        import tomllib
        data = tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        data = tomli.loads(text)

    scripts: dict[str, str] = {}
    scripts_dir = REPO_ROOT / "scripts" / "governance"
    for r in data.get("rules", []):
        tier = r.get("tier", "T2")
        for c in r.get("checks", []):
            # 跳过当前已实现的脚本（不算"占位"）
            if (scripts_dir / f"{c}.py").exists():
                continue
            # 同一脚本在多规则出现时，取首次的 tier
            if c not in scripts:
                scripts[c] = tier
    return scripts


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    doc_scripts = parse_doc_section()
    gate_scripts = parse_gate_rules()

    issues: list[Issue] = []

    # 1. doc 中提到但 gate-rules 中没有
    for script, tier in doc_scripts.items():
        if script not in gate_scripts:
            issues.append(Issue(
                kind="missing_in_gate_rules",
                script=script,
                detail=f"§4.6 列为 {tier} 但 gate-rules.toml 没有占位规则",
            ))

    # 2. gate-rules 中有但 doc 没提
    for script, tier in gate_scripts.items():
        if script not in doc_scripts:
            issues.append(Issue(
                kind="missing_in_doc",
                script=script,
                detail=f"gate-rules.toml 列为 {tier} 但 §4.6 表格未提及",
            ))

    # 3. tier 不一致
    for script in set(doc_scripts) & set(gate_scripts):
        if doc_scripts[script] != gate_scripts[script]:
            issues.append(Issue(
                kind="tier_mismatch",
                script=script,
                detail=f"§4.6={doc_scripts[script]} vs gate-rules.toml={gate_scripts[script]}",
            ))

    if args.json:
        payload = {
            "check": "check_governance_consistency",
            "tier": "T1",
            "category": "文档治理",
            "doc_scripts": doc_scripts,
            "gate_rules_scripts": gate_scripts,
            "issues": [asdict(i) for i in issues],
            "ok": not issues,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_governance_consistency (T1, 文档治理)")
        print(f"  · §4.6 列 {len(doc_scripts)} 个 diff-触发脚本")
        print(f"  · gate-rules.toml 列 {len(gate_scripts)} 个占位脚本")
        if not issues:
            print("  ✓ §4.6 ↔ gate-rules.toml 完全一致")
        else:
            print(f"  ✘ {len(issues)} 处不一致:")
            for i in issues:
                print(f"    [{i.kind}] {i.script}: {i.detail}")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
