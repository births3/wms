#!/usr/bin/env python3
"""check_governance_consistency.py — 治理文档与配置一致性元检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/governance.md §4.6 表格 + governance/gate-rules.toml + justfile
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
JUSTFILE = REPO_ROOT / "justfile"

# §4.6 表格行模式：| Wave N | `script_a.py` / `script_b.py` | T2 | `path/**` |
TABLE_ROW_RE = re.compile(
    r"^\|\s*Wave\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*(T\d+)\s*\|\s*([^|]+?)\s*\|"
)
SCRIPT_RE = re.compile(r"`([a-z_][a-z0-9_]*)\.py`")
PATTERN_RE = re.compile(r"`([^`]+)`")


@dataclass
class Issue:
    kind: str  # "missing_in_gate_rules" | "missing_in_doc" | "tier_mismatch"
    script: str
    detail: str


@dataclass
class RuleSpec:
    tier: str
    patterns: set[str]


TIER_ENTRYPOINT_RECIPES = {
    "_t1-fmt",
    "_t1-governance",
    "_t2-diff-checks",
    "_t2-lint",
    "_t2-unit-tests",
    "_t2-contract-static",
    "_t3-integration",
    "_t3-governance-l3",
    "_t4-full-tests",
    "_t4-e2e",
    "_t4-perf-bench",
    "_t4-compat-check",
    "_t4-governance-l4",
}

FORCED_SUCCESS_RE = re.compile(r"(?:\|\||;\s*(?:true|exit\s+0)(?:\s|$)|\bset\s+\+e\b)")


def tier_entrypoint_issues(just_text: str, *, required: set[str] | None = None) -> list[Issue]:
    """检查主 Tier recipe 有真实命令且失败不会被吞掉。"""
    required = required or TIER_ENTRYPOINT_RECIPES
    lines = just_text.splitlines()
    bodies: dict[str, list[str]] = {}
    current: str | None = None
    for line in lines:
        if line and not line[0].isspace() and not line.startswith("#") and line.endswith(":"):
            current = line[:-1]
            bodies.setdefault(current, [])
        elif current is not None and (not line or line[0].isspace() or line.startswith("#")):
            bodies[current].append(line)
        elif line:
            current = None

    issues: list[Issue] = []
    for recipe in sorted(required):
        body = "\n".join(bodies.get(recipe, []))
        if recipe not in bodies:
            issues.append(Issue("tier_recipe_missing", recipe, "justfile 缺少主 Tier recipe"))
            continue
        has_placeholder = "placeholder" in body.lower() or "占位" in body
        if has_placeholder:
            issues.append(Issue("tier_placeholder", recipe, "主 Tier recipe 仍包含占位实现"))
        if FORCED_SUCCESS_RE.search(body):
            issues.append(Issue("tier_failure_swallowed", recipe, "主 Tier recipe 强制返回成功并吞掉失败"))
        commands = [
            line.lstrip().lstrip("@").strip()
            for line in bodies[recipe]
            if line.strip() and not line.lstrip().startswith(("#", "@#", "@echo"))
        ]
        if not has_placeholder and not any(
            command.startswith(("cargo ", "pnpm ", "python3 ", "node ", "just "))
            for command in commands
        ):
            issues.append(Issue("tier_command_missing", recipe, "主 Tier recipe 没有可执行检查命令"))
    return issues


def _doc_section_text() -> str:
    if not GOVERNANCE_MD.exists():
        return ""
    text = GOVERNANCE_MD.read_text(encoding="utf-8")
    start = text.find("### 4.6 Tier 启动 SOP")
    if start == -1:
        return ""
    end = text.find("\n---", start)
    return text[start:end] if end != -1 else text[start:]


def parse_doc_rule_specs() -> dict[str, RuleSpec]:
    """从 governance.md §4.6 解析 {script_name: RuleSpec}。

    跳过标记"CI 全量，非 diff 触发"的脚本（这些不需在 gate-rules.toml 出现）。
    """
    scripts: dict[str, RuleSpec] = {}
    for line in _doc_section_text().splitlines():
        m = TABLE_ROW_RE.match(line)
        if not m:
            continue
        # 跳过"CI 全量，非 diff 触发"的行（这些不应在 gate-rules.toml 出现）
        if "非 diff 触发" in line or "CI 全量" in line:
            continue
        scripts_cell = m.group(2)
        tier = m.group(3)
        patterns_cell = m.group(4)
        patterns = set(PATTERN_RE.findall(patterns_cell))
        for sm in SCRIPT_RE.finditer(scripts_cell):
            scripts[sm.group(1)] = RuleSpec(tier=tier, patterns=patterns)
    return scripts


def parse_doc_section() -> dict[str, str]:
    """从 governance.md §4.6 解析 {script_name: tier}。"""
    return {script: spec.tier for script, spec in parse_doc_rule_specs().items()}


def parse_gate_rule_specs() -> dict[str, RuleSpec]:
    """从 gate-rules.toml 的 Wave 计划段解析 {check_name: RuleSpec}。"""
    if not GATE_RULES.exists():
        return {}
    text = GATE_RULES.read_text(encoding="utf-8")
    start = text.find("# 占位规则")
    end = text.find("# 兜底规则", start)
    if start != -1 and end != -1:
        text = text[start:end]

    try:
        import tomllib
        data = tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli
        data = tomli.loads(text)

    scripts: dict[str, RuleSpec] = {}
    for r in data.get("rules", []):
        tier = r.get("tier", "T2")
        pattern = r.get("match", "")
        for c in r.get("checks", []):
            if c not in scripts:
                scripts[c] = RuleSpec(tier=tier, patterns=set())
            scripts[c].patterns.add(pattern)
    return scripts


def parse_gate_rules() -> dict[str, str]:
    """从 gate-rules.toml 的 Wave 计划段解析 {check_name: tier}。"""
    return {script: spec.tier for script, spec in parse_gate_rule_specs().items()}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    doc_specs = parse_doc_rule_specs()
    gate_specs = parse_gate_rule_specs()
    doc_scripts = {script: spec.tier for script, spec in doc_specs.items()}
    gate_scripts = {script: spec.tier for script, spec in gate_specs.items()}

    issues: list[Issue] = []
    issues.extend(tier_entrypoint_issues(JUSTFILE.read_text(encoding="utf-8")))

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
        missing_patterns = doc_specs[script].patterns - gate_specs[script].patterns
        if missing_patterns:
            issues.append(Issue(
                kind="pattern_mismatch",
                script=script,
                detail=(
                    "§4.6 声明但 gate-rules.toml 未覆盖的模式: "
                    f"{', '.join(sorted(missing_patterns))}"
                ),
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
