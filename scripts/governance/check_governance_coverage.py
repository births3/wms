#!/usr/bin/env python3
"""check_governance_coverage.py — 治理脚本自身覆盖性元检查

类别：1. 文档治理（元治理：守护治理脚本注册表）
Tier：T1（< 10s，纯静态扫描）
输入：scripts/governance/*.py + governance_checks.py + task_check.py 引用
      + governance/gate-rules.toml + justfile + tests/test_smoke.py
输出：人类可读 + --json
退出码：
  0  每个 check_*/validate_* 脚本和被 gate 使用的 report_* 脚本都被运行器覆盖且纳入 smoke 测试
  1  存在未注册（孤儿）或未测试的脚本
  2  脚本自身错误

背景：
  治理脚本不断新增，但容易"写了却没接线"——
  既没进 governance_checks.py 的 Tier 全量扫描，
  也没被 gate-rules.toml 的 diff 触发引用，更没纳入 smoke 测试。
  这类脚本永远不会跑，等于没写。此脚本守护"写了就必须被覆盖"。

覆盖维度（每个 check_*/validate_* 脚本和被 gate 使用的 report_* 脚本都必须满足）：
  R = reachable   被某运行器覆盖：在 TIER_SCRIPTS 列表里、被 gate-rules.toml
                  引用，或由 justfile 显式编排（适用于需要参数/环境的脚本）
  S = smoke       纳入 tests/test_smoke.py 的通用 smoke 测试清单

smoke 豁免：
  少数脚本本质不适合"快速 smoke"（依赖 cargo/chrome，或 warning-only
  故意打破 ok⟺exit 契约）。这些在 SMOKE_EXEMPT 显式登记理由，免除 S 维度，
  但仍强制 R 维度（必须被某运行器覆盖）。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = SCRIPTS_DIR.parent.parent
GATE_RULES = REPO_ROOT / "governance" / "gate-rules.toml"
JUSTFILE = REPO_ROOT / "justfile"
SMOKE_TEST = SCRIPTS_DIR / "tests" / "test_smoke.py"

# 不参与覆盖性统计的脚本：调度器自身、辅助库（_ 前缀）、本元检查脚本
EXCLUDE = {"governance_checks.py", "task_check.py", _THIS.name}
SCRIPT_RE = re.compile(r'"([a-z_][a-z0-9_]*\.py)"')

# smoke 豁免：本质不适合"快速 smoke"（< 10s、无外部依赖）的脚本，必须写明理由。
# 仍要求 reachable（被运行器覆盖），仅免除 smoke 维度。
SMOKE_EXEMPT = {
    "check_openapi_in_sync.py": "T2 契约同步脚本，依赖 cargo + pnpm/openapi-typescript 且可能触发编译，超 smoke 时限；已注册进 T2 TIER_SCRIPTS（reachable）",
    "check_visual_regression.py": "T3 重脚本，依赖 chrome 渲染快照，超 smoke 时限；已注册进 T3 TIER_SCRIPTS（reachable）",
    "check_story_size.py": "warning-only，故意 exit0/ok=False（拆分是建议非硬约束），违反 smoke 的 ok⟺exit 契约",
    "check_wave1_runtime_evidence_prereqs.py": "Wave 1 runtime evidence 前置检查需要 --mode 选择 h2/rollback-k8s/rollback-compose；由 justfile 多个 evidence/readiness 入口显式编排",
    "check_wave1_h2_runtime_readiness.py": "Wave 1 H2 DB readiness 需要 --database-url 指向真实 dev PostgreSQL；由 just wave-1-h2-runtime-readiness 显式编排",
    "check_wave4_external_dependencies_readiness.py": "Wave 4 外部依赖 readiness 需要真实 dev/staging 证据引用参数；由 just wave-4-external-dependencies-readiness 显式编排并由专项测试覆盖",
}

# 需要运行参数或真实环境的脚本不能由 gate-rules.toml 裸跑；这些脚本的
# 可达性由 justfile 中的人工 evidence/readiness 入口证明。
JUSTFILE_REACHABLE = {
    "check_wave1_runtime_evidence_prereqs.py",
    "check_wave1_h2_runtime_readiness.py",
    "check_wave3_pda_runtime_readiness.py",
    "check_wave4_external_dependencies_readiness.py",
    "check_wave6_deploy_readiness.py",
    "report_wave6_deploy_materials.py",
}


@dataclass
class Gap:
    script: str
    missing: str  # "reachable" | "smoke"
    detail: str


def discover_scripts() -> list[str]:
    """所有应被覆盖的治理校验脚本（含被 gate 使用的 report_*）。"""
    names = {p.name for p in SCRIPTS_DIR.glob("check_*.py")}
    names |= {p.name for p in SCRIPTS_DIR.glob("validate_*.py")}
    names |= {name for name in gate_referenced() if name.startswith("report_")}
    return sorted(n for n in names if n not in EXCLUDE and not n.startswith("_"))


def tier_registered() -> set[str]:
    """governance_checks.py TIER_SCRIPTS 中登记的脚本名。"""
    return set(SCRIPT_RE.findall((SCRIPTS_DIR / "governance_checks.py").read_text(encoding="utf-8")))


def gate_referenced() -> set[str]:
    """gate-rules.toml checks 引用的脚本名（补 .py 后缀）。"""
    try:
        import tomllib as toml
    except ModuleNotFoundError:
        import tomli as toml
    data = toml.loads(GATE_RULES.read_text(encoding="utf-8"))
    return {f"{c}.py" for r in data.get("rules", []) for c in r.get("checks", [])}


def justfile_referenced() -> set[str]:
    """justfile 中显式调用的治理脚本名。"""
    text = JUSTFILE.read_text(encoding="utf-8")
    return {
        Path(match).name
        for match in re.findall(r"scripts/governance/([a-z_][a-z0-9_]*\.py)", text)
    } & JUSTFILE_REACHABLE


def smoke_listed() -> set[str]:
    """tests/test_smoke.py GOVERNANCE_SCRIPTS 清单中的脚本名。"""
    return set(SCRIPT_RE.findall(SMOKE_TEST.read_text(encoding="utf-8")))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    scripts = discover_scripts()
    reachable = tier_registered() | gate_referenced() | justfile_referenced()
    smoke = smoke_listed()

    gaps: list[Gap] = []
    for s in scripts:
        if s not in reachable:
            gaps.append(Gap(s, "reachable", "未在 TIER_SCRIPTS 注册、未被 gate-rules.toml 引用，也未由 justfile 显式编排 → 永不执行"))
        if s not in smoke and s not in SMOKE_EXEMPT:
            gaps.append(Gap(s, "smoke", "未纳入 tests/test_smoke.py 通用 smoke 清单"))

    covered = sum(1 for s in scripts if s in reachable and (s in smoke or s in SMOKE_EXEMPT))
    total = len(scripts)

    if args.json:
        print(json.dumps({
            "check": "check_governance_coverage",
            "tier": "T1",
            "category": "文档治理",
            "total_scripts": total,
            "fully_covered": covered,
            "reachable_count": sum(1 for s in scripts if s in reachable),
            "smoke_count": sum(1 for s in scripts if s in smoke),
            "smoke_exempt": {s: r for s, r in SMOKE_EXEMPT.items() if s in scripts},
            "gaps": [asdict(g) for g in gaps],
            "ok": not gaps,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_governance_coverage (T1, 文档治理)")
        print(f"  · 发现 {total} 个 check_*/validate_* / gate report_* 脚本")
        print(f"  · 运行器可达 {sum(1 for s in scripts if s in reachable)}/{total}，"
              f"smoke 覆盖 {sum(1 for s in scripts if s in smoke)}/{total}"
              f"（豁免 {sum(1 for s in scripts if s in SMOKE_EXEMPT)}），"
              f"全维度覆盖 {covered}/{total}")
        if not gaps:
            print("  ✓ 所有脚本均被运行器覆盖且纳入 smoke 测试（或已记录豁免）")
        else:
            print(f"  ✘ {len(gaps)} 处覆盖缺口:")
            for g in gaps:
                print(f"    [{g.missing}] {g.script}: {g.detail}")

    return 0 if not gaps else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
