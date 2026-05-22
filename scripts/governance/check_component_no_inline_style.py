#!/usr/bin/env python3
"""check_component_no_inline_style.py — 业务复合组件禁止静态 inline style

类别：6. 原型治理
Tier：T1（< 10s）
输入：prototypes/src/components/business/**/*.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项（对照 docs/frontend-coding-standards.md §4.3）：
- 业务复合组件不允许 style={{ ... }} 静态对象
- 例外：紧邻上一行有 // 动态：xxx 注释 → 豁免（动态计算）

只检查 components/business/，pages/ 不检查（页面级允许更灵活）。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
BUSINESS_DIR = REPO_ROOT / "prototypes" / "src" / "components" / "business"

# 匹配 style={{ ... 或 style={  （JSX 内联 style 对象）
INLINE_STYLE_RE = re.compile(r"\bstyle\s*=\s*\{\s*\{")
DYNAMIC_COMMENT_RE = re.compile(r"//\s*动态[:：]")


def _is_dynamic_style(body: str) -> bool:
    """判断 style 对象内容是否含动态值"""
    if "${" in body or "var(" in body:
        return True
    # 拆分顶层 , 分隔的 key:value 对
    pairs = re.split(r",(?![^{(]*[})])", body)
    for p in pairs:
        if ":" not in p:
            continue
        _, value = p.split(":", 1)
        v = value.strip().rstrip(",").strip()
        if not v:
            continue
        # 字面量：数字、字符串、true/false/null
        if re.match(r'^("[^"]*"|\'[^\']*\'|`[^`]*`|\d+(?:\.\d+)?(?:px|rem|em|%|s|ms|fr)?|true|false|null)$', v):
            continue
        # 标识符 / 表达式 → 动态
        return True
    return False


def _extract_style_blocks(text: str) -> list[tuple[int, str]]:
    """提取所有 style={{ ... }} 块，返回 [(line_no, body)]"""
    blocks: list[tuple[int, str]] = []
    i = 0
    n = len(text)
    while i < n:
        m = re.search(r"\bstyle\s*=\s*\{\{", text[i:])
        if not m:
            break
        start = i + m.end()
        # 计算行号
        line_no = text.count("\n", 0, i + m.start()) + 1
        # 找匹配的 }}
        depth = 2  # 已经吃掉 {{
        j = start
        while j < n and depth > 0:
            c = text[j]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            j += 1
        body = text[start:j - 2]  # 去掉末尾 }}
        blocks.append((line_no, body))
        i = j
    return blocks


def _check_file(path: Path) -> list[str]:
    issues: list[str] = []
    if not path.exists():
        return issues
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    for line_no, body in _extract_style_blocks(text):
        if _is_dynamic_style(body):
            continue
        # 检查紧邻上一行注释豁免
        prev = lines[line_no - 2] if line_no >= 2 else ""
        same = lines[line_no - 1] if line_no >= 1 else ""
        if DYNAMIC_COMMENT_RE.search(prev) or DYNAMIC_COMMENT_RE.search(same):
            continue
        issues.append(f"L{line_no}: 静态 inline style（须改为 className 或加 '// 动态：理由' 注释豁免）")
    return issues


def run() -> list[str]:
    errors: list[str] = []
    if not BUSINESS_DIR.exists():
        return errors
    for f in sorted(BUSINESS_DIR.rglob("*.tsx")):
        if ".stories." in f.name or ".spec." in f.name or ".test." in f.name:
            continue
        for issue in _check_file(f):
            rel = f.relative_to(REPO_ROOT).as_posix()
            errors.append(f"{rel}:{issue}")
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
            print(f"✗ check_component_no_inline_style: {len(errors)} 项违规")
            for e in errors:
                print(f"  - {e}")
        else:
            print("✓ check_component_no_inline_style: 通过")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
