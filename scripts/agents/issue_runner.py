#!/usr/bin/env python3
"""Gitea issue agent runner.

默认只 dry-run：读取 issue 和评论，生成方案提案或 codex exec prompt。
只有传 `--apply` 才会评论 Gitea 或在独立 worktree 中直接运行 codex exec。
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from shlex import quote
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUT_DIR = REPO_ROOT / ".codex" / "issue-agent"
CONSUMED_CONFIRMATIONS_FILE = DEFAULT_OUT_DIR / "consumed-confirmations.json"
ISSUE_AGENT_ENV_FILE = DEFAULT_OUT_DIR / "env"
ISSUE_WORKTREE_PARENT = REPO_ROOT.parent
PROPOSAL_MARKER = "<!-- wms-issue-agent:proposal:v1 -->"
REVISION_PROPOSAL_MARKER = "<!-- wms-issue-agent:revision-proposal:v1 -->"
EXEC_MARKER = "<!-- wms-issue-agent:exec:v1 -->"
DELIVERY_MARKER = "<!-- wms-issue-agent:delivery:v1 -->"
MERGE_MARKER = "<!-- wms-issue-agent:merge:v1 -->"
MERGE_FAILED_MARKER = "<!-- wms-issue-agent:merge-failed:v1 -->"
MERGE_CORRECTION_MARKER = "<!-- wms-issue-agent:merge-correction:v1 -->"
STATUS_CORRECTION_MARKER = "<!-- wms-issue-agent:status-correction:v1 -->"
MERGE_RETRY_COMMAND = "/retry-merge"
CONFIRM_PHRASE = "确认方案"
CODEX_ENV_KEYS = (
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
)

STATUS_COMMENT_TOKENS = (
    "最终交付 PR",
    "PR 合并前置条件",
    "等待用户确认合并",
    "等待主代理合并",
    "等待 issue watcher 自动合并",
    "等待自动合并",
    "已创建待合并 PR",
    "本轮不会自行合并",
    "本 PR 不自行合并",
    "已按确认处理 issue",
    "codex exec 日志",
)


@dataclass(frozen=True)
class Action:
    issue: dict[str, Any]
    kind: str
    body: str
    confirm_key: str | None = None


def run(cmd: list[str], *, input_text: str | None = None) -> str:
    result = subprocess.run(
        cmd,
        input=input_text,
        capture_output=True,
        text=True,
        check=False,
        cwd=REPO_ROOT,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"{' '.join(cmd)} failed: {detail}")
    return result.stdout


def tea_api(endpoint: str, *, method: str = "GET", payload: dict[str, Any] | None = None) -> Any:
    cmd = ["tea", "api", "-X", method, endpoint]
    if payload is not None:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as f:
            json.dump(payload, f, ensure_ascii=False)
            payload_path = f.name
        try:
            cmd.extend(["-d", f"@{payload_path}"])
            text = run(cmd)
        finally:
            Path(payload_path).unlink(missing_ok=True)
    else:
        text = run(cmd)
    return json.loads(text or "null")


def list_open_issues(limit: int) -> list[dict[str, Any]]:
    items = tea_api(f"/repos/{{owner}}/{{repo}}/issues?state=open&limit={limit}")
    return [item for item in items if not item.get("pull_request")]


def list_closed_issues(limit: int) -> list[dict[str, Any]]:
    items = tea_api(f"/repos/{{owner}}/{{repo}}/issues?state=closed&limit={limit}")
    return [item for item in items if not item.get("pull_request")]


def list_open_pulls(limit: int) -> list[dict[str, Any]]:
    return tea_api(f"/repos/{{owner}}/{{repo}}/pulls?state=open&limit={limit}")


def get_pull(index: int) -> dict[str, Any]:
    return tea_api(f"/repos/{{owner}}/{{repo}}/pulls/{index}")


def get_issue(index: int) -> dict[str, Any]:
    return tea_api(f"/repos/{{owner}}/{{repo}}/issues/{index}")


def get_comments(index: int) -> list[dict[str, Any]]:
    return tea_api(f"/repos/{{owner}}/{{repo}}/issues/{index}/comments")


def post_comment(index: int, body: str) -> None:
    tea_api(f"/repos/{{owner}}/{{repo}}/issues/{index}/comments", method="POST", payload={"body": body})


def parse_time(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def body_of(comment: dict[str, Any]) -> str:
    return str(comment.get("body") or "")


def latest_marker_time(comments: list[dict[str, Any]], marker: str) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if marker in body_of(c)]
    return max(times) if times else None


def latest_marker_comment(comments: list[dict[str, Any]], markers: tuple[str, ...]) -> dict[str, Any] | None:
    matches = [c for c in comments if any(marker in body_of(c) for marker in markers)]
    if not matches:
        return None
    return max(matches, key=lambda c: parse_time(str(c["created_at"])))


def has_command_after(comments: list[dict[str, Any]], since: datetime, command: str) -> bool:
    for comment in comments:
        created = parse_time(str(comment["created_at"]))
        if created > since and not is_agent_comment(comment) and has_command(body_of(comment), command):
            return True
    return False


def has_active_merge_marker(comments: list[dict[str, Any]]) -> bool:
    merge_time = latest_marker_time(comments, MERGE_MARKER)
    if merge_time is None:
        return False
    correction_time = latest_marker_time(comments, MERGE_CORRECTION_MARKER)
    return correction_time is None or correction_time < merge_time


def has_command(text: str, command: str) -> bool:
    pattern = re.compile(rf"^\s*{re.escape(command)}(?:\s|$)")
    return any(pattern.match(line) for line in text.splitlines())


def has_confirm_after(comments: list[dict[str, Any]], since: datetime) -> bool:
    return latest_confirm_after(comments, since) is not None


def latest_confirm_after(comments: list[dict[str, Any]], since: datetime) -> dict[str, Any] | None:
    matches: list[dict[str, Any]] = []
    for comment in comments:
        created = parse_time(str(comment["created_at"]))
        if created <= since or is_agent_comment(comment):
            continue
        if is_confirm_comment(comment):
            matches.append(comment)
    if not matches:
        return None
    return max(matches, key=lambda c: parse_time(str(c["created_at"])))


def confirm_key(comment: dict[str, Any]) -> str:
    if comment.get("id") is not None:
        return str(comment["id"])
    return f"{comment.get('created_at', '')}:{body_of(comment).strip()}"


def load_consumed_confirmations() -> dict[str, list[str]]:
    try:
        data = json.loads(CONSUMED_CONFIRMATIONS_FILE.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}
    if not isinstance(data, dict):
        return {}
    return {str(k): [str(item) for item in v] for k, v in data.items() if isinstance(v, list)}


def confirmation_consumed(issue_number: int, key: str) -> bool:
    return key in set(load_consumed_confirmations().get(str(issue_number), []))


def mark_confirmation_consumed(issue_number: int, key: str) -> None:
    data = load_consumed_confirmations()
    values = data.setdefault(str(issue_number), [])
    if key not in values:
        values.append(key)
    CONSUMED_CONFIRMATIONS_FILE.parent.mkdir(parents=True, exist_ok=True)
    tmp = CONSUMED_CONFIRMATIONS_FILE.with_suffix(".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(CONSUMED_CONFIRMATIONS_FILE)


def has_reject_after(comments: list[dict[str, Any]], since: datetime) -> bool:
    for comment in comments:
        created = parse_time(str(comment["created_at"]))
        if created > since and has_command(body_of(comment), "/reject"):
            return True
    return False


def short(text: str, limit: int) -> str:
    normalized = " ".join(str(text or "").split())
    if len(normalized) <= limit:
        return normalized
    return normalized[: limit - 1] + "…"


def is_agent_comment(comment: dict[str, Any]) -> bool:
    return "wms-issue-agent:" in body_of(comment)


def is_status_comment(comment: dict[str, Any]) -> bool:
    text = body_of(comment)
    return is_agent_comment(comment) or any(token in text for token in STATUS_COMMENT_TOKENS)


def is_obsolete_control_comment(comment: dict[str, Any]) -> bool:
    if is_agent_comment(comment):
        return False
    text = body_of(comment)
    return (
        has_command(text, "/confirm")
        or has_command(text, "/codex run")
        or any(line.strip() == "开始处理" for line in text.splitlines())
    )


def is_confirm_comment(comment: dict[str, Any]) -> bool:
    return body_of(comment).strip() == CONFIRM_PHRASE


def comment_summary(comments: list[dict[str, Any]], limit: int = 5) -> str:
    human_comments = [c for c in comments if not is_agent_comment(c)]
    if not human_comments:
        return "暂无人工评论。"
    lines: list[str] = []
    for c in human_comments[-limit:]:
        author = (c.get("user") or {}).get("login", "unknown")
        lines.append(f"- {author}: {short(normalize_attachment_links(body_of(c)), 220)}")
    return "\n".join(lines)


def gitea_root() -> str:
    remote = run(["git", "remote", "get-url", "origin"]).strip()
    return remote.removesuffix(".git").rsplit("/", 2)[0]


def normalize_attachment_links(text: str) -> str:
    root = gitea_root()
    return text.replace("](/attachments/", f"]({root}/attachments/").replace("(/attachments/", f"({root}/attachments/")


def attachment_summary(
    comments: list[dict[str, Any]],
    *,
    issue: dict[str, Any] | None = None,
    limit: int = 10,
) -> str:
    lines: list[str] = []
    if issue:
        author = (issue.get("user") or {}).get("login", "issue")
        for asset in issue.get("assets") or []:
            url = asset.get("browser_download_url")
            if url:
                lines.append(f"- {author}: {asset.get('name', 'attachment')} {url}")
    for c in comments[-limit:]:
        if is_agent_comment(c):
            continue
        author = (c.get("user") or {}).get("login", "unknown")
        for asset in c.get("assets") or []:
            url = asset.get("browser_download_url")
            if url:
                lines.append(f"- {author}: {asset.get('name', 'attachment')} {url}")
    return "\n".join(lines) if lines else "无"


def text_blob(*parts: Any) -> str:
    return "\n".join(str(part or "") for part in parts)


CODE_CONTEXT_GLOBS = [
    "packages/ui/src",
    "packages/ui/self-checks",
    "apps/web-admin/src",
    "apps/web-admin/vite.config.ts",
    "backend/crates/api/src",
    "docs",
    "scripts",
    "justfile",
]

DOMAIN_KEYWORDS = [
    "供应商",
    "客户",
    "来源",
    "批量导入",
    "新建供应商",
    "新建客户",
    "DataGrid",
    "datagrid",
    "视图",
    "保存",
    "下拉",
    "输入值",
    "登录",
    "按钮",
    "弹窗",
    "窗口",
    "菜单",
    "导航",
    "9002",
    "字段",
    "创建时间",
    "截图",
    "收货",
    "验收",
    "上架",
    "出库",
    "库位",
    "商品",
    "客商",
    "字典",
]

KEYWORD_ALIASES = {
    "供应商": ["m1-suppliers", "supplier"],
    "客户": ["m1-customers", "customer"],
    "来源": ["sourceValue", "source"],
    "批量导入": ["批量导入", "import"],
    "新建供应商": ["createSupplier", "新建供应商"],
    "新建客户": ["createCustomer", "新建客户"],
    "弹窗": ["Dialog", "Popover", "Modal"],
    "窗口": ["Dialog", "Popover", "Modal"],
}


def issue_keywords(text: str) -> list[str]:
    keywords: list[str] = []
    for word in DOMAIN_KEYWORDS:
        if word in text:
            keywords.extend(KEYWORD_ALIASES.get(word, []))
            keywords.append(word)
    for word in re.findall(r"[A-Za-z][A-Za-z0-9_-]{2,}|M\d+", text):
        if word.lower() not in {"issue", "http", "https"}:
            keywords.append(word)
    seen: set[str] = set()
    return [word for word in keywords if not (word in seen or seen.add(word))][:10]


def code_context_summary(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    keywords = issue_keywords(text)
    if not keywords:
        return "未提取到可检索关键词；当前判断只能基于 issue 文本，执行前必须补充页面、接口、截图或复现步骤。"

    lines: list[str] = []
    for keyword in keywords:
        matches: list[str] = []
        for target in CODE_CONTEXT_GLOBS:
            cmd = ["rg", "-n", "--with-filename", "--fixed-strings", keyword, target]
            result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT, check=False)
            for line in result.stdout.splitlines():
                if line.strip() and not line.startswith("scripts/agents/issue_runner.py:"):
                    matches.append(line)
                if len(matches) >= 3:
                    break
            if len(matches) >= 3:
                break
        if matches:
            lines.append(f"- `{keyword}` 命中：")
            lines.extend(f"  - `{short(match, 180)}`" for match in matches)
        if len(lines) >= 9:
            break
    if not lines:
        return "未在前端、后端、脚本或文档中命中 issue 关键词；执行前必须补充更具体的页面、接口、截图或复现步骤。"
    return "\n".join(lines[:12])


COMMONALITY_RULES = [
    (
        ("弹窗", "弹出", "窗口", "遮罩", "关闭", "外边", "外部"),
        "弹层 / 弹窗关闭交互",
        "共享 Dialog / Popover / Dropdown / DataGrid 弹层组件，以及页面私有弹窗",
        "先查是否走共享组件；能在共享组件修就一起修同类弹层，不能只补单页。若是页面私有实现，先修当前页并把关闭规则沉淀到前端规范和回归检查。",
    ),
    (
        ("字段", "创建时间", "货主", "状态", "单据类型", "筛选", "分页", "列设置"),
        "列表字段 / DataGrid 规则",
        "所有管理端 DataGrid 列表、字段矩阵、查询条件和列设置",
        "先对齐字段矩阵和 DataGrid 规范；同类页面缺口一起补，并扩展 T1 静态检查。",
    ),
    (
        ("截图", "附件", "回写", "证据", "重启", "版本"),
        "证据回写闭环",
        "issue / PR 评论、截图附件、本地测试环境重启和版本校验证据",
        "先补当前证据，再更新 issue-agent prompt、runbook 或执行复盘技能，避免只写本地路径或漏传附件。",
    ),
    (
        ("按钮", "菜单", "导航", "查询", "新增", "导入"),
        "管理端动作入口一致性",
        "同类业务页面的按钮、菜单入口、弹窗动作和权限状态",
        "先查同模块同类页面；如果模式重复，抽到共享页面模型或组件，并补页面动作矩阵。",
    ),
]


def commonality_summary(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    matched: list[tuple[str, str, str]] = []
    for words, category, scope, action in COMMONALITY_RULES:
        if any(word in text for word in words):
            matched.append((category, scope, action))
    if not matched:
        return (
            "- 初判：暂未证明是共性问题。\n"
            "- 是否一起修改：先按当前 issue 定位；执行中若发现同类页面、组件、字段、流程或治理脚本也受影响，停止并升级为共性修复。\n"
            "- 预防治理：若能脚本化，补 T1 检查；否则更新对应 skill、runbook 或规范文档。"
        )
    lines = ["- 初判：可能是共性问题，执行前必须先核查同类范围。"]
    for category, scope, action in matched[:2]:
        lines.append(f"- 共性类型：{category}")
        lines.append(f"- 相似范围：{scope}")
        lines.append(f"- 是否一起修改：{action}")
    lines.append("- 预防治理：把可复发约束写入 prompt / skill / runbook；能静态检查的再补治理脚本。")
    return "\n".join(lines)


def pr_mentions_issue(pr: dict[str, Any], issue_number: int) -> bool:
    text = text_blob(pr.get("title"), pr.get("body"))
    return bool(re.search(rf"(?:issue|关联|关闭|close[sd]?)\s*#?{issue_number}\b", text, re.I))


def has_merge_blocker(text: str) -> bool:
    return any(word in text for word in ("/reject", "不要合并", "暂不合并", "合并阻塞", "blocked", "BLOCKED"))


def has_merge_evidence(text: str) -> bool:
    return "验证" in text and ("截图" in text or "附件" in text) and ("重启" in text or "healthz" in text)


def is_auto_merge_branch(head_ref: str, issue_number: int) -> bool:
    return head_ref.startswith("agent/") or head_ref.startswith(f"fix/issue-{issue_number}-")


def current_branch() -> str:
    return run(["git", "branch", "--show-current"]).strip()


def pull_is_merged(pr: dict[str, Any]) -> bool:
    return bool(pr.get("merged") or pr.get("merged_at") or pr.get("merge_commit_sha"))


def merge_verification_error(pr: dict[str, Any]) -> str | None:
    if pull_is_merged(pr):
        return None
    return (
        f"state={pr.get('state')}, merged={pr.get('merged')}, "
        f"mergeable={pr.get('mergeable')}, merged_at={pr.get('merged_at')}, "
        f"merge_commit_sha={pr.get('merge_commit_sha')}"
    )


def merge_blockers(issue: dict[str, Any], comments: list[dict[str, Any]], pr: dict[str, Any]) -> list[str]:
    human_comments = [body_of(c) for c in comments if "wms-issue-agent:" not in body_of(c)]
    text = text_blob(issue.get("body"), pr.get("body"), *human_comments)
    head_ref = str((pr.get("head") or {}).get("ref") or "")
    base_ref = str((pr.get("base") or {}).get("ref") or "")
    blockers: list[str] = []
    if issue.get("state") != "closed":
        blockers.append("issue 未关闭")
    if base_ref and base_ref != current_branch():
        blockers.append(f"PR base 不是当前工作分支：{base_ref}")
    if has_active_merge_marker(comments):
        if pull_is_merged(pr):
            blockers.append("issue 已有合并 marker")
        else:
            blockers.append("issue 有未纠正的合并 marker 但 PR 未合并")
    failure_time = latest_marker_time(comments, MERGE_FAILED_MARKER)
    if failure_time and not has_command_after(comments, failure_time, MERGE_RETRY_COMMAND):
        blockers.append(f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试")
    if pr.get("state") != "open":
        blockers.append("PR 不是 open")
    if pull_is_merged(pr):
        blockers.append("PR 已合并")
    if not pr.get("mergeable"):
        blockers.append("PR 不可自动合并")
    if not is_auto_merge_branch(head_ref, int(issue["number"])):
        blockers.append("PR 分支不是 agent/* 或 fix/issue-<编号>-*")
    if has_merge_blocker(text):
        blockers.append("存在阻塞或拒绝合并评论")
    if not has_merge_evidence(text):
        blockers.append("缺少验证、截图或重启证据")
    return blockers


def latest_human_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if not is_agent_comment(c)]
    return max(times) if times else None


def latest_actionable_human_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [
        parse_time(str(c["created_at"]))
        for c in comments
        if not is_status_comment(c) and not is_obsolete_control_comment(c) and not is_confirm_comment(c)
    ]
    return max(times) if times else None


def latest_status_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if is_status_comment(c)]
    return max(times) if times else None


def build_proposal_comment(issue: dict[str, Any], comments: list[dict[str, Any]], *, revision: bool = False) -> str:
    labels = ", ".join(label["name"] for label in issue.get("labels") or []) or "无"
    scope = "待确认"
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    code_context = code_context_summary(issue, comments)
    commonality = commonality_summary(issue, comments)
    if "菜单" in text or "导航" in text:
        scope = "可能是管理端左侧导航 / 菜单交互"
    conclusion = "需要补充"
    confidence = "中"
    reason = "issue 描述较短，缺少截图、页面入口或期望效果，直接开发容易修错位置。"
    action = "建议先补充截图或说明具体页面；如果确认是管理端菜单视觉问题，再按低风险 UI 修复执行。"
    if any(word in text for word in ("报错", "失败", "打不开", "不能", "无法")):
        conclusion = "建议执行"
        confidence = "中"
        reason = "描述指向可复现故障，优先用日志、接口或页面复现定位。"
        action = "回复裸一行 `确认方案` 后执行最小修复；如复现信息不足，执行时会先停止补问。"
    elif any(word in text for word in ("截图", "图片", "红框", "圈", "页面")):
        conclusion = "建议执行"
        confidence = "中"
        reason = "已有截图或页面位置补充，可以按当前范围做最小修复。"
        action = "回复裸一行 `确认方案` 后执行最小修复；未确认前继续只评论判断。"
    elif any(word in text for word in ("新增", "增加", "字段", "状态", "流程", "规则")):
        conclusion = "需要人工决策"
        confidence = "中"
        reason = "可能引入字段、状态、流程或业务规则变化，不能直接由 agent 拍板。"
        action = "先补业务边界和验收标准，再决定是否确认执行。"
    marker = REVISION_PROPOSAL_MARKER if revision else PROPOSAL_MARKER
    title = "修正方案" if revision else "方案提案"
    feedback_note = "\n本轮是用户验收反馈后的修正方案；旧确认不得复用。\n" if revision else ""
    return f"""{marker}
## WMS Issue Agent {title}

- Issue：#{issue["number"]} {issue["title"]}
- 作者：{(issue.get("user") or {}).get("login", "unknown")}
- 标签：{labels}
- 当前状态：open
- 可能影响范围：{scope}
- 结论：{conclusion}
- 置信度：{confidence}
{feedback_note}

### 问题复述

{short(issue.get("body") or issue.get("title") or "", 800)}

### 评论摘要

{comment_summary(comments)}

### 截图 / 附件

{attachment_summary(comments, issue=issue)}

### 根因文件和代码核查

{code_context}

### 相似 / 共性问题判断

{commonality}

### 改动计划

- 前端：按根因文件定位；若是用户可见行为，补真实页面验证和截图。
- 后端：暂未发现必须改动；如果执行中发现 API / 数据模型缺口，停止并重新提案。
- 数据：暂未发现必须改动；新增字段、状态、角色、模块或业务默认值必须停止确认。
- 文档：如共性成立，更新对应 prompt / runbook / skill / 规范；能脚本化的补治理脚本。
- 测试：先补最小回归测试，再做实现。

### 判断依据

- {reason}
- 上方代码核查是本轮判断的仓库证据；如果没有命中代码，执行前必须先补充复现信息。
- 当前步骤只做判断和确认，不改代码、不提交、不推送。

### 预计影响范围

- 前端：{scope}
- 后端：暂未发现必须改动
- 文档 / 测试：按实际修复范围补最小验证

### 建议动作

{action}

### 验证要求

- 至少运行 `git diff --check` 和 `just gov-t1`。
- 涉及前端或用户可见行为时，必须截图、重启本地测试前后端，并校验运行版本是本次修复提交。

### 风险与停止条件

- 如果需要新增字段、状态、角色、模块或业务默认值，必须停止并向用户确认。
- 如果复现信息不足，必须先补信息，不能靠猜测开发。

### 请确认

- 同意按上述方案执行：回复裸一行 `确认方案`
- 需要补充：直接继续评论需求细节或截图
- 暂不执行：评论 `/reject`
"""


def build_fix_prompt(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    proposal = latest_marker_comment(comments, (PROPOSAL_MARKER, REVISION_PROPOSAL_MARKER))
    proposal_url = str(proposal.get("html_url") or "") if proposal else ""
    commonality = commonality_summary(issue, comments)
    return f"""请处理 Gitea issue #{issue["number"]}：{issue["title"]}

来源：{issue.get("html_url", "")}
确认对应方案：{proposal_url or "未提供"}

用户已在最新方案后回复裸一行“确认方案”。请按 WMS 仓库规则处理：
1. 使用 `wms-issue-codex-exec`、`wms-loop-engineering` 和 `wms-execution-retrospective` 执行；先按 proposal 的“相似 / 共性问题判断”核查同类页面、组件、字段、流程和治理脚本，若共性成立，必须同步补规则、脚本或矩阵，不能只修一个页面。
2. 当前命令已由 issue-agent 在 issue 专属 worktree 中用 `codex exec` 直接启动；先运行 `pwd`、`git status --short --branch` 和 `git worktree list` 核对，禁止修改主工作区。
3. 只修复 issue 指向的问题。新增字段、状态、角色、模块或业务默认值时停止并向用户确认。
4. 完成后运行相关测试，至少 `git diff --check` 和 `just gov-t1`。
5. 涉及前端或用户可见行为时，禁止在子 worktree 中启动或占用 9002；9002 只允许主工作区固定会话 `wms-web-admin-9002`。需要真实截图或重启证据时，合并到主工作区后由主代理运行 `just dev-web-restart` 和 `just dev-web-verify`，后端按仓库现有运行方式启动并检查 `/healthz`。重启后必须校验运行的是本次修复版本：记录当前提交哈希，并用 `just dev-web-verify` 输出、页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存；把端口、URL、提交哈希和版本校验结果写入 issue。
6. 如果 issue 评论包含截图 / 附件，必须先打开或下载附件辅助定位，不能只按文字猜测。
7. 涉及前端或用户可见行为时，必须采集真实前端截图；必须用 `POST /repos/{{owner}}/{{repo}}/issues/<编号>/assets` 把截图上传为 Gitea 附件，并用 Markdown 图片评论到 issue；不能只写本地路径。
8. 当前可用截图 / 附件如下：
{attachment_summary(comments, issue=issue, limit=10)}
9. 读取 Gitea issue、评论或附件元数据必须使用 `tea api`，例如 `tea api /repos/{{owner}}/{{repo}}/issues/{issue["number"]}`；禁止裸 `curl` 访问 Gitea API。
10. 当前暂停 Gitea PR：允许在 issue 专属 worktree 的本地分支上提交，禁止推送远端、禁止创建 Gitea PR、禁止自行合并到主工作区。
11. 最后必须评论 `{DELIVERY_MARKER}` delivery 信息：本地 worktree、分支、提交哈希或 diff 状态、验证结果、截图证据、本地测试环境重启结果、codex exec 日志位置、本地合并前置条件和剩余风险，并明确下一步是“等待主代理本地 review 后合并”或“阻塞”。
12. 如果用户验收反馈不对，不得继续复用本次确认；必须等待 issue-agent 重新生成修正方案并由用户再次回复“确认方案”。

当前相似 / 共性问题判断：
{commonality}

Issue 正文：
{issue.get("body") or ""}

近期人工评论：
{comment_summary(comments, limit=10)}
"""


def choose_action(
    issue: dict[str, Any],
    comments: list[dict[str, Any]],
) -> Action | None:
    human_time = latest_actionable_human_time(comments)
    proposal = latest_marker_comment(comments, (PROPOSAL_MARKER, REVISION_PROPOSAL_MARKER))
    proposal_time = parse_time(str(proposal["created_at"])) if proposal else None
    exec_time = latest_marker_time(comments, EXEC_MARKER)
    status_time = latest_status_time(comments)
    confirm = latest_confirm_after(comments, proposal_time) if proposal_time else None
    key = confirm_key(confirm) if confirm else None
    confirmed = confirm is not None and not confirmation_consumed(int(issue["number"]), key or "")
    if status_time and not confirmed and (human_time is None or human_time <= status_time):
        return None
    if proposal_time is None:
        return Action(issue=issue, kind="proposal", body=build_proposal_comment(issue, comments))
    if has_reject_after(comments, proposal_time):
        return None
    if human_time and human_time > proposal_time:
        if exec_time and exec_time > proposal_time:
            return Action(issue=issue, kind="revision-proposal", body=build_proposal_comment(issue, comments, revision=True))
        return Action(issue=issue, kind="proposal", body=build_proposal_comment(issue, comments))
    if confirmed:
        if exec_time is None or exec_time < proposal_time:
            return Action(issue=issue, kind="exec", body=build_fix_prompt(issue, comments), confirm_key=key)
    if exec_time and exec_time > proposal_time:
        return None
    return None


def write_preview(out_dir: Path, issue_number: int, kind: str, body: str) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"issue-{issue_number}-{kind}.txt"
    path.write_text(body, encoding="utf-8")
    return path


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def issue_worktree(issue_number: int, stamp: str) -> tuple[Path, str]:
    name = f"{REPO_ROOT.name}-agent-issue-{issue_number}-{stamp}"
    branch = f"fix/issue-{issue_number}-{stamp}"
    return ISSUE_WORKTREE_PARENT / name, branch


def read_issue_agent_env(path: Path = ISSUE_AGENT_ENV_FILE) -> dict[str, str]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError:
        return {}
    values: dict[str, str] = {}
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        try:
            parts = shlex.split(stripped, comments=True, posix=True)
        except ValueError:
            continue
        if parts and parts[0] == "export":
            parts = parts[1:]
        for part in parts:
            if "=" not in part:
                continue
            key, value = part.split("=", 1)
            if key in CODEX_ENV_KEYS:
                values[key] = value
    return values


def codex_env_exports() -> list[str]:
    env = codex_exec_env()
    exports = []
    for key in CODEX_ENV_KEYS:
        value = env.get(key)
        if value:
            exports.append(f"export {key}={quote(value)}")
    return exports


def codex_exec_env() -> dict[str, str]:
    env = dict(os.environ)
    env.update(read_issue_agent_env())
    return env


def run_codex_smoke(args: argparse.Namespace) -> int:
    timeout_seconds = args.timeout
    env = codex_exec_env()
    present = [key for key in CODEX_ENV_KEYS if env.get(key)]
    print(f"codex-smoke: env={','.join(present) if present else 'none'}", flush=True)
    result = subprocess.run(
        [
            "codex",
            "exec",
            "--dangerously-bypass-approvals-and-sandbox",
            "-C",
            str(REPO_ROOT),
            "-",
        ],
        input="pwd\n",
        text=True,
        cwd=REPO_ROOT,
        env=env,
        timeout=timeout_seconds,
        check=False,
    )
    print(f"codex-smoke: exit={result.returncode}", flush=True)
    return int(result.returncode)


def build_codex_command(work_dir: Path, prompt_path: Path, log_path: Path) -> str:
    return (
        f"codex exec --dangerously-bypass-approvals-and-sandbox -C {quote(str(work_dir))} "
        f"- < {quote(str(prompt_path))} 2>&1 | tee {quote(str(log_path))}"
    )


def prepare_codex_exec(issue_number: int, prompt_path: Path) -> tuple[Path, str, Path, Path]:
    stamp = datetime.now().strftime("%Y%m%d%H%M%S")
    script = prompt_path.with_suffix(".run.sh")
    log_path = prompt_path.with_suffix(".log")
    work_dir, branch = issue_worktree(issue_number, stamp)
    run(["git", "worktree", "add", "-b", branch, str(work_dir), "HEAD"])
    command = build_codex_command(work_dir, prompt_path, log_path)
    script.write_text(
        "\n".join(
            [
                "#!/usr/bin/env bash",
                "set -euo pipefail",
                *codex_env_exports(),
                f"cd {quote(str(work_dir))}",
                command,
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    script.chmod(0o700)
    return work_dir, branch, script, log_path


def codex_progress_signature(work_dir: Path, log_path: Path) -> tuple[str, str, str]:
    try:
        stat = log_path.stat()
        log_state = f"{stat.st_size}:{stat.st_mtime_ns}"
    except FileNotFoundError:
        log_state = "missing"
    status = subprocess.run(
        ["git", "status", "--porcelain=v1"],
        capture_output=True,
        text=True,
        cwd=work_dir,
        check=False,
    ).stdout
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        cwd=work_dir,
        check=False,
    ).stdout.strip()
    return log_state, status, head


def run_codex_exec(script: Path, *, work_dir: Path, log_path: Path) -> int:
    idle_seconds = int(os.environ.get("WMS_ISSUE_AGENT_CODEX_IDLE_SECONDS", "900"))
    max_seconds = int(os.environ.get("WMS_ISSUE_AGENT_CODEX_MAX_SECONDS", "0"))
    process = subprocess.Popen([str(script)], cwd=work_dir, start_new_session=True)
    started_at = last_progress_at = time.monotonic()
    last_signature = codex_progress_signature(work_dir, log_path)
    try:
        while True:
            try:
                return int(process.wait(timeout=5))
            except subprocess.TimeoutExpired:
                now = time.monotonic()
                signature = codex_progress_signature(work_dir, log_path)
                if signature != last_signature:
                    last_signature = signature
                    last_progress_at = now
                timed_out = (idle_seconds > 0 and now - last_progress_at >= idle_seconds) or (
                    max_seconds > 0 and now - started_at >= max_seconds
                )
                if not timed_out:
                    continue
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                return 124
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()


def execute_action(
    action: Action,
    *,
    apply: bool,
    out_dir: Path,
) -> None:
    issue_number = int(action.issue["number"])
    preview = write_preview(out_dir, issue_number, action.kind, action.body)
    print(f"{action.kind}: issue #{issue_number} preview={display_path(preview)}", flush=True)
    if not apply:
        print("dry-run: 未评论 Gitea，未运行 codex exec", flush=True)
        return
    if action.kind in {"proposal", "revision-proposal"}:
        post_comment(issue_number, action.body)
        print(f"applied: 已评论 issue #{issue_number}", flush=True)
        return
    work_dir, branch, script, log_path = prepare_codex_exec(issue_number, preview)
    post_comment(
        issue_number,
        (
            f"{EXEC_MARKER}\n"
            "已开始直接运行 `codex exec`，等待 Codex 执行并回写 delivery 证据。\n"
            f"- worktree：`{work_dir}`\n"
            f"- 分支：`{branch}`\n"
            f"- 日志：`{log_path}`\n"
            f"- prompt：`{preview}`\n"
        ),
    )
    if action.confirm_key:
        mark_confirmation_consumed(issue_number, action.confirm_key)
    print(f"applied: 已开始 codex exec issue #{issue_number} worktree={work_dir}", flush=True)
    exit_code = run_codex_exec(script, work_dir=work_dir, log_path=log_path)
    if exit_code != 0:
        if exit_code == 124:
            reason = "执行空闲超时"
            next_step = "排查 Codex 连接或执行卡住原因后，由人工补充新评论触发下一轮判断；旧确认不会复用。"
        elif exit_code < 0:
            reason = f"收到信号 {-exit_code} 终止"
            next_step = "本轮视为外部停止，只记录状态；不会自动重发方案，也不会复用旧确认。"
        else:
            reason = "执行失败"
            next_step = "排查失败原因后，由人工补充新评论触发下一轮判断；旧确认不会复用。"
        post_comment(
            issue_number,
            (
                f"{STATUS_CORRECTION_MARKER}\n"
                f"`codex exec` {reason}，未形成完成闭环。\n"
                f"- issue：#{issue_number}\n"
                f"- 分支：`{branch}`\n"
                f"- worktree：`{work_dir}`\n"
                f"- 日志：`{log_path}`\n"
                f"- 退出码：{exit_code}\n"
                f"- 下一步：{next_step}\n"
            ),
        )
        raise RuntimeError(f"codex exec failed: exit={exit_code}, log={log_path}")
    print(f"applied: codex exec 已结束 issue #{issue_number} exit=0", flush=True)


def merge_pull(pr_number: int) -> str:
    return run(["tea", "pulls", "merge", str(pr_number), "--style", "squash"])


def merge_pull_checked(pr_number: int) -> dict[str, Any]:
    merge_pull(pr_number)
    refreshed = get_pull(pr_number)
    error = merge_verification_error(refreshed)
    if error:
        raise RuntimeError(f"合并命令已返回但 PR #{pr_number} 未处于已合并状态：{error}")
    return refreshed


def build_merge_failed_comment(issue_number: int, pr: dict[str, Any], error: str) -> str:
    return (
        f"{MERGE_FAILED_MARKER}\n"
        "自动合并已停止：合并命令执行后复查 PR，未确认进入已合并状态，因此没有写入合并 marker。\n"
        f"- issue：#{issue_number}\n"
        f"- PR：#{pr['number']}\n"
        f"- 分支：`{(pr.get('head') or {}).get('ref', '')}`\n"
        f"- 复查结果：{short(error, 500)}\n"
        f"- 处理方式：修复 PR 可合并状态后，人工评论 `{MERGE_RETRY_COMMAND}` 再允许 watcher 重试。\n"
    )


def run_merge_closed(args: argparse.Namespace) -> int:
    issues = [get_issue(args.issue)] if args.issue else list_closed_issues(args.limit)
    pulls = list_open_pulls(args.pr_limit)
    for issue in issues:
        issue_number = int(issue["number"])
        issue_comments = get_comments(issue_number)
        linked = [pr for pr in pulls if pr_mentions_issue(pr, issue_number)]
        if not linked:
            print(f"skip: issue #{issue_number} 未找到关联 open PR", flush=True)
            continue
        for pr in linked:
            pr = get_pull(int(pr["number"]))
            pr_comments = get_comments(int(pr["number"]))
            comments = [*issue_comments, *pr_comments]
            blockers = merge_blockers(issue, comments, pr)
            if blockers:
                print(f"skip: issue #{issue_number} PR #{pr['number']}：{'; '.join(blockers)}", flush=True)
                continue
            if not args.apply:
                print(f"dry-run: 将合并 issue #{issue_number} PR #{pr['number']}", flush=True)
                return 0
            try:
                merged_pr = merge_pull_checked(int(pr["number"]))
            except Exception as exc:  # noqa: BLE001
                body = build_merge_failed_comment(issue_number, pr, str(exc))
                post_comment(issue_number, body)
                post_comment(int(pr["number"]), body)
                print(f"merge-failed: issue #{issue_number} PR #{pr['number']}：{short(str(exc), 240)}", flush=True)
                return 2
            body = (
                f"{MERGE_MARKER}\n"
                f"issue 已关闭，PR #{pr['number']} 已按闭环规则自动合并。\n"
                f"- 分支：`{(pr.get('head') or {}).get('ref', '')}`\n"
                f"- 合并提交：`{merged_pr.get('merge_commit_sha', '')}`\n"
                f"- 触发：closed issue #{issue_number}\n"
            )
            post_comment(issue_number, body)
            post_comment(int(pr["number"]), body)
            print(f"merged: issue #{issue_number} PR #{pr['number']}", flush=True)
            return 0
    print("no-action: 没有可自动合并的 closed issue PR", flush=True)
    return 0


def run_once(args: argparse.Namespace) -> int:
    raw_issues = [get_issue(args.issue)] if args.issue else list_open_issues(args.limit)
    for raw_issue in raw_issues:
        issue = get_issue(int(raw_issue["number"]))
        comments = get_comments(int(issue["number"]))
        action = choose_action(issue, comments)
        if action is not None:
            execute_action(
                action,
                apply=args.apply,
                out_dir=Path(args.out_dir),
            )
            return 0
    print("no-action: 没有需要处理的 issue", flush=True)
    return 0


def run_watch(args: argparse.Namespace) -> int:
    count = 0
    while True:
        try:
            run_once(args)
            if args.merge_closed:
                run_merge_closed(args)
        except Exception as exc:  # noqa: BLE001
            print(f"watch-error: {exc}", file=sys.stderr, flush=True)
        count += 1
        if args.max_iterations and count >= args.max_iterations:
            return 0
        time.sleep(args.interval)


def self_test() -> int:
    global CONSUMED_CONFIRMATIONS_FILE
    CONSUMED_CONFIRMATIONS_FILE = Path(tempfile.mkdtemp(prefix="wms-issue-agent-test-")) / "consumed.json"
    issue = {"number": 7, "title": "测试", "body": "正文", "labels": [], "user": {"login": "u"}}
    assert display_path(REPO_ROOT / "justfile") == "justfile"
    assert display_path(Path("/tmp/wms-issue-agent-preview.txt")) == "/tmp/wms-issue-agent-preview.txt"
    worktree_path, branch = issue_worktree(7, "20260702010101")
    assert worktree_path.name == "wms-agent-issue-7-20260702010101"
    assert branch == "fix/issue-7-20260702010101"
    tmp_env = Path(tempfile.mkdtemp(prefix="wms-issue-agent-env-")) / "env"
    tmp_env.write_text(
        "\n".join(
            [
                "# comment",
                "export https_proxy=http://127.0.0.1:7894",
                "http_proxy='http://127.0.0.1:7894'",
                "IGNORED=value",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    assert read_issue_agent_env(tmp_env) == {
        "https_proxy": "http://127.0.0.1:7894",
        "http_proxy": "http://127.0.0.1:7894",
    }
    command = build_codex_command(
        worktree_path,
        DEFAULT_OUT_DIR / "issue-7-exec.txt",
        DEFAULT_OUT_DIR / "issue-7-exec.log",
    )
    assert f"-C {quote(str(worktree_path))}" in command
    assert f"-C {quote(str(REPO_ROOT))} " not in command
    t0 = "2026-07-01T00:00:00Z"
    t1 = "2026-07-01T00:01:00Z"
    comments = [{"created_at": t0, "body": build_proposal_comment(issue, [])}]
    assert "### 根因文件和代码核查" in comments[0]["body"]
    assert "### 相似 / 共性问题判断" in comments[0]["body"]
    assert "暂未证明是共性问题" in comments[0]["body"]
    assert "/confirm" not in comments[0]["body"]
    assert "开始处理" not in comments[0]["body"]
    popup_issue = {**issue, "title": "点击按钮弹出新窗口时点击外边区域不会关闭"}
    popup_proposal = build_proposal_comment(popup_issue, [])
    assert "弹层 / 弹窗关闭交互" in popup_proposal
    assert "是否一起修改" in popup_proposal
    assert "prompt / runbook / skill / 规范" in popup_proposal
    assert "弹层 / 弹窗关闭交互" in build_fix_prompt(popup_issue, [])
    assert choose_action(issue, []).kind == "proposal"
    assert choose_action(issue, comments) is None
    confirmed = [*comments, {"created_at": t1, "body": "/confirm"}]
    assert choose_action(issue, confirmed) is None
    confirmed_zh = [*comments, {"created_at": t1, "body": "确认方案"}]
    action = choose_action(issue, confirmed_zh)
    assert action.kind == "exec"
    assert action.confirm_key
    mark_confirmation_consumed(7, action.confirm_key)
    assert choose_action(issue, confirmed_zh) is None
    later_confirmed_zh = [*comments, {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"}]
    assert choose_action(issue, later_confirmed_zh).kind == "exec"
    multiline_confirm = [*comments, {"created_at": t1, "body": "确认方案\n补充：需要截图"}]
    assert choose_action(issue, multiline_confirm).kind == "proposal"
    detail_then_confirm = [
        *comments,
        {"created_at": t1, "body": "补充：需要截图"},
        {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"},
    ]
    assert choose_action(issue, detail_then_confirm).kind == "proposal"
    start_zh = [*comments, {"created_at": t1, "body": "开始处理"}]
    assert choose_action(issue, start_zh) is None
    mentioned = [*comments, {"created_at": t1, "body": "不要 /confirm，先补截图"}]
    assert choose_action(issue, mentioned).kind == "proposal"
    mentioned_zh = [*comments, {"created_at": t1, "body": "不要确认方案，先补截图"}]
    assert choose_action(issue, mentioned_zh).kind == "proposal"
    refreshed = [*mentioned, {"created_at": "2026-07-01T00:02:00Z", "body": build_proposal_comment(issue, mentioned)}]
    assert choose_action(issue, refreshed) is None
    sent = [*confirmed_zh, {"created_at": "2026-07-01T00:02:00Z", "body": EXEC_MARKER}]
    assert choose_action(issue, sent) is None
    failed_exec = [
        *sent,
        {
            "created_at": "2026-07-01T00:03:00Z",
            "body": f"{STATUS_CORRECTION_MARKER}\n`codex exec` 执行超时，未形成完成闭环。",
        },
    ]
    assert choose_action(issue, failed_exec) is None
    failed_exec_with_feedback = [
        *failed_exec,
        {"created_at": "2026-07-01T00:04:00Z", "body": "重新执行，继续处理"},
    ]
    assert choose_action(issue, failed_exec_with_feedback).kind == "revision-proposal"
    failed_exec_refreshed = [
        *failed_exec_with_feedback,
        {
            "created_at": "2026-07-01T00:05:00Z",
            "body": build_proposal_comment(issue, failed_exec_with_feedback, revision=True),
        },
    ]
    assert choose_action(issue, failed_exec_refreshed) is None
    delivered = [
        *sent,
        {
            "created_at": "2026-07-01T00:03:00Z",
            "body": "已按确认处理 issue #7，最终交付 PR 为 #9。PR 合并前置条件：等待用户确认合并。",
        },
    ]
    assert choose_action(issue, delivered) is None
    after_delivery = [*delivered, {"created_at": "2026-07-01T00:04:00Z", "body": "补充：按钮还缺一个"}]
    assert choose_action(issue, after_delivery).kind == "revision-proposal"
    after_exec = [*sent, {"created_at": "2026-07-01T00:03:00Z", "body": "不对，还缺下拉异常"}]
    assert choose_action(issue, after_exec).kind == "revision-proposal"
    after_exec_refreshed = [
        *after_exec,
        {"created_at": "2026-07-01T00:04:00Z", "body": build_proposal_comment(issue, after_exec)},
    ]
    assert choose_action(issue, after_exec_refreshed) is None
    after_exec_confirmed = [*after_exec_refreshed, {"created_at": "2026-07-01T00:05:00Z", "body": "确认方案"}]
    assert choose_action(issue, after_exec_confirmed).kind == "exec"
    rejected = [*comments, {"created_at": t1, "body": "/reject 不执行 /confirm"}]
    assert choose_action(issue, rejected) is None
    with_asset = [
        *comments,
        {
            "created_at": t1,
            "body": "见截图",
            "user": {"login": "u"},
            "assets": [{"name": "image.png", "browser_download_url": "http://gitea/attachments/a"}],
        },
    ]
    assert choose_action(issue, [*with_asset, {"created_at": "2026-07-01T00:02:00Z", "body": "确认方案"}]).kind == "proposal"
    with_asset_refreshed = [
        *with_asset,
        {"created_at": "2026-07-01T00:02:00Z", "body": build_proposal_comment(issue, with_asset)},
        {"created_at": "2026-07-01T00:03:00Z", "body": "确认方案"},
    ]
    prompt = choose_action(issue, with_asset_refreshed).body
    assert "http://gitea/attachments/a" in prompt
    assert "上传为 Gitea 附件" in prompt
    assert "wms-execution-retrospective" in prompt
    assert "共性问题" in prompt
    assert DELIVERY_MARKER in prompt
    assert "tea api" in prompt
    assert "禁止裸 `curl`" in prompt
    issue_asset = {
        **issue,
        "assets": [{"name": "issue.png", "browser_download_url": "http://gitea/attachments/issue"}],
    }
    assert "http://gitea/attachments/issue" in build_proposal_comment(issue_asset, [])
    assert "http://gitea/attachments/issue" in build_fix_prompt(issue_asset, [])
    closed_issue = {"number": 8, "state": "closed", "body": "已验收"}
    pr = {
        "number": 9,
        "state": "open",
        "merged": False,
        "mergeable": True,
        "title": "修复：issue #8",
        "body": "关联 issue #8\n验证：通过\n截图证据：已上传附件\n后端重启：/healthz ok",
        "head": {"ref": "agent/issue-8-demo"},
    }
    assert pr_mentions_issue(pr, 8)
    assert not pr_mentions_issue({**pr, "title": "普通变更", "body": "普通 PR #8，不是 issue 关联"}, 8)
    assert merge_blockers(closed_issue, [], pr) == []
    assert merge_blockers(closed_issue, [], {**pr, "head": {"ref": "fix/issue-8-datagrid-views"}}) == []
    assert "PR 分支不是 agent/* 或 fix/issue-<编号>-*" in merge_blockers(
        closed_issue,
        [],
        {**pr, "head": {"ref": "fix/issue-9-other"}},
    )
    assert f"PR base 不是当前工作分支：other-branch" in merge_blockers(
        closed_issue,
        [],
        {**pr, "base": {"ref": "other-branch"}},
    )
    assert pull_is_merged({**pr, "merged": True})
    assert pull_is_merged({**pr, "merged_at": "2026-07-01T00:00:00Z"})
    assert pull_is_merged({**pr, "merge_commit_sha": "abc"})
    assert not pull_is_merged(pr)
    assert merge_verification_error(pr) == (
        "state=open, merged=False, mergeable=True, merged_at=None, merge_commit_sha=None"
    )
    false_merge_marker = [{"created_at": t1, "body": MERGE_MARKER}]
    assert "issue 有未纠正的合并 marker 但 PR 未合并" in merge_blockers(closed_issue, false_merge_marker, pr)
    corrected_marker = [
        *false_merge_marker,
        {"created_at": "2026-07-01T00:02:00Z", "body": MERGE_CORRECTION_MARKER},
    ]
    assert "issue 有未纠正的合并 marker 但 PR 未合并" not in merge_blockers(
        closed_issue,
        corrected_marker,
        pr,
    )
    failed_marker = [
        *corrected_marker,
        {"created_at": "2026-07-01T00:03:00Z", "body": MERGE_FAILED_MARKER},
    ]
    assert f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试" in merge_blockers(
        closed_issue,
        failed_marker,
        pr,
    )
    retried = [
        *failed_marker,
        {"created_at": "2026-07-01T00:04:00Z", "body": MERGE_RETRY_COMMAND},
    ]
    assert f"已有自动合并失败 marker，需人工评论 {MERGE_RETRY_COMMAND} 后重试" not in merge_blockers(
        closed_issue,
        retried,
        pr,
    )
    assert "缺少验证、截图或重启证据" in merge_blockers(closed_issue, [], {**pr, "body": "关联 issue #8"})
    assert merge_blockers(closed_issue, [{"created_at": t1, "body": f"{PROPOSAL_MARKER}\n评论 `/reject`"}], pr) == []
    assert "存在阻塞或拒绝合并评论" in merge_blockers(
        closed_issue,
        [{"created_at": t1, "body": "不要合并"}],
        pr,
    )
    print("self-test: ok", flush=True)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Gitea issue → codex exec runner")
    sub = parser.add_subparsers(dest="command", required=True)

    def add_common(p: argparse.ArgumentParser) -> None:
        p.add_argument("--apply", action="store_true", help="执行写操作：评论 Gitea 或运行 codex exec")
        p.add_argument("--issue", type=int, help="只处理指定 issue 编号")
        p.add_argument("--limit", type=int, default=20, help="每轮最多扫描 open issue 数")
        p.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR), help="本地预览输出目录")

    once = sub.add_parser("once", help="执行一轮扫描")
    add_common(once)
    once.set_defaults(func=run_once)

    watch = sub.add_parser("watch", help="循环扫描")
    add_common(watch)
    watch.add_argument("--interval", type=int, default=60, help="轮询间隔秒数")
    watch.add_argument("--max-iterations", type=int, default=0, help="最多轮询次数；0 表示一直运行")
    watch.add_argument("--merge-closed", action="store_true", help="同时扫描已关闭 issue 并合并满足闭环条件的 PR")
    watch.add_argument("--pr-limit", type=int, default=20, help="每轮最多扫描 open PR 数")
    watch.set_defaults(func=run_watch)

    merge_closed = sub.add_parser("merge-closed", help="扫描已关闭 issue，并合并满足闭环条件的关联 PR")
    merge_closed.add_argument("--apply", action="store_true", help="执行合并并评论；默认只 dry-run")
    merge_closed.add_argument("--issue", type=int, help="只处理指定 issue 编号")
    merge_closed.add_argument("--limit", type=int, default=20, help="每轮最多扫描 closed issue 数")
    merge_closed.add_argument("--pr-limit", type=int, default=20, help="每轮最多扫描 open PR 数")
    merge_closed.set_defaults(func=run_merge_closed)

    test = sub.add_parser("self-test", help="运行内置测试")
    test.set_defaults(func=lambda _args: self_test())

    smoke = sub.add_parser("codex-smoke", help="用 issue-agent 环境运行最小 codex exec")
    smoke.add_argument("--timeout", type=int, default=90, help="smoke 超时秒数")
    smoke.set_defaults(func=run_codex_smoke)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return int(args.func(args))
    except Exception as exc:  # noqa: BLE001
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
