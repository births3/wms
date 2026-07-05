#!/usr/bin/env python3
"""governance_checks.py — Tier 调度入口（T1-T4）

类别：（调度，跨类）
Tier：作为入口，按 --tier 参数调度对应 Tier 的脚本

用法：
  python3 scripts/governance/governance_checks.py --tier T1
  python3 scripts/governance/governance_checks.py --tier T2
  python3 scripts/governance/governance_checks.py --tier T3
  python3 scripts/governance/governance_checks.py --tier T4

退出码：
  0  所属 Tier 的所有脚本通过
  1  有一个或多个脚本返回 1（违规）
  2  调度器自身错误

第 0 周仅含 4 个起步脚本：
  T1: validate_environment, check_doc_links, validate_adr_index, check_commit_convention
  T2: T1 + (placeholder for diff-driven checks via task_check.py)
  T3: T2 + (placeholder for L3-L5/L8/L11 governance checks; 引入于 Wave 3+)
  T4: T3 + (placeholder for L6/L7/L9/L10 governance checks; 引入于 Wave 4+)
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent


# Tier → 该 Tier 要跑的脚本列表
# 列表顺序即执行顺序；同 Tier 全部跑完后聚合结果（不短路）
TIER_SCRIPTS: dict[str, list[str]] = {
    "T1": [
        "validate_environment.py",
        "check_doc_links.py",
        "validate_adr_index.py",
        "check_mkdocs_nav_consistency.py",
        "validate_doc_layers.py",
        "check_file_naming.py",
        "check_user_story_structure.py",
        "check_glossary_consistency.py",
        "check_approval_source_chain.py",
        "check_config_center_consistency.py",
        "check_pda_story_completeness.py",
        "check_pda_production_gate.py",
        "check_gsp_field_traceability.py",
        "check_field_coding_standards.py",
        "check_business_rules_registry.py",
        "check_system_dictionary_alignment.py",
        "check_admin_navigation.py",
        "check_runtime_route_mounts.py",
        "check_project_rtm.py",
        "check_owner_scope_sql.py",
        "check_web_design_rtm.py",
        "check_admin_list_pages_use_datagrid.py",
        "check_admin_datagrid_system_fields.py",
        "check_admin_page_query_panel.py",
        "check_quality_matrix.py",
        "check_m1_master_data_source_actions.py",
        "check_baseline_health.py",
        "check_governance_consistency.py",
        "check_governance_coverage.py",
        "check_integration_contract.py",
        "check_feature_flags.py",
        "check_changelog_freshness.py",
        "check_wave6_evidence_preflight.py",
        "check_commit_convention.py",
        "check_prototype_index_consistency.py",
        "check_prototype_story_sync.py",
        "check_prototype_freshness.py",
        "check_prototype_usability_baseline.py",
        "check_component_doc_header.py",
        "check_component_no_inline_style.py",
        "check_component_props_classname.py",
        "check_component_registry_consistency.py",
        "check_datagrid_popover_portal.py",
        "check_page_size.py",
        "check_prototype_fidelity.py",
        "check_prototype_navigation.py",
        "check_baseline_completeness.py",
        "check_e2e_matrix_completeness.py",
    ],
    "T2": [
        # T1 + diff 驱动（task_check.py）之外，T2 全量入口也要跑
        # 影响跨端契约的同步检查，避免非 diff 场景漏掉生成物漂移。
        "check_openapi_in_sync.py",
        "validate_openapi_artifacts.py",
        "check_openapi_contract.py",
        "check_prototype_review_signoff.py",
    ],
    "T3": [
        # Wave 3+ handler test coverage / idempotency / permission matrix
        # 视觉回归（重，依赖 vite + chrome 提前生成 snapshot）
        "check_visual_regression.py",
    ],
    "T4": [
        # 完整矩阵 E2E 截图报告由 just verify 的 _t4-e2e 生成，这里只校验报告。
        "check_matrix_e2e_report.py",
    ],
}

# 单脚本附加参数：全量 Tier 入口必须用严格语义运行关键契约检查。
SCRIPT_ARGS: dict[str, list[str]] = {
    "check_openapi_in_sync.py": ["--strict"],
}


@dataclass
class ScriptResult:
    name: str
    exit_code: int
    duration_ms: int


def run_script(name: str, *, json_mode: bool) -> ScriptResult:
    import time

    cmd = [sys.executable, str(SCRIPTS_DIR / name), *SCRIPT_ARGS.get(name, [])]
    if json_mode:
        cmd.append("--json")
    start = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True, text=True, check=False)
    dur = int((time.perf_counter() - start) * 1000)
    # 缩进子脚本输出，避免与调度器混排
    if p.stdout and not json_mode:
        for line in p.stdout.splitlines():
            print(f"    {line}")
    if p.stderr:
        for line in p.stderr.splitlines():
            print(f"    [err] {line}", file=sys.stderr)
    return ScriptResult(name=name, exit_code=p.returncode, duration_ms=dur)


def expand_tier_scripts(tier: str) -> list[str]:
    """累积式：T2 包含 T1，T3 包含 T2，T4 包含 T3。"""
    order = ["T1", "T2", "T3", "T4"]
    if tier not in order:
        raise SystemExit(f"unknown tier: {tier}")
    out: list[str] = []
    for t in order:
        out.extend(TIER_SCRIPTS.get(t, []))
        if t == tier:
            break
    # 去重（保留首次出现）
    seen: set[str] = set()
    deduped: list[str] = []
    for s in out:
        if s not in seen:
            seen.add(s)
            deduped.append(s)
    return deduped


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tier", required=True, choices=["T1", "T2", "T3", "T4"])
    parser.add_argument("--json", action="store_true", help="子脚本以 JSON 模式运行")
    parser.add_argument("--report-json", action="store_true", help="本调度器输出 JSON 总结")
    args = parser.parse_args(argv)

    scripts = expand_tier_scripts(args.tier)
    if not scripts:
        msg = f"no scripts registered for {args.tier} (Wave 0 placeholder)"
        if args.report_json:
            print(json.dumps({"tier": args.tier, "scripts": [], "note": msg, "ok": True}))
        else:
            print(msg)
        return 0

    print(f"▶ governance_checks {args.tier} ({len(scripts)} scripts)")

    results: list[ScriptResult] = []
    for s in scripts:
        if not (SCRIPTS_DIR / s).exists():
            print(f"  ! missing script: {s}", file=sys.stderr)
            results.append(ScriptResult(name=s, exit_code=2, duration_ms=0))
            continue
        print(f"  · running {s}")
        r = run_script(s, json_mode=args.json)
        results.append(r)

    failed = [r for r in results if r.exit_code != 0]
    total_ms = sum(r.duration_ms for r in results)

    if args.report_json:
        print(json.dumps({
            "tier": args.tier,
            "scripts": [asdict(r) for r in results],
            "total_ms": total_ms,
            "ok": not failed,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"\n▶ {args.tier} summary: {len(results) - len(failed)}/{len(results)} ok, {total_ms}ms total")
        for r in results:
            mark = "✓" if r.exit_code == 0 else "✘"
            print(f"  {mark} {r.name:<35} exit={r.exit_code} {r.duration_ms}ms")

    return 0 if not failed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except SystemExit:
        raise
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
