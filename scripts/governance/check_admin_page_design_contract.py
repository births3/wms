#!/usr/bin/env python3
"""check_admin_page_design_contract.py — 管理端页面设计契约检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin/src/pages/**/*Page.tsx
输出：人类可读 + --json
退出码：
  0  管理端页面没有明显违反动作弹窗和常驻详情规则
  1  发现列表页常驻详情/轨迹/动作面板，或私有动作缺少弹窗承载
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PAGES_DIR = REPO_ROOT / "apps" / "web-admin" / "src" / "pages"
AST_SCRIPT = Path(__file__).resolve().with_name("admin_page_design_contract_ast.cjs")
SKIP_TAG = "@governance: skip-admin-page-design-contract"

RESIDENT_PANEL_PATTERNS = [
    (re.compile(r"\b(?:Waybill|Tracking|Audit|Detail|Status)Panel\b"), "列表页疑似常驻详情/轨迹/审计/状态面板"),
    (re.compile(r"运单与轨迹|轨迹缓存|最近生成的运单"), "列表页疑似常驻运单轨迹区域"),
    (re.compile(r"当前处理单|本环节操作"), "列表页疑似常驻当前处理对象或动作区"),
]

ACTION_DIALOG_RULES = [
    (re.compile(r'key:\s*["\']tracking["\']|label:\s*["\']轨迹["\']'), re.compile(r"Tracking\w*Dialog|trackingDialog|轨迹.*Dialog"), "轨迹动作必须打开轨迹弹窗"),
    (re.compile(r'key:\s*["\']cancel["\']|label:\s*["\']取消["\']'), re.compile(r"Cancel\w*Dialog|cancelDialog|取消.*Dialog"), "取消动作必须打开取消确认弹窗"),
    (re.compile(r'key:\s*["\']resend["\']|label:\s*["\']重发["\']'), re.compile(r"Resend\w*Dialog|resendDialog|AlertDialog|\bConfirm(?:Dialog)?\b|window\.confirm"), "重发动作必须打开重发确认弹窗"),
    (re.compile(r"\b(?:DataGridDisableAction|disableAction)\b"), re.compile(r"Disable\w*Dialog|disableDialog|AlertDialog|\bConfirm(?:Dialog)?\b|window\.confirm"), "启停动作必须打开启停确认弹窗"),
]

BROWSER_ACTION_RULES = [
    (re.compile(r"window\.prompt\("), "禁止用 window.prompt 承载管理端新增/修改表单"),
    (re.compile(r"window\.print\("), "打印动作必须打开打印预览/确认弹窗或写明豁免"),
]


@dataclass(frozen=True)
class Issue:
    file: str
    kind: str
    message: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def page_files() -> list[Path]:
    if not PAGES_DIR.exists():
        return []
    return sorted(PAGES_DIR.rglob("*Page.tsx"))


def is_list_page(text: str) -> bool:
    return "<DataGrid" in text or "DataGridColumn" in text


def should_scan(path: Path, text: str) -> bool:
    return path.name != "LoginPage.tsx" and SKIP_TAG not in text


def scan_file(path: Path) -> list[Issue]:
    text = path.read_text(encoding="utf-8")
    if not should_scan(path, text):
        return []

    issues: list[Issue] = []
    if is_list_page(text):
        for pattern, message in RESIDENT_PANEL_PATTERNS:
            if pattern.search(text):
                issues.append(Issue(rel(path), "resident_detail_panel", message))

    for action_pattern, dialog_pattern, message in ACTION_DIALOG_RULES:
        if action_pattern.search(text) and not dialog_pattern.search(text):
            issues.append(Issue(rel(path), "action_without_dialog", message))

    for pattern, message in BROWSER_ACTION_RULES:
        if pattern.search(text):
            issues.append(Issue(rel(path), "browser_action_without_dialog", message))

    return issues


def ast_scan(paths: list[Path]) -> list[Issue]:
    if not paths:
        return []
    result = subprocess.run(
        ["node", str(AST_SCRIPT), "--repo-root", str(REPO_ROOT), *(str(path) for path in paths)],
        cwd=REPO_ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=10,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "TypeScript AST scan failed")
    payload = json.loads(result.stdout)
    return [Issue(issue["file"], issue["kind"], issue["message"]) for issue in payload.get("issues", [])]


def dedupe(issues: list[Issue]) -> list[Issue]:
    seen: set[tuple[str, str, str]] = set()
    unique: list[Issue] = []
    for issue in issues:
        key = (issue.file, issue.kind, issue.message)
        if key in seen:
            continue
        seen.add(key)
        unique.append(issue)
    return unique


def scan() -> list[Issue]:
    paths = page_files()
    scannable_paths = [
        path for path in paths
        if should_scan(path, path.read_text(encoding="utf-8"))
    ]
    issues: list[Issue] = []
    for path in scannable_paths:
        issues.extend(scan_file(path))
    issues.extend(ast_scan(scannable_paths))
    return dedupe(issues)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    issues = scan()
    payload = {
        "check": "check_admin_page_design_contract",
        "tier": "T1",
        "category": "前端治理",
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_admin_page_design_contract (T1, 前端治理)")
        if issues:
            print(f"  ✘ {len(issues)} 个管理端页面设计契约问题:")
            for issue in issues:
                print(f"    - {issue.file}: [{issue.kind}] {issue.message}")
        else:
            print("  ✓ 管理端页面未发现常驻详情/轨迹/动作面板违规")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
