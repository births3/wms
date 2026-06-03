#!/usr/bin/env python3
"""check_openapi_in_sync.py — 验证后端 utoipa 与仓库内 openapi.json 同步

类别：5. 接口契约治理（Wave 1+ 启用）
Tier：T2（< 120s；含 cargo run 编译耗时）
输入：
  backend/ (Cargo workspace) + shared/openapi/openapi.json + packages/api-client/src/schema.ts
输出：人类可读
退出码：0 通过 / 1 不同步 / 2 脚本错误

机制：
  1. 跑 cargo run --bin openapi-export 生成临时 openapi.json
  2. 跟仓库内 openapi.json diff（jq normalize 后再比较，避免缩进/排序噪声）
  3. 跑 openapi-typescript 生成临时 schema.ts，并与仓库内生成物比较
  4. 不同步 → 报告并退出 1，提示开发者跑 `just openapi-sync`

不覆盖：
  - 实际服务运行时的 OpenAPI 暴露（应由 axum router 同源生成）

注：spike-003 已在 Wave 0.5 验证；Wave 1 W1.C 起本脚本只校验主仓路径。
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

BACKEND_DIR = REPO_ROOT / "backend"
SHARED_OPENAPI = REPO_ROOT / "shared" / "openapi" / "openapi.json"
API_CLIENT_SCHEMA = REPO_ROOT / "packages" / "api-client" / "src" / "schema.ts"
EXPORT_BIN = "openapi-export"
CARGO_EXPORT_TIMEOUT_SECONDS = 90
SCHEMA_EXPORT_TIMEOUT_SECONDS = 20


def _normalize(data: dict) -> str:
    """jq-style normalize：稳定排序 + 缩进，规避 commit 噪声"""
    return json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True)


def _emit_json(status: str, *, ok: bool, **extra: object) -> None:
    payload = {
        "check": "check_openapi_in_sync",
        "tier": "T2",
        "category": "接口契约治理",
        "status": status,
        "ok": ok,
        **extra,
    }
    print(json.dumps(payload, ensure_ascii=False, indent=2))


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--strict", action="store_true", help="严格模式（依赖缺失或超时即失败；T2/CI 必须启用）")
    args = parser.parse_args(argv)

    if not BACKEND_DIR.exists():
        if args.json:
            _emit_json("missing_backend", ok=not args.strict, backend_dir=str(BACKEND_DIR.relative_to(REPO_ROOT)))
        else:
            print(f"⚠ {BACKEND_DIR.relative_to(REPO_ROOT)} 不存在", file=sys.stderr)
        return 1 if args.strict else 0

    if not SHARED_OPENAPI.exists():
        if args.json:
            _emit_json("missing_openapi", ok=False, openapi_path=str(SHARED_OPENAPI.relative_to(REPO_ROOT)))
        else:
            print(f"✘ {SHARED_OPENAPI.relative_to(REPO_ROOT)} 不存在；请跑 just openapi-sync", file=sys.stderr)
        return 1

    if not API_CLIENT_SCHEMA.exists():
        if args.json:
            _emit_json("missing_schema_ts", ok=False, schema_path=str(API_CLIENT_SCHEMA.relative_to(REPO_ROOT)))
        else:
            print(f"✘ {API_CLIENT_SCHEMA.relative_to(REPO_ROOT)} 不存在；请跑 just openapi-sync", file=sys.stderr)
        return 1

    # 跑 cargo run --bin openapi-export
    try:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--bin", EXPORT_BIN],
            cwd=BACKEND_DIR,
            capture_output=True,
            text=True,
            timeout=CARGO_EXPORT_TIMEOUT_SECONDS,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        if args.json:
            _emit_json("cargo_unavailable", ok=not args.strict, error=str(e))
        else:
            print(f"⚠ cargo 不可用或超时：{e}", file=sys.stderr)
        return 0 if not args.strict else 2

    if result.returncode != 0:
        if args.json:
            _emit_json("export_failed", ok=False, stderr=result.stderr)
        else:
            print(f"✘ cargo run --bin {EXPORT_BIN} 失败：\n{result.stderr}", file=sys.stderr)
        return 1

    # 解析两边的 JSON 并 normalize 后比较
    try:
        live = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        if args.json:
            _emit_json("invalid_export_json", ok=False, error=str(e))
        else:
            print(f"✘ openapi-export 输出非合法 JSON：{e}", file=sys.stderr)
        return 1

    try:
        with SHARED_OPENAPI.open() as f:
            committed = json.load(f)
    except json.JSONDecodeError as e:
        if args.json:
            _emit_json("invalid_committed_json", ok=False, error=str(e))
        else:
            print(f"✘ {SHARED_OPENAPI.relative_to(REPO_ROOT)} 非合法 JSON：{e}", file=sys.stderr)
        return 1

    live_str = _normalize(live)
    committed_str = _normalize(committed)

    if live_str != committed_str:
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
            _emit_json(
                "openapi_out_of_sync",
                ok=False,
                openapi_path=str(SHARED_OPENAPI.relative_to(REPO_ROOT)),
                diff_preview="\n".join(diff_lines[:30]),
            )
        else:
            print(f"✘ openapi 不同步！代码改了但 openapi.json 没重生。", file=sys.stderr)
            print(f"  路径: {SHARED_OPENAPI.relative_to(REPO_ROOT)}", file=sys.stderr)
            print("\n  diff 前 30 行：", file=sys.stderr)
            for line in diff_lines[:30]:
                print(f"  {line}", file=sys.stderr)
            print(f"\n  → 请跑：just openapi-sync", file=sys.stderr)
            print("    然后 git add 提交。", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as tmpdir:
        generated_schema = Path(tmpdir) / "schema.ts"
        try:
            schema_result = subprocess.run(
                [
                    "pnpm",
                    "--filter",
                    "@wms/api-client",
                    "exec",
                    "openapi-typescript",
                    "../../shared/openapi/openapi.json",
                    "--output",
                    str(generated_schema),
                ],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                timeout=SCHEMA_EXPORT_TIMEOUT_SECONDS,
            )
        except (FileNotFoundError, subprocess.TimeoutExpired) as e:
            if args.json:
                _emit_json("schema_generator_unavailable", ok=not args.strict, error=str(e))
            else:
                print(f"⚠ openapi-typescript 不可用或超时：{e}", file=sys.stderr)
            return 0 if not args.strict else 2

        if schema_result.returncode != 0:
            if args.json:
                _emit_json("schema_export_failed", ok=False, stderr=schema_result.stderr)
            else:
                print(f"✘ openapi-typescript 生成 schema.ts 失败：\n{schema_result.stderr}", file=sys.stderr)
            return 1

        generated_schema_text = _read_text(generated_schema)
        committed_schema_text = _read_text(API_CLIENT_SCHEMA)

    if generated_schema_text != committed_schema_text:
        import difflib
        diff_lines = list(
            difflib.unified_diff(
                committed_schema_text.splitlines(),
                generated_schema_text.splitlines(),
                fromfile=f"committed: {API_CLIENT_SCHEMA.relative_to(REPO_ROOT)}",
                tofile="generated: openapi-typescript",
                lineterm="",
                n=2,
            )
        )

        if args.json:
            _emit_json(
                "schema_ts_out_of_sync",
                ok=False,
                schema_path=str(API_CLIENT_SCHEMA.relative_to(REPO_ROOT)),
                diff_preview="\n".join(diff_lines[:30]),
            )
        else:
            print("✘ api-client schema.ts 不同步！openapi.json 改了但前端类型没重生。", file=sys.stderr)
            print(f"  路径: {API_CLIENT_SCHEMA.relative_to(REPO_ROOT)}", file=sys.stderr)
            print("\n  diff 前 30 行：", file=sys.stderr)
            for line in diff_lines[:30]:
                print(f"  {line}", file=sys.stderr)
            print("\n  → 请跑：just openapi-sync", file=sys.stderr)
            print("    然后 git add 提交。", file=sys.stderr)
        return 1

    if args.json:
        _emit_json(
            "ok",
            ok=True,
            openapi_path=str(SHARED_OPENAPI.relative_to(REPO_ROOT)),
            schema_path=str(API_CLIENT_SCHEMA.relative_to(REPO_ROOT)),
        )
    else:
        print(f"✓ openapi 同步：{SHARED_OPENAPI.relative_to(REPO_ROOT)}")
        print(f"✓ api-client schema 同步：{API_CLIENT_SCHEMA.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
