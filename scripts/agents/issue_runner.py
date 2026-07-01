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
ANALYSIS_MARKER = "<!-- wms-issue-agent:analysis:v2 -->"
TMUX_MARKER = "<!-- wms-issue-agent:tmux:v1 -->"


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


def has_label(issue: dict[str, Any], name: str) -> bool:
    return any(str(label.get("name")) == name for label in issue.get("labels") or [])


def has_command(text: str, command: str) -> bool:
    pattern = re.compile(rf"^\s*{re.escape(command)}(?:\s|$)")
    return any(pattern.match(line) for line in text.splitlines())


def has_confirm_after(comments: list[dict[str, Any]], since: datetime, token: str) -> bool:
    for comment in comments:
        created = parse_time(str(comment["created_at"]))
        text = body_of(comment)
        if created > since and (has_command(text, token) or has_command(text, "/codex run")):
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


def comment_summary(comments: list[dict[str, Any]], limit: int = 5) -> str:
    human_comments = [c for c in comments if "wms-issue-agent:" not in body_of(c)]
    if not human_comments:
        return "暂无人工评论。"
    lines: list[str] = []
    for c in human_comments[-limit:]:
        author = (c.get("user") or {}).get("login", "unknown")
        lines.append(f"- {author}: {short(body_of(c), 220)}")
    return "\n".join(lines)


def build_analysis_comment(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    labels = ", ".join(label["name"] for label in issue.get("labels") or []) or "无"
    scope = "待确认"
    text = f"{issue.get('title', '')} {issue.get('body', '')}"
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
        action = "评论 `/confirm` 后执行最小修复；如复现信息不足，执行时会先停止补问。"
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

### 判断依据

- {reason}
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

- 同意按上述范围执行：评论 `/confirm`
- 需要补充：直接继续评论需求细节或截图
- 暂不执行：评论 `/reject`
"""


def build_fix_prompt(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    return f"""请处理 Gitea issue #{issue["number"]}：{issue["title"]}

来源：{issue.get("html_url", "")}

用户已在 issue 评论中确认执行。请按 WMS 仓库规则处理：
1. 使用 `wms-loop-engineering` 定义目标、输入、检查、反馈和停止条件。
2. 必须使用 `wms-worktree-subagent` 或独立 git worktree 隔离开发；禁止直接修改当前主工作区。
3. 只修复 issue 指向的问题。新增字段、状态、角色、模块或业务默认值时停止并向用户确认。
4. 完成后运行相关测试，至少 `git diff --check` 和 `just gov-t1`。
5. 涉及前端或用户可见行为时，必须重启本地测试环境前端和后端；前端优先使用 `pnpm -C apps/web-admin dev --host 0.0.0.0 --port 9002 --strictPort`，后端按仓库现有运行方式启动并检查 `/healthz`。重启后必须校验运行的是本次修复版本：记录当前提交哈希，并用页面可见变更、接口响应版本字段或等价证据证明不是旧进程缓存；把端口、URL、提交哈希和版本校验结果写入 PR 与 issue。
6. 涉及前端或用户可见行为时，必须采集真实前端截图；截图路径、视口、页面 URL、截图结论必须同时评论到 PR 和 issue，不能只写本地路径。
7. 用户已授权：创建修复分支、推送到远端并创建 Gitea PR；禁止推 main，禁止强推。
8. 最后把 PR 链接、提交哈希、验证结果、截图证据、本地测试环境重启结果和剩余风险评论回 issue。

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
    if latest_marker_time(comments, TMUX_MARKER) and not force_dispatch:
        return None
    analysis_time = latest_marker_time(comments, ANALYSIS_MARKER)
    if analysis_time is None:
        return Action(issue=issue, kind="analysis", body=build_analysis_comment(issue, comments))
    if has_reject_after(comments, analysis_time):
        return None
    if has_label(issue, confirm_label) or has_confirm_after(comments, analysis_time, confirm_token):
        return Action(issue=issue, kind="tmux", body=build_fix_prompt(issue, comments))
    return None


def write_preview(out_dir: Path, issue_number: int, kind: str, body: str) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"issue-{issue_number}-{kind}.md"
    path.write_text(body, encoding="utf-8")
    return path


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
    session = f"{session_prefix}-{issue_number}-{datetime.now().strftime('%Y%m%d%H%M%S')}"
    script = prompt_path.with_suffix(".run.sh")
    log_path = prompt_path.with_suffix(".log")
    if exec_mode:
        command = (
            f"codex exec --dangerously-bypass-approvals-and-sandbox -C {quote(str(REPO_ROOT))} "
            f"- < {quote(str(prompt_path))} 2>&1 | tee {quote(str(log_path))}"
        )
    else:
        command = (
            f"codex --dangerously-bypass-approvals-and-sandbox -C {quote(str(REPO_ROOT))} "
            f"\"$(cat {quote(str(prompt_path))})\""
        )
    script.write_text(
        "\n".join(
            [
                "#!/usr/bin/env bash",
                "set -euo pipefail",
                f"cd {quote(str(REPO_ROOT))}",
                command,
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    script.chmod(0o700)
    run(["tmux", "new-session", "-d", "-s", session, "-c", str(REPO_ROOT), str(script)])
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
    print(f"{action.kind}: issue #{issue_number} preview={preview.relative_to(REPO_ROOT)}", flush=True)
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


def run_once(args: argparse.Namespace) -> int:
    issues = [get_issue(args.issue)] if args.issue else list_open_issues(args.limit)
    for issue in issues:
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
        except Exception as exc:  # noqa: BLE001
            print(f"watch-error: {exc}", file=sys.stderr, flush=True)
        count += 1
        if args.max_iterations and count >= args.max_iterations:
            return 0
        time.sleep(args.interval)


def self_test() -> int:
    issue = {"number": 7, "title": "测试", "body": "正文", "labels": [], "user": {"login": "u"}}
    t0 = "2026-07-01T00:00:00Z"
    t1 = "2026-07-01T00:01:00Z"
    comments = [{"created_at": t0, "body": build_analysis_comment(issue, [])}]
    assert choose_action(issue, [], confirm_token="/confirm", confirm_label="codex:confirmed").kind == "analysis"
    assert choose_action(issue, comments, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    confirmed = [*comments, {"created_at": t1, "body": "/confirm"}]
    assert choose_action(issue, confirmed, confirm_token="/confirm", confirm_label="codex:confirmed").kind == "tmux"
    mentioned = [*comments, {"created_at": t1, "body": "不要 /confirm，先补截图"}]
    assert choose_action(issue, mentioned, confirm_token="/confirm", confirm_label="codex:confirmed") is None
    sent = [*confirmed, {"created_at": "2026-07-01T00:02:00Z", "body": TMUX_MARKER}]
    assert choose_action(issue, sent, confirm_token="/confirm", confirm_label="codex:confirmed") is None
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
    watch.set_defaults(func=run_watch)

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
