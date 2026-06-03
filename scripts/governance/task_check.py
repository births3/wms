#!/usr/bin/env python3
"""task_check.py — diff 触发的最小检查集调度（骨架）

类别：4. 流程治理
Tier：作为入口，根据 git diff 与 governance/gate-rules.toml 决定跑哪些脚本

详细规则：见 docs/adr/0003-governance-model.md §机制 4

用法：
  python3 scripts/governance/task_check.py --tier T2
  python3 scripts/governance/task_check.py --tier T2 --base main

退出码：
  0  通过
  1  有脚本失败
  2  脚本自身错误

第 0 周阶段：
- gate-rules.toml 是骨架（仅几条示例规则）
- 大部分 check_* 脚本尚未实现，会跳过并提示
- 只有依赖图引用的脚本才实际跑

模式：
- 默认（非 --strict）：未实现的脚本仅 print warning，exit_code=0（Wave 0 阶段需要）
- --strict：未实现的脚本视为失败（Wave 1+ 进入 CI 时启用，强制脚本补齐）
- 启用时机：进入 Wave 1 后在 lefthook pre-push / CI 中加 --strict 标志
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
REPO_ROOT = SCRIPTS_DIR.parent.parent

# 复用公共库
sys.path.insert(0, str(SCRIPTS_DIR))
from _diff import get_changed_files, load_gate_rules, match_rules  # noqa: E402


@dataclass
class ScriptResult:
    name: str
    matched_files: int
    exit_code: int
    duration_ms: int


# task_check --strict 时，这些脚本自身也必须以严格语义执行。
STRICT_SCRIPT_ARGS = {
    "check_openapi_in_sync": ["--strict"],
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
    p = subprocess.run(cmd, check=False)
    dur = int((time.perf_counter() - start) * 1000)
    return ScriptResult(
        name=check_name, matched_files=0, exit_code=p.returncode, duration_ms=dur
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tier", default="T2", choices=["T1", "T2", "T3", "T4"])
    parser.add_argument("--base", default="main", help="diff base ref，默认 main")
    parser.add_argument("--strict", action="store_true",
                        help="--strict 模式下，gate-rules.toml 引用的占位脚本视为失败（Wave 1+ 推荐启用）")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--report-json", action="store_true")
    args = parser.parse_args(argv)

    rules = load_gate_rules()
    changed = get_changed_files(base_ref=args.base, include_untracked=True)

    # 按 Tier 过滤规则
    rules_for_tier = [r for r in rules if r.tier == args.tier or r.tier == "any"]
    triggered = match_rules(changed, rules_for_tier)

    print(f"▶ task_check {args.tier} (base={args.base})")
    print(f"  · changed files: {len(changed)}")
    print(f"  · gate rules for {args.tier}: {len(rules_for_tier)}")
    print(f"  · triggered checks: {len(triggered)}")

    if not triggered:
        msg = "no diff-triggered checks (Wave 0 骨架阶段属正常)"
        if args.report_json:
            print(json.dumps({
                "tier": args.tier,
                "base": args.base,
                "changed": len(changed),
                "triggered": [],
                "note": msg,
                "ok": True,
            }, ensure_ascii=False, indent=2))
        else:
            print(f"  {msg}")
        return 0

    results: list[ScriptResult] = []
    for check_name, files in triggered.items():
        print(f"  · running {check_name}  (matched {len(files)} files)")
        r = run_one(check_name, json_mode=args.json, strict_mode=args.strict)
        r.matched_files = len(files)
        if r.exit_code == -1:
            # 脚本未实现：根据 --strict 决定是阻塞还是降级
            if args.strict:
                print(f"    ✘ script not implemented yet: {check_name} (--strict 模式下视为失败)")
                r.exit_code = 2
            else:
                print(f"    ⚠ script not implemented yet: {check_name} (placeholder, 加 --strict 视为失败)")
                r.exit_code = 0  # 默认降级（不阻塞 Wave 演进）
        results.append(r)

    failed = [r for r in results if r.exit_code != 0]

    if args.report_json:
        print(json.dumps({
            "tier": args.tier,
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
