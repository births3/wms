#!/usr/bin/env python3
"""Gitea issue agent runner.

默认只 dry-run：读取 issue 和评论，生成判断评论或 tmux prompt。
只有传 `--apply` 才会评论 Gitea 或发送 prompt 到 tmux。
"""
from __future__ import annotations

import argparse
import json
import os
import re
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
ISSUE_WORKTREE_PARENT = REPO_ROOT.parent
ANALYSIS_MARKER = "<!-- wms-issue-agent:analysis:v2 -->"
TMUX_MARKER = "<!-- wms-issue-agent:tmux:v1 -->"
MERGE_MARKER = "<!-- wms-issue-agent:merge:v1 -->"
MERGE_FAILED_MARKER = "<!-- wms-issue-agent:merge-failed:v1 -->"
MERGE_CORRECTION_MARKER = "<!-- wms-issue-agent:merge-correction:v1 -->"
STATUS_CORRECTION_MARKER = "<!-- wms-issue-agent:status-correction:v1 -->"
MERGE_RETRY_COMMAND = "/retry-merge"
CONFIRM_PHRASES = ("确认方案", "开始处理")

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
)


@dataclass(frozen=True)
class Action:
    issue: dict[str, Any]
    kind: str
    body: str


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


def has_label(issue: dict[str, Any], name: str) -> bool:
    return any(str(label.get("name")) == name for label in issue.get("labels") or [])


def has_command(text: str, command: str) -> bool:
    pattern = re.compile(rf"^\s*{re.escape(command)}(?:\s|$)")
    return any(pattern.match(line) for line in text.splitlines())


def has_confirm_after(comments: list[dict[str, Any]], since: datetime, token: str) -> bool:
    for comment in comments:
        created = parse_time(str(comment["created_at"]))
        text = body_of(comment)
        phrase_confirmed = any(line.strip() in CONFIRM_PHRASES for line in text.splitlines())
        if created > since and (
            has_command(text, token) or has_command(text, "/codex run") or phrase_confirmed
        ):
            return True
    return False


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
    times = [parse_time(str(c["created_at"])) for c in comments if not is_status_comment(c)]
    return max(times) if times else None


def latest_status_time(comments: list[dict[str, Any]]) -> datetime | None:
    times = [parse_time(str(c["created_at"])) for c in comments if is_status_comment(c)]
    return max(times) if times else None


def build_analysis_comment(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    labels = ", ".join(label["name"] for label in issue.get("labels") or []) or "无"
    scope = "待确认"
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    code_context = code_context_summary(issue, comments)
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
        action = "评论 `/confirm`、`确认方案` 或 `开始处理` 后执行最小修复；如复现信息不足，执行时会先停止补问。"
    elif any(word in text for word in ("截图", "图片", "红框", "圈", "页面")):
        conclusion = "建议执行"
        confidence = "中"
        reason = "已有截图或页面位置补充，可以按当前范围做最小修复。"
        action = "评论 `/confirm`、`确认方案` 或 `开始处理` 后执行最小修复；未确认前继续只评论判断。"
    elif any(word in text for word in ("新增", "增加", "字段", "状态", "流程", "规则")):
        conclusion = "需要人工决策"
        confidence = "中"
        reason = "可能引入字段、状态、流程或业务规则变化，不能直接由 agent 拍板。"
        action = "先补业务边界和验收标准，再决定是否 `/confirm`。"
    return f"""{ANALYSIS_MARKER}
## WMS Issue Agent 判断

- Issue：#{issue["number"]} {issue["title"]}
- 作者：{(issue.get("user") or {}).get("login", "unknown")}
- 标签：{labels}
- 当前状态：open
- 可能影响范围：{scope}
- 结论：{conclusion}
- 置信度：{confidence}

### 问题复述

{short(issue.get("body") or issue.get("title") or "", 800)}

### 评论摘要

{comment_summary(comments)}

### 截图 / 附件

{attachment_summary(comments, issue=issue)}

### 代码核查

{code_context}

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

- 同意按上述范围执行：评论 `/confirm`、`确认方案` 或 `开始处理`
- 需要补充：直接继续评论需求细节或截图
- 暂不执行：评论 `/reject`
"""


def build_fix_prompt(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    return f"""请处理 Gitea issue #{issue["number"]}：{issue["title"]}

来源：{issue.get("html_url", "")}

用户已在 issue 评论中确认执行。请按 WMS 仓库规则处理：
1. 使用 `wms-loop-engineering` 定义目标、输入、检查、反馈和停止条件。
2. 使用 `wms-execution-retrospective` 从 issue 标题、正文、评论和附件中先判断是否存在共性问题；若是共性问题，必须同步补规则、脚本或矩阵，不能只修一个页面。
3. 当前 `codex exec` 应运行在 issue 专属 worktree 中；先运行 `pwd`、`git status --short --branch` 和 `git worktree list` 核对。必须使用 `wms-worktree-subagent` 或该独立 git worktree 隔离开发；禁止 checkout、修改或提交主工作区。
4. 只修复 issue 指向的问题。新增字段、状态、角色、模块或业务默认值时停止并向用户确认。
5. 完成后运行相关测试，至少 `git diff --check` 和 `just gov-t1`。
6. 涉及前端或用户可见行为时，禁止在子 worktree 中启动或占用 9002；9002 只允许主工作区固定会话 `wms-web-admin-9002`。需要真实截图或重启证据时，合并到主工作区后由主代理运行 `just dev-web-restart` 和 `just dev-web-verify`，后端按仓库现有运行方式启动并检查 `/healthz`。重启后必须校验运行的是本次修复版本：记录当前提交哈希，并用 `just dev-web-verify` 输出、页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存；把端口、URL、提交哈希和版本校验结果写入 PR 与 issue。
7. 如果 issue 评论包含截图 / 附件，必须先打开或下载附件辅助定位，不能只按文字猜测。
8. 涉及前端或用户可见行为时，必须采集真实前端截图；必须用 `POST /repos/{{owner}}/{{repo}}/issues/<编号>/assets` 把截图上传为 Gitea 附件，并用 Markdown 图片同时评论到 PR 和 issue；不能只写本地路径。
9. 当前可用截图 / 附件如下：
{attachment_summary(comments, issue=issue, limit=10)}
10. 读取 Gitea issue、评论、PR 或附件元数据必须使用 `tea api`，例如 `tea api /repos/{{owner}}/{{repo}}/issues/{issue["number"]}`；禁止裸 `curl` 访问 Gitea API。
11. 用户已授权：创建修复分支、推送到远端并创建 Gitea PR；禁止推 main，禁止强推，禁止自行合并 PR。
12. PR 创建不是完成。最后必须把 PR 链接、提交哈希、验证结果、截图证据、本地测试环境重启结果、tmux 任务会话状态、PR 合并前置条件和剩余风险评论回 issue，并明确下一步是“等待用户确认关闭 issue 后由 issue watcher 自动合并”或“阻塞”；子代理和主代理都不得直接合并 issue-agent PR。

Issue 正文：
{issue.get("body") or ""}

近期人工评论：
{comment_summary(comments, limit=10)}
"""


def choose_action(
    issue: dict[str, Any],
    comments: list[dict[str, Any]],
    *,
    confirm_token: str,
    confirm_label: str,
    force_dispatch: bool = False,
) -> Action | None:
    tmux_time = latest_marker_time(comments, TMUX_MARKER)
    human_time = latest_actionable_human_time(comments)
    analysis_time = latest_marker_time(comments, ANALYSIS_MARKER)
    status_time = latest_status_time(comments)
    confirmed = has_label(issue, confirm_label) or (
        analysis_time is not None and has_confirm_after(comments, analysis_time, confirm_token)
    )
    if status_time and not force_dispatch and not confirmed and (human_time is None or human_time <= status_time):
        return None
    if analysis_time is None:
        return Action(issue=issue, kind="analysis", body=build_analysis_comment(issue, comments))
    if has_reject_after(comments, analysis_time):
        return None
    if confirmed:
        if force_dispatch or tmux_time is None or tmux_time < analysis_time:
            return Action(issue=issue, kind="tmux", body=build_fix_prompt(issue, comments))
    if tmux_time and tmux_time > analysis_time:
        if human_time and human_time > tmux_time:
            return Action(issue=issue, kind="analysis", body=build_analysis_comment(issue, comments))
        return None
    if force_dispatch and has_label(issue, confirm_label):
        return Action(issue=issue, kind="tmux", body=build_fix_prompt(issue, comments))
    if human_time and human_time > analysis_time:
        return Action(issue=issue, kind="analysis", body=build_analysis_comment(issue, comments))
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


def build_codex_command(work_dir: Path, prompt_path: Path, log_path: Path, *, exec_mode: bool) -> str:
    if exec_mode:
        return (
            f"codex exec --dangerously-bypass-approvals-and-sandbox -C {quote(str(work_dir))} "
            f"- < {quote(str(prompt_path))} 2>&1 | tee {quote(str(log_path))}"
        )
    return (
        f"codex --dangerously-bypass-approvals-and-sandbox -C {quote(str(work_dir))} "
        f"\"$(cat {quote(str(prompt_path))})\""
    )


def inject_tmux(target: str, prompt: str) -> str:
    run(["tmux", "display-message", "-p", "-t", target, "#{session_name}:#{window_index}.#{pane_index}"])
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", delete=False) as f:
        f.write(prompt)
        prompt_path = f.name
    try:
        run(["tmux", "load-buffer", prompt_path])
        run(["tmux", "paste-buffer", "-t", target])
        run(["tmux", "send-keys", "-t", target, "Enter"])
        return target
    finally:
        Path(prompt_path).unlink(missing_ok=True)


def start_tmux_session(issue_number: int, prompt_path: Path, session_prefix: str, *, exec_mode: bool) -> str:
    stamp = datetime.now().strftime("%Y%m%d%H%M%S")
    session = f"{session_prefix}-{issue_number}-{stamp}"
    script = prompt_path.with_suffix(".run.sh")
    log_path = prompt_path.with_suffix(".log")
    work_dir, branch = issue_worktree(issue_number, stamp)
    run(["git", "worktree", "add", "-b", branch, str(work_dir), "HEAD"])
    command = build_codex_command(work_dir, prompt_path, log_path, exec_mode=exec_mode)
    script.write_text(
        "\n".join(
            [
                "#!/usr/bin/env bash",
                "set -euo pipefail",
                f"cd {quote(str(work_dir))}",
                command,
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    script.chmod(0o700)
    run(["tmux", "new-session", "-d", "-s", session, "-c", str(work_dir), str(script)])
    return session


def dispatch_tmux(
    *,
    issue_number: int,
    prompt: str,
    prompt_path: Path,
    mode: str,
    target: str,
    session_prefix: str,
) -> str:
    if mode == "paste":
        return inject_tmux(target, prompt)
    if mode == "exec":
        return start_tmux_session(issue_number, prompt_path, session_prefix, exec_mode=True)
    if mode == "session":
        return start_tmux_session(issue_number, prompt_path, session_prefix, exec_mode=False)
    raise ValueError(f"不支持的 tmux 投递模式：{mode}")


def execute_action(
    action: Action,
    *,
    apply: bool,
    out_dir: Path,
    tmux_target: str,
    tmux_mode: str,
    tmux_session_prefix: str,
) -> None:
    issue_number = int(action.issue["number"])
    preview = write_preview(out_dir, issue_number, action.kind, action.body)
    print(f"{action.kind}: issue #{issue_number} preview={display_path(preview)}", flush=True)
    if not apply:
        print("dry-run: 未评论 Gitea，未发送到 tmux", flush=True)
        return
    if action.kind == "analysis":
        post_comment(issue_number, action.body)
        print(f"applied: 已评论 issue #{issue_number}", flush=True)
        return
    destination = dispatch_tmux(
        issue_number=issue_number,
        prompt=action.body,
        prompt_path=preview,
        mode=tmux_mode,
        target=tmux_target,
        session_prefix=tmux_session_prefix,
    )
    post_comment(
        issue_number,
        f"{TMUX_MARKER}\n已通过 `{tmux_mode}` 发送到 tmux `{destination}`，等待 Codex 执行。",
    )
    print(f"applied: 已通过 {tmux_mode} 发送到 tmux {destination} 并评论 issue #{issue_number}", flush=True)


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
        action = choose_action(
            issue,
            comments,
            confirm_token=args.confirm_token,
            confirm_label=args.confirm_label,
            force_dispatch=getattr(args, "force_dispatch", False),
        )
        if action is not None:
            execute_action(
                action,
                apply=args.apply,
                out_dir=Path(args.out_dir),
                tmux_target=args.tmux_target,
                tmux_mode=args.tmux_mode,
                tmux_session_prefix=args.tmux_session_prefix,
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
    issue = {"number": 7, "title": "测试", "body": "正文", "labels": [], "user": {"login": "u"}}
    assert display_path(REPO_ROOT / "justfile") == "justfile"
    assert display_path(Path("/tmp/wms-issue-agent-preview.txt")) == "/tmp/wms-issue-agent-preview.txt"
    worktree_path, branch = issue_worktree(7, "20260702010101")
    assert worktree_path.name == "wms-agent-issue-7-20260702010101"
    assert branch == "fix/issue-7-20260702010101"
    command = build_codex_command(
        worktree_path,
        DEFAULT_OUT_DIR / "issue-7-tmux.txt",
        DEFAULT_OUT_DIR / "issue-7-tmux.log",
        exec_mode=True,
    )
    assert f"-C {quote(str(worktree_path))}" in command
    assert f"-C {quote(str(REPO_ROOT))} " not in command
    t0 = "2026-07-01T00:00:00Z"
    t1 = "2026-07-01T00:01:00Z"
    comments = [{"created_at": t0, "body": build_analysis_comment(issue, [])}]
    assert "### 代码核查" in comments[0]["body"]
    assert choose_action(issue, [], confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    assert choose_action(issue, comments, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    confirmed = [*comments, {"created_at": t1, "body": "/confirm"}]
    assert choose_action(issue, confirmed, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    confirmed_zh = [*comments, {"created_at": t1, "body": "确认方案"}]
    assert choose_action(issue, confirmed_zh, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    start_zh = [*comments, {"created_at": t1, "body": "开始处理"}]
    assert choose_action(issue, start_zh, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    mentioned = [*comments, {"created_at": t1, "body": "不要 /confirm，先补截图"}]
    assert choose_action(issue, mentioned, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    mentioned_zh = [*comments, {"created_at": t1, "body": "不要确认方案，先补截图"}]
    assert choose_action(issue, mentioned_zh, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    refreshed = [*mentioned, {"created_at": "2026-07-01T00:02:00Z", "body": build_analysis_comment(issue, mentioned)}]
    assert choose_action(issue, refreshed, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    sent = [*confirmed, {"created_at": "2026-07-01T00:02:00Z", "body": TMUX_MARKER}]
    assert choose_action(issue, sent, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    delivered = [
        *sent,
        {
            "created_at": "2026-07-01T00:03:00Z",
            "body": "已按确认处理 issue #7，最终交付 PR 为 #9。PR 合并前置条件：等待用户确认合并。",
        },
    ]
    assert choose_action(issue, delivered, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    after_delivery = [*delivered, {"created_at": "2026-07-01T00:04:00Z", "body": "补充：按钮还缺一个"}]
    assert choose_action(issue, after_delivery, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    after_tmux = [*sent, {"created_at": "2026-07-01T00:03:00Z", "body": "补充：还有下拉异常"}]
    assert choose_action(issue, after_tmux, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    after_tmux_refreshed = [
        *after_tmux,
        {"created_at": "2026-07-01T00:04:00Z", "body": build_analysis_comment(issue, after_tmux)},
    ]
    assert choose_action(issue, after_tmux_refreshed, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    after_tmux_confirmed = [*after_tmux_refreshed, {"created_at": "2026-07-01T00:05:00Z", "body": "/confirm"}]
    assert choose_action(issue, after_tmux_confirmed, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    assert choose_action(
        issue,
        sent,
        confirm_token="/confirm",
        confirm_label="codex:confirmed",
        force_dispatch=True,
    ).kind == "tmux"
    rejected = [*comments, {"created_at": t1, "body": "/reject 不执行 /confirm"}]
    assert choose_action(issue, rejected, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    labeled = {**issue, "labels": [{"name": "codex:confirmed"}]}
    assert choose_action(labeled, comments, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    with_asset = [
        *comments,
        {
            "created_at": t1,
            "body": "见截图",
            "user": {"login": "u"},
            "assets": [{"name": "image.png", "browser_download_url": "http://gitea/attachments/a"}],
        },
        {"created_at": "2026-07-01T00:02:00Z", "body": "/confirm"},
    ]
    prompt = choose_action(issue, with_asset, confirm_token="/confirm", confirm_label="codex:confirmed").body
    assert "http://gitea/attachments/a" in prompt
    assert "上传为 Gitea 附件" in prompt
    assert "wms-execution-retrospective" in prompt
    assert "共性问题" in prompt
    assert "tea api" in prompt
    assert "禁止裸 `curl`" in prompt
    issue_asset = {
        **issue,
        "assets": [{"name": "issue.png", "browser_download_url": "http://gitea/attachments/issue"}],
    }
    assert "http://gitea/attachments/issue" in build_analysis_comment(issue_asset, [])
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
    assert merge_blockers(closed_issue, [{"created_at": t1, "body": f"{ANALYSIS_MARKER}\n评论 `/reject`"}], pr) == []
    assert "存在阻塞或拒绝合并评论" in merge_blockers(
        closed_issue,
        [{"created_at": t1, "body": "不要合并"}],
        pr,
    )
    print("self-test: ok", flush=True)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Gitea issue → tmux Codex runner")
    sub = parser.add_subparsers(dest="command", required=True)

    def add_common(p: argparse.ArgumentParser) -> None:
        p.add_argument("--apply", action="store_true", help="执行写操作：评论 Gitea 或发送到 tmux")
        p.add_argument("--issue", type=int, help="只处理指定 issue 编号")
        p.add_argument("--limit", type=int, default=20, help="每轮最多扫描 open issue 数")
        p.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR), help="本地预览输出目录")
        p.add_argument("--tmux-target", default=os.getenv("WMS_ISSUE_AGENT_TMUX_TARGET", "wms-codex:0.0"))
        p.add_argument("--tmux-mode", choices=["exec", "session", "paste"], default="exec")
        p.add_argument("--tmux-session-prefix", default="wms-issue")
        p.add_argument("--confirm-token", default="/confirm")
        p.add_argument("--confirm-label", default="codex:confirmed")

    once = sub.add_parser("once", help="执行一轮扫描")
    add_common(once)
    once.add_argument("--force-dispatch", action="store_true", help="即使 issue 已有 tmux 标记，也重新投递一次")
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
