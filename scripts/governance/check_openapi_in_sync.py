#!/usr/bin/env python3
"""check_openapi_in_sync.py — 验证后端 utoipa 与仓库内 openapi.json 同步

类别：5. 接口契约治理（Wave 1+ 启用）
Tier：T2（< 1min；含 cargo run 编译耗时）
输入：
  spikes/spike-003-utoipa-openapi-ts-pipeline/  (本 spike 范围)
  Wave 1+ 改为：backend/ (Cargo workspace) + shared/openapi/openapi.json
输出：人类可读
退出码：0 通过 / 1 不同步 / 2 脚本错误

机制：
  1. 跑 cargo run --bin openapi-export 生成临时 openapi.json
  2. 跟仓库内 openapi.json diff（jq normalize 后再比较，避免缩进/排序噪声）
  3. 不同步 → 报告并退出 1，提示开发者跑 `just openapi-sync`

不覆盖：
  - openapi.json → schema.ts 的同步（应在前端 CI 跑 openapi-typescript --check）
  - 实际服务运行时的 OpenAPI 暴露（应由 axum router 同源生成）

注：本脚本目前仅验证 spike-003 demo。Wave 1 W1.C 启动后改路径为
backend/ 主项目，加入 T2 治理集（gate-rules.toml match shared/openapi/**
+ backend/crates/api/**）。
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

# spike-003 默认配置（Wave 1 启动时改 BACKEND_DIR / SHARED_OPENAPI）
BACKEND_DIR = REPO_ROOT / "spikes" / "spike-003-utoipa-openapi-ts-pipeline"
SHARED_OPENAPI = BACKEND_DIR / "shared" / "openapi.json"
EXPORT_BIN = "openapi-export"


def _normalize(data: dict) -> str:
    """jq-style normalize：稳定排序 + 缩进，规避 commit 噪声"""
    return json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="严格模式（缺 cargo 即失败）")
    args = parser.parse_args()

    if not SHARED_OPENAPI.exists():
        print(f"⚠ {SHARED_OPENAPI.relative_to(REPO_ROOT)} 不存在；spike-003 范围未生成", file=sys.stderr)
        return 0 if not args.strict else 1

    # 跑 cargo run --bin openapi-export
    try:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--bin", EXPORT_BIN],
            cwd=BACKEND_DIR,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        print(f"⚠ cargo 不可用或超时：{e}", file=sys.stderr)
        return 0 if not args.strict else 2

    if result.returncode != 0:
        print(f"✘ cargo run --bin {EXPORT_BIN} 失败：\n{result.stderr}", file=sys.stderr)
        return 1

    # 解析两边的 JSON 并 normalize 后比较
    try:
        live = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        print(f"✘ openapi-export 输出非合法 JSON：{e}", file=sys.stderr)
        return 1

    with SHARED_OPENAPI.open() as f:
        committed = json.load(f)

    live_str = _normalize(live)
    committed_str = _normalize(committed)

    if live_str == committed_str:
        if args.json:
            print(json.dumps({"status": "ok", "openapi_path": str(SHARED_OPENAPI.relative_to(REPO_ROOT))}))
        else:
            print(f"✓ openapi 同步：{SHARED_OPENAPI.relative_to(REPO_ROOT)}")
        return 0

    # 不同步 — 用 difflib 给出可读 diff（仅前 30 行）
    import difflib
    diff_lines = list(
        difflib.unified_diff(
            committed_str.splitlines(),
            live_str.splitlines(),
            fromfile=f"committed: {SHARED_OPENAPI.relative_to(REPO_ROOT)}",
            tofile="live: cargo run --bin openapi-export",
            lineterm="",
            n=2,
        )
    )

    if args.json:
        print(json.dumps({
            "status": "out_of_sync",
            "openapi_path": str(SHARED_OPENAPI.relative_to(REPO_ROOT)),
            "diff_preview": "\n".join(diff_lines[:30]),
        }))
    else:
        print(f"✘ openapi 不同步！代码改了但 openapi.json 没重生。", file=sys.stderr)
        print(f"  路径: {SHARED_OPENAPI.relative_to(REPO_ROOT)}", file=sys.stderr)
        print("\n  diff 前 30 行：", file=sys.stderr)
        for line in diff_lines[:30]:
            print(f"  {line}", file=sys.stderr)
        print(f"\n  → 请跑：cd {BACKEND_DIR.relative_to(REPO_ROOT)} && cargo run --bin {EXPORT_BIN} > shared/openapi.json", file=sys.stderr)
        print("    然后 git add 提交。", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
