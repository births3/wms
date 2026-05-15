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
        "validate_doc_layers.py",
        "check_commit_convention.py",
    ],
    "T2": [
        # T1 + diff 驱动（由 task_check.py 处理）
        # 这里只追加 T2 专属脚本，未来 Wave 2+ 加入
    ],
    "T3": [
        # 未来 Wave 3+：handler test coverage、idempotency、permission matrix 等
    ],
    "T4": [
        # 未来 Wave 4+：perf baseline、observability、concurrency、API compat 等
    ],
}


@dataclass
class ScriptResult:
    name: str
    exit_code: int
    duration_ms: int


def run_script(name: str, *, json_mode: bool) -> ScriptResult:
    import time

    cmd = [sys.executable, str(SCRIPTS_DIR / name)]
    if json_mode:
        cmd.append("--json")
    start = time.perf_counter()
    p = subprocess.run(cmd, check=False)
    dur = int((time.perf_counter() - start) * 1000)
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
