#!/usr/bin/env python3
"""check_component_doc_header.py — 业务复合组件文档头规范

类别：6. 原型治理
Tier：T1（< 10s）
输入：packages/ui/src/business/**/*.tsx + prototypes/src/pages/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项（对照 docs/frontend-coding-standards.md §5）：
- 顶部必须含文档头（/** ... */ 块）
- 5 项强制字段：一句话用途 / 层级 / 关联故事 / Wave / @example
- 例外：@governance: skip-doc-header（豁免标签）

不覆盖（依赖人工 review）：
- 关联故事 ID 是否真实存在
- 业务约束的真实性
- @example 的可运行性
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
PROTO_DIR = REPO_ROOT / "prototypes" / "src"
BUSINESS_DIR = REPO_ROOT / "packages" / "ui" / "src" / "business"
PAGES_DIR = PROTO_DIR / "pages"

DOC_BLOCK_RE = re.compile(r"^\s*/\*\*(.*?)\*/", re.DOTALL | re.MULTILINE)
SKIP_TAG = "@governance: skip-doc-header"

REQUIRED_FIELDS = [
    ("用途（首行 — 描述）", re.compile(r"\*\s+\S+\s+—\s+\S+", re.MULTILINE)),
    ("层级", re.compile(r"\*\s*层级[:：]", re.MULTILINE)),
    ("关联故事", re.compile(r"\*\s*关联故事[:：]", re.MULTILINE)),
    ("Wave", re.compile(r"\*\s*Wave[:：]", re.MULTILINE)),
    ("@example", re.compile(r"\*\s*@example", re.MULTILINE)),
]


def _component_files() -> list[Path]:
    files: list[Path] = []
    if BUSINESS_DIR.exists():
        for p in BUSINESS_DIR.rglob("*.tsx"):
            # 跳过 stories / spec
            name = p.name
            if ".stories." in name or ".spec." in name or ".test." in name:
                continue
            # 主文件 = 跟所在目录同名；index.ts 跳过；其他子组件文件也校验
            files.append(p)
    if PAGES_DIR.exists():
        for p in PAGES_DIR.rglob("*.tsx"):
            name = p.name
            if ".stories." in name or ".spec." in name or ".test." in name:
                continue
            files.append(p)
    return sorted(files)


def _check_file(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    if SKIP_TAG in text[:500]:
        return []  # 显式豁免
    # 全文搜索文档块（pages 文件可能有 MOCK 数据前置，文档头在后）
    blocks = list(DOC_BLOCK_RE.finditer(text))
    if not blocks:
        return [f"缺少 /** ... */ 文档块"]
    # 检查所有文档块，至少一个满足全部 5 项字段
    for m in blocks:
        block = m.group(0)
        if all(p.search(block) for _, p in REQUIRED_FIELDS):
            return []  # 找到合格的文档块
    # 没找到 → 报告第一个块缺哪些字段
    block = blocks[0].group(0)
    missing = [name for name, p in REQUIRED_FIELDS if not p.search(block)]
    return [f"文档头缺失字段: {', '.join(missing)}"]


def run() -> list[str]:
    errors: list[str] = []
    files = _component_files()
    for f in files:
        for issue in _check_file(f):
            rel = f.relative_to(REPO_ROOT).as_posix()
            errors.append(f"{rel}: {issue}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({"status": "fail" if errors else "pass", "errors": errors}))
    else:
        if errors:
            print(f"✗ check_component_doc_header: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_component_doc_header: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
