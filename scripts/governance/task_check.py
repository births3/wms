#!/usr/bin/env python3
"""task_check.py — diff 触发的最小检查集调度

类别：4. 流程治理
Tier：作为入口，根据 git diff 与 governance/gate-rules.toml 决定跑哪些脚本

详细规则：见 ADR-0003 与 ADR-0037

用法：
  python3 scripts/governance/task_check.py --tier T2
  python3 scripts/governance/task_check.py --tier T2 --base main
  WMS_GOV_CONTEXT=pr WMS_GOV_BASE=origin/main python3 scripts/governance/task_check.py --tier T2

退出码：
  0  通过
  1  有脚本失败
  2  脚本自身错误

模式：
- 本地渐进模式下，未实现的脚本仅提示并跳过。
- `--strict` 下未实现脚本记为 `error` 并阻塞，CI 必须使用该模式。
- `--context` 与 Tier 正交；CI 可通过 `WMS_GOV_CONTEXT` 提供默认场景。
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path


_THIS = Path(__file__).resolve()
SCRIPTS_DIR = _THIS.parent
REPO_ROOT = SCRIPTS_DIR.parent.parent

# 复用公共库
sys.path.insert(0, str(SCRIPTS_DIR))
from _diff import (  # noqa: E402
    get_changed_files,
    load_gate_rules,
    match_rules,
    metadata_for_check,
    rules_for_execution,
)


@dataclass
class ScriptResult:
    name: str
    matched_files: int
    exit_code: int
    duration_ms: int
    rule_ids: list[str] = field(default_factory=list)
    sources: list[str] = field(default_factory=list)
    contexts: list[str] = field(default_factory=list)
    status: str = ""

    def __post_init__(self) -> None:
        if not self.status:
            self.status = execution_status(self.exit_code)

    def set_exit_code(self, exit_code: int) -> None:
        self.exit_code = exit_code
        self.status = execution_status(exit_code)


def execution_status(exit_code: int, child_output: str = "") -> str:
    """把通用脚本退出码映射到 G3 执行状态。"""
    if exit_code == 0:
        return "passed"
    if child_output:
        try:
            reported = json.loads(child_output).get("status")
        except (json.JSONDecodeError, AttributeError):
            reported = None
        if reported in {"failed", "error", "blocked"}:
            return reported
    if exit_code == 1:
        return "failed"
    return "error"


# task_check --strict 时，这些脚本自身也必须以严格语义执行。
STRICT_SCRIPT_ARGS = {
    "check_openapi_in_sync": ["--strict"],
    "report_wave2_completion": ["--strict", "--require-runtime-evidence"],
}


def run_one(check_name: str, json_mode: bool, *, strict_mode: bool = False) -> ScriptResult:
    import time

    script = SCRIPTS_DIR / f"{check_name}.py"
    if not script.exists():
        return ScriptResult(
            name=check_name, matched_files=0, exit_code=-1, duration_ms=0
        )
    cmd = [sys.executable, str(script)]
    if strict_mode:
        cmd.extend(STRICT_SCRIPT_ARGS.get(check_name, []))
    if json_mode:
        cmd.append("--json")
    start = time.perf_counter()
    p = subprocess.run(
        cmd,
        check=False,
        capture_output=json_mode,
        text=json_mode,
    )
    dur = int((time.perf_counter() - start) * 1000)
    return ScriptResult(
        name=check_name,
        matched_files=0,
        exit_code=p.returncode,
        duration_ms=dur,
        status=execution_status(p.returncode, p.stdout if json_mode else ""),
    )


def run_t1_fallback(json_mode: bool) -> ScriptResult:
    import time

    cmd = [sys.executable, str(SCRIPTS_DIR / "governance_checks.py"), "--tier", "T1"]
    if json_mode:
        cmd.append("--json")
    start = time.perf_counter()
    result = subprocess.run(
        cmd,
        check=False,
        capture_output=json_mode,
        text=json_mode,
    )
    return ScriptResult(
        name="governance_t1_fallback",
        matched_files=0,
        exit_code=result.returncode,
        duration_ms=int((time.perf_counter() - start) * 1000),
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tier", default="T2", choices=["T1", "T2", "T3", "T4"])
    parser.add_argument(
        "--base",
        default=os.environ.get("WMS_GOV_BASE", "main"),
        help="diff base ref；默认 main，可由 WMS_GOV_BASE 覆盖",
    )
    parser.add_argument(
        "--context",
        choices=["local", "pr", "main", "release", "runtime"],
        default=os.environ.get("WMS_GOV_CONTEXT"),
        help="执行场景；省略时保持默认，不按场景过滤",
    )
    parser.add_argument("--strict", action="store_true",
                        help="--strict 模式下，gate-rules.toml 引用的占位脚本视为失败（Wave 1+ 推荐启用）")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    rules = load_gate_rules()
    changed = get_changed_files(base_ref=args.base, include_untracked=True)

    # Tier 是成本预算，context 是执行场景；两者正交。
    rules_for_tier = rules_for_execution(
        rules,
        tier=args.tier,
        context=args.context,
    )
    triggered = match_rules(changed, rules_for_tier)
    specific_rules = [rule for rule in rules_for_tier if rule.match != "**"]
    specifically_matched = {
        path for path in changed if any(rule.matches(path) for rule in specific_rules)
    }
    unknown_files = [path for path in changed if path not in specifically_matched]
    machine_output = args.json

    if not machine_output:
        print(f"▶ task_check {args.tier} (base={args.base})")
        print(f"  · changed files: {len(changed)}")
        print(f"  · gate rules for {args.tier}: {len(rules_for_tier)}")
        print(f"  · triggered checks: {len(triggered)}")

    fallback_result: ScriptResult | None = None
    if changed and unknown_files:
        fallback_result = run_t1_fallback(machine_output)
        fallback_result.matched_files = len(unknown_files)
        if not specifically_matched:
            fallback = fallback_result
            payload = {
                "tier": args.tier,
                "context": args.context,
                "base": args.base,
                "changed": len(changed),
                "triggered": [asdict(fallback)],
                "note": "未匹配 diff 规则，已执行 T1 全量兜底",
                "ok": fallback.exit_code == 0,
            }
            if machine_output:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print("  · 未匹配 diff 规则，执行 T1 全量兜底")
                print(f"  {'✓' if fallback.exit_code == 0 else '✘'} governance_t1_fallback")
            return 0 if fallback.exit_code == 0 else 1
        if args.tier == "T1":
            triggered = match_rules(changed, specific_rules)

    if not triggered:
        if changed:
            fallback = run_t1_fallback(machine_output)
            fallback.matched_files = len(changed)
            payload = {
                "tier": args.tier,
                "context": args.context,
                "base": args.base,
                "changed": len(changed),
                "triggered": [asdict(fallback)],
                "note": "未匹配 diff 规则，已执行 T1 全量兜底",
                "ok": fallback.exit_code == 0,
            }
            if machine_output:
                print(json.dumps(payload, ensure_ascii=False, indent=2))
            else:
                print("  · 未匹配 diff 规则，执行 T1 全量兜底")
                print(f"  {'✓' if fallback.exit_code == 0 else '✘'} governance_t1_fallback")
            return 0 if fallback.exit_code == 0 else 1

        msg = "no changed files"
        if machine_output:
            print(json.dumps({
                "tier": args.tier,
                "context": args.context,
                "base": args.base,
                "changed": len(changed),
                "triggered": [],
                "note": msg,
                "ok": True,
            }, ensure_ascii=False, indent=2))
        else:
            print(f"  {msg}")
        return 0

    results: list[ScriptResult] = [fallback_result] if fallback_result else []
    if fallback_result and not machine_output:
        print(f"  · {len(unknown_files)} 个未知路径，追加 T1 全量兜底")
        print(f"  {'✓' if fallback_result.exit_code == 0 else '✘'} governance_t1_fallback")
    for check_name, files in triggered.items():
        if not machine_output:
            print(f"  · running {check_name}  (matched {len(files)} files)")
        r = run_one(check_name, json_mode=machine_output, strict_mode=args.strict)
        r.matched_files = len(files)
        r.rule_ids, r.sources, r.contexts = metadata_for_check(
            check_name,
            files,
            rules_for_tier,
        )
        if r.exit_code == -1:
            if args.strict:
                if not machine_output:
                    print(f"    ✘ script not implemented yet: {check_name} (--strict 模式下视为失败)")
                r.set_exit_code(2)
            else:
                if not machine_output:
                    print(f"    ⚠ script not implemented yet: {check_name} (placeholder, 加 --strict 视为失败)")
                r.set_exit_code(0)
        results.append(r)

    failed = [r for r in results if r.exit_code != 0]

    if machine_output:
        print(json.dumps({
            "tier": args.tier,
            "context": args.context,
            "base": args.base,
            "changed": len(changed),
            "triggered": [asdict(r) for r in results],
            "ok": not failed,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"\n▶ task_check summary: {len(results) - len(failed)}/{len(results)} ok")
        for r in results:
            mark = "✓" if r.exit_code == 0 else "✘"
            print(f"  {mark} {r.name:<35} files={r.matched_files} {r.duration_ms}ms")

    return 0 if not failed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
