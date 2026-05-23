#!/usr/bin/env python3
"""check_file_naming.py — 文件命名规范校验

类别：2. 代码治理
Tier：T1（< 10s）
输入：git diff 变更文件（默认）或全量扫描（--all）
输出：人类可读 + --json
退出码：
  0  通过
  1  发现命名违规
  2  脚本自身错误

规则来源：docs/coding-standards.md §零 文件命名速查表
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
sys.path.insert(0, str(_THIS.parent))
from _diff import get_changed_files  # noqa: E402


@dataclass
class Violation:
    file: str
    rule: str
    message: str


# ============================================================
# 命名规则定义
# ============================================================

# snake_case: 小写字母 + 数字 + 下划线
SNAKE_RE = re.compile(r"^[a-z][a-z0-9_]*$")
# kebab-case: 小写字母 + 数字 + 短横线
KEBAB_RE = re.compile(r"^[a-z][a-z0-9\-]*$")
# PascalCase: 大写开头，字母数字
PASCAL_RE = re.compile(r"^[A-Z][a-zA-Z0-9]*$")
# 迁移文件: NNNN_<desc>.sql 或 NNNN_<desc>.down.sql 或 NNNNNNNNNNNNNN_<desc>.sql（sqlx 标准）
# - 简短数字：4-6 位（refinery 风格）
# - 完整 timestamp：14 位 YYYYMMDDHHMMSS（sqlx-cli 默认；ADR-0001 §D 锁定）
MIGRATION_RE = re.compile(r"^\d{4,14}_[a-z][a-z0-9_]*\.(down\.)?sql$")
# ADR: NNNN-<slug>.md
ADR_RE = re.compile(r"^\d{4}-[a-z0-9][a-z0-9\-]*\.md$")
# Retro: wave-N-retro.md 或 wave-N.M-retro.md（如 wave-0.5-retro.md）
RETRO_RE = re.compile(r"^wave-\d+(?:\.\d+)?-retro\.md$")
# 合规文档: gsp-<topic>.md
COMPLIANCE_RE = re.compile(r"^gsp-[a-z0-9][a-z0-9\-]*\.md$")
# 治理脚本: snake_case.py（公共库 _前缀）
GOV_SCRIPT_RE = re.compile(r"^_?[a-z][a-z0-9_]*\.py$")
# TS 测试: *.spec.ts / *.spec.tsx
TS_TEST_RE = re.compile(r"^.+\.(spec|test|stories)\.tsx?$")

# 忽略的文件/目录
IGNORE_NAMES = {
    ".gitkeep", ".gitignore", ".gitattributes", ".editorconfig",
    ".env", ".env.example", "justfile", "lefthook.yml",
    "Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml",
    "pnpm-workspace.yaml", "tsconfig.json", "vite.config.ts",
    "index.html", "index.ts", "index.tsx", "main.rs", "lib.rs",
    "mod.rs", "main.tsx", "App.tsx",
}

# 根目录固定大写文档
ROOT_DOCS = {"README.md", "ROADMAP.md", "TODO.md", "CHANGELOG.md", "AGENTS.md"}


def check_file(rel_path: str) -> Violation | None:
    """检查单个文件路径是否符合命名规范。"""
    parts = rel_path.split("/")
    filename = parts[-1]

    # 跳过忽略列表
    if filename in IGNORE_NAMES or filename.startswith("."):
        return None

    # 根目录固定名文档
    if len(parts) == 1 and filename in ROOT_DOCS:
        return None

    # --- Rust 源文件 ---
    if filename.endswith(".rs"):
        stem = filename[:-3]
        if not SNAKE_RE.match(stem):
            return Violation(rel_path, "rust-file-snake", f"Rust file must be snake_case: '{filename}'")
        return None

    # --- Rust crate 目录（backend/crates/<name>/） ---
    if len(parts) >= 3 and parts[0] == "backend" and parts[1] == "crates":
        crate_dir = parts[2]
        if not KEBAB_RE.match(crate_dir):
            return Violation(rel_path, "crate-dir-kebab", f"Crate directory must be kebab-case: '{crate_dir}'")
        # crate 内子目录应 snake_case
        for p in parts[3:]:
            if "." in p:
                break  # 到文件名了
            if not SNAKE_RE.match(p) and p not in ("src", "tests", "benches"):
                return Violation(rel_path, "rust-dir-snake", f"Rust module directory must be snake_case: '{p}'")
        return None

    # --- SQL 迁移 ---
    if "migrations" in parts and filename.endswith(".sql"):
        if not MIGRATION_RE.match(filename):
            return Violation(rel_path, "migration-naming", f"Migration must be NNNN_<desc>.sql or NNNNNNNNNNNNNN_<desc>.sql (sqlx 14-digit timestamp): '{filename}'")
        return None

    # --- 治理脚本 ---
    if "scripts/governance" in rel_path and filename.endswith(".py"):
        if not GOV_SCRIPT_RE.match(filename):
            return Violation(rel_path, "gov-script-snake", f"Governance script must be snake_case.py: '{filename}'")
        return None

    # --- ADR ---
    if "docs/adr" in rel_path and filename.endswith(".md") and filename != "README.md":
        if not ADR_RE.match(filename):
            return Violation(rel_path, "adr-naming", f"ADR must be NNNN-<slug>.md: '{filename}'")
        return None

    # --- Retro ---
    if "docs/retros" in rel_path and filename.endswith(".md") and filename != "README.md":
        if not RETRO_RE.match(filename):
            return Violation(rel_path, "retro-naming", f"Retro must be wave-N-retro.md or wave-N.M-retro.md: '{filename}'")
        return None

    # --- 合规文档 ---
    if "docs/compliance" in rel_path and filename.endswith(".md") and filename not in (".gitkeep", "README.md"):
        if not COMPLIANCE_RE.match(filename):
            return Violation(rel_path, "compliance-naming", f"Compliance doc must be gsp-<topic>.md or README.md: '{filename}'")
        return None

    # --- 领域文档 ---
    if "docs/domain" in rel_path and filename.endswith(".md") and filename != ".gitkeep":
        stem = filename[:-3]
        if not KEBAB_RE.match(stem):
            return Violation(rel_path, "domain-doc-kebab", f"Domain doc must be <context>.md (kebab-case): '{filename}'")
        return None

    # --- TS 组件文件 (.tsx) ---
    if filename.endswith(".tsx"):
        stem = filename[:-4]
        # 测试文件特殊处理
        if TS_TEST_RE.match(filename):
            return None
        # shadcn/ui 原子组件用 kebab-case（典型路径：components/ui/ 或 packages/ui/src/ui/）
        # 这是 shadcn CLI 标准，跟 React 生态对齐
        if "/components/ui/" in rel_path or rel_path.startswith("packages/ui/src/ui/"):
            if not KEBAB_RE.match(stem):
                return Violation(rel_path, "ui-kebab", f"shadcn UI atom file must be kebab-case: '{filename}'")
            return None
        if not PASCAL_RE.match(stem):
            return Violation(rel_path, "tsx-pascal", f"TSX component file must be PascalCase: '{filename}'")
        return None

    # --- TS 非组件文件 (.ts) ---
    if filename.endswith(".ts"):
        stem = filename[:-3]
        # 测试文件
        if TS_TEST_RE.match(filename):
            return None
        # 配置文件（vite.config.ts 等）已在 IGNORE_NAMES
        if not KEBAB_RE.match(stem):
            return Violation(rel_path, "ts-kebab", f"TS non-component file must be kebab-case: '{filename}'")
        return None

    # --- TS/前端目录 ---
    if any(p in parts for p in ("apps", "packages")):
        for p in parts:
            if "." in p or p in ("apps", "packages", "src", "public", "node_modules", "__tests__"):
                continue
            if not KEBAB_RE.match(p):
                return Violation(rel_path, "frontend-dir-kebab", f"Frontend directory must be kebab-case: '{p}'")
        return None

    return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--all", action="store_true", help="扫描全部跟踪文件（而非仅 diff）")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    if args.all:
        import subprocess
        p = subprocess.run(
            ["git", "ls-files"], cwd=REPO_ROOT,
            capture_output=True, text=True, check=False,
        )
        files = [l.strip() for l in p.stdout.splitlines() if l.strip()]
    else:
        files = get_changed_files(base_ref="main", include_untracked=True)

    violations: list[Violation] = []
    for f in files:
        v = check_file(f)
        if v:
            violations.append(v)

    if args.json:
        payload = {
            "check": "check_file_naming",
            "tier": "T1",
            "category": "代码治理",
            "scanned": len(files),
            "violations": [asdict(v) for v in violations],
            "ok": not violations,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_file_naming (T1, 代码治理) — scanned {len(files)} files")
        if not violations:
            print("  ✓ all file names comply")
        else:
            print(f"  ✘ {len(violations)} violation(s):")
            for v in violations:
                print(f"    [{v.rule}] {v.file}: {v.message}")

    return 0 if not violations else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
