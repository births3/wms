#!/usr/bin/env python3
"""check_prototype_navigation.py — 原型导航层次治理

类别：6. 原型治理
Tier：T1（< 10s）
输入：prototypes/src/App.tsx + prototypes/src/Tabs.tsx
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- App.tsx 必须存在领域层导航 DOMAINS，并覆盖 Tabs.tsx 的 GROUP_ORDER
- 导航必须具备 3 层结构：领域 / 模块分组 / 原型页
- 导航必须支持搜索、端筛选、当前页面包屑
- 切换 tab 时必须同步当前领域，避免选中页藏在其它领域下
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
APP_FILE = REPO_ROOT / "prototypes" / "src" / "App.tsx"
TABS_FILE = REPO_ROOT / "prototypes" / "src" / "Tabs.tsx"

GROUP_ORDER_RE = re.compile(r"export const GROUP_ORDER:\s*Group\[\]\s*=\s*\[(.*?)\];", re.DOTALL)
DOMAINS_RE = re.compile(r"const DOMAINS:\s*DomainMeta\[\]\s*=\s*\[(.*?)\];", re.DOTALL)
STRING_RE = re.compile(r'"([^"]+)"')

REQUIRED_DOMAIN_IDS = {"kit", "foundation", "warehouse", "portals", "extensions"}
REQUIRED_TOKENS = {
    "领域层 DOMAINS": "const DOMAINS",
    "领域反查 DOMAIN_BY_GROUP": "DOMAIN_BY_GROUP",
    "当前领域状态 activeDomain": "activeDomain",
    "模块分组 navigationSections": "navigationSections",
    "搜索输入 query": "query",
    "搜索匹配 tabMatchesQuery": "tabMatchesQuery",
    "端筛选 deviceFilter": "deviceFilter",
    "端筛选 DEVICE_CHIPS": "DEVICE_CHIPS",
    "面包屑 domainLabel": "domainLabel(domainForGroup(currentTab.group))",
    "面包屑 currentTab.group": "currentTab.group",
    "切换页同步领域": "setActiveDomain(domainForGroup(nextTab.group))",
    "分组页计数": "section.tabs.length",
}


@dataclass
class Issue:
    kind: str
    target: str
    detail: str


def _extract_group_order(tabs_text: str) -> list[str]:
    match = GROUP_ORDER_RE.search(tabs_text)
    if not match:
        return []
    return STRING_RE.findall(match.group(1))


def _extract_domain_body(app_text: str) -> str:
    match = DOMAINS_RE.search(app_text)
    return match.group(1) if match else ""


def run() -> list[Issue]:
    issues: list[Issue] = []
    for path in (APP_FILE, TABS_FILE):
        if not path.exists():
            issues.append(Issue("missing_file", path.relative_to(REPO_ROOT).as_posix(), "文件不存在"))
    if issues:
        return issues

    app_text = APP_FILE.read_text(encoding="utf-8")
    tabs_text = TABS_FILE.read_text(encoding="utf-8")

    group_order = _extract_group_order(tabs_text)
    if not group_order:
        issues.append(Issue("parse_group_order", "Tabs.tsx", "未解析到 GROUP_ORDER"))

    domain_body = _extract_domain_body(app_text)
    if not domain_body:
        issues.append(Issue("parse_domains", "App.tsx", "未解析到 DOMAINS 领域导航配置"))

    for label, token in REQUIRED_TOKENS.items():
        if token not in app_text:
            issues.append(Issue("missing_navigation_capability", "App.tsx", f"缺少 {label}"))

    domain_ids = set(re.findall(r'id:\s*"([^"]+)"', domain_body))
    missing_domains = REQUIRED_DOMAIN_IDS - domain_ids
    for domain_id in sorted(missing_domains):
        issues.append(Issue("missing_domain", "App.tsx", f"DOMAINS 缺少领域 id={domain_id}"))

    domain_groups = set(re.findall(r'"([^"]+)"', domain_body))
    for group in group_order:
        if group not in domain_groups:
            issues.append(Issue("unmapped_group", group, "GROUP_ORDER 中的模块分组未被 DOMAINS 领域层覆盖"))

    if "TABS.map(" in app_text and "navigationSections.map(" not in app_text:
        issues.append(Issue("flat_navigation_regression", "App.tsx", "疑似退回平铺导航：直接 TABS.map 且未按 navigationSections 分组"))

    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        issues = run()
    except Exception as e:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({
            "status": "fail" if issues else "pass",
            "issues": [asdict(issue) for issue in issues],
            "ok": not issues,
        }, ensure_ascii=False, indent=2))
    else:
        if issues:
            print(f"✗ check_prototype_navigation: {len(issues)} 项违规")
            for issue in issues:
                print(f"  - [{issue.kind}] {issue.target}: {issue.detail}")
        else:
            print("✓ check_prototype_navigation: 原型导航层次通过")

    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
