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
import socket
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
WORKTREE_WEB_PORT_RANGE = range(9003, 9100)
WORKTREE_API_PORT_RANGE = range(18081, 18100)
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
LOCAL_BRANCH_PATTERN = re.compile(r"(?<![\w./-])(?:fix/issue-\d+-[A-Za-z0-9._/-]+|agent/[A-Za-z0-9._/-]+)")
LOCAL_MERGE_VALIDATIONS = (
    ("git", "diff", "--check"),
    ("git", "diff", "--cached", "--check"),
    ("just", "gov-t1"),
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


from _issue_runner_comments import *  # noqa: F403

































































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
    "全局勾选",
    "取消勾选",
    "自动勾选",
    "勾选",
    "全选",
    "选中",
    "第一个",
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
    "供应商": ["m1-business-partners", "supplier"],
    "客户": ["m1-business-partners", "customer"],
    "来源": ["sourceValue", "source"],
    "批量导入": ["批量导入", "import"],
    "新建供应商": ["createSupplier", "新建供应商"],
    "新建客户": ["createCustomer", "新建客户"],
    "全局勾选": ["updatePageSelected", "onPageSelected", "selectedRowKeys"],
    "取消勾选": ["updatePageSelected", "selectedRowKeys", "onSelectedRowKeysChange"],
    "自动勾选": ["selectedRowKeys", "onSelectedRowKeysChange", "selectFirst"],
    "勾选": ["selectedRowKeys", "onSelectedRowKeysChange", "selectable"],
    "全选": ["updatePageSelected", "selectedRowKeys", "onPageSelected"],
    "选中": ["selectedRowKeys", "onSelectedRowKeysChange", "selectedRowFrom"],
    "第一个": ["keys.at(-1)", "selectedRowKeys", "selectedRowFrom"],
    "弹窗": ["Dialog", "Popover", "Modal"],
    "窗口": ["Dialog", "Popover", "Modal"],
}


from _issue_runner_context import *  # noqa: F403





COMMONALITY_RULES = [
    (
        ("全局勾选", "取消勾选", "自动勾选", "勾选", "全选", "选中", "第一个", "selectedRowKeys"),
        "DataGrid 选择状态一致性",
        "管理端 DataGrid 表头全选、行勾选、详情焦点和页面自动首选逻辑",
        "先追踪 selectedRowKeys / onSelectedRowKeysChange 的状态所有者，再查页面是否把多选数组压成单选或自动回填第一条；不能只按按钮入口泛搜。",
    ),
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






from _issue_runner_merge import *  # noqa: F403







































from _issue_runner_proposal import *  # noqa: F403













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


def port_in_use(port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.2)
        return sock.connect_ex(("127.0.0.1", port)) == 0


def first_free_port(ports: range) -> int:
    for port in ports:
        if not port_in_use(port):
            return port
    raise RuntimeError(f"没有可用端口：{ports.start}-{ports.stop - 1}")


def append_runtime_context(prompt_path: Path, work_dir: Path, web_port: int, api_port: int) -> None:
    with prompt_path.open("a", encoding="utf-8") as f:
        f.write(
            "\n\n"
            "本轮 issue-agent 运行时分配：\n"
            f"- worktree：`{work_dir}`\n"
            f"- WMS_ISSUE_WEB_PORT={web_port}\n"
            f"- WMS_ISSUE_API_PORT={api_port}\n"
            f"- 前端预览命令：`just dev-web-worktree-restart {work_dir} {web_port}`\n"
            f"- 前端校验命令：`just dev-web-worktree-verify {work_dir} {web_port}`\n"
            f"- 后端联调命令：`just dev-api-worktree-restart {work_dir} {api_port}`\n"
            f"- 后端校验命令：`just dev-api-worktree-verify {work_dir} {api_port}`\n"
        )


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


def prepare_codex_exec(issue_number: int, prompt_path: Path) -> tuple[Path, str, Path, Path, int, int]:
    stamp = datetime.now().strftime("%Y%m%d%H%M%S")
    script = prompt_path.with_suffix(".run.sh")
    log_path = prompt_path.with_suffix(".log")
    work_dir, branch = issue_worktree(issue_number, stamp)
    run(["git", "worktree", "add", "-b", branch, str(work_dir), "HEAD"])
    web_port = first_free_port(WORKTREE_WEB_PORT_RANGE)
    api_port = first_free_port(WORKTREE_API_PORT_RANGE)
    append_runtime_context(prompt_path, work_dir, web_port, api_port)
    command = build_codex_command(work_dir, prompt_path, log_path)
    script.write_text(
        "\n".join(
            [
                "#!/usr/bin/env bash",
                "set -euo pipefail",
                *codex_env_exports(),
                f"export WMS_ISSUE_WEB_PORT={web_port}",
                f"export WMS_ISSUE_API_PORT={api_port}",
                f"cd {quote(str(work_dir))}",
                command,
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    script.chmod(0o700)
    return work_dir, branch, script, log_path, web_port, api_port


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
    work_dir, branch, script, log_path, web_port, api_port = prepare_codex_exec(issue_number, preview)
    post_comment(
        issue_number,
        (
            f"{EXEC_MARKER}\n"
            "已开始直接运行 `codex exec`，等待 Codex 执行并回写 delivery 证据。\n"
            f"- worktree：`{work_dir}`\n"
            f"- 分支：`{branch}`\n"
            f"- 前端预览端口：`{web_port}`\n"
            f"- 后端联调端口：`{api_port}`（仅改后端 / API / 数据库时使用；纯前端共用 18080）\n"
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
            if args.local_merge_closed:
                run_local_merge_closed(args)
            if args.merge_closed:
                run_merge_closed(args)
        except Exception as exc:  # noqa: BLE001
            print(f"watch-error: {exc}", file=sys.stderr, flush=True)
        count += 1
        if args.max_iterations and count >= args.max_iterations:
            return 0
        time.sleep(args.interval)


from _issue_runner_selftest import self_test



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
    watch.add_argument("--local-merge-closed", action="store_true", help="同时扫描已关闭 issue 并合并满足闭环条件的本地分支")
    watch.add_argument("--merge-closed", action="store_true", help="同时扫描已关闭 issue 并合并满足闭环条件的 PR")
    watch.add_argument("--pr-limit", type=int, default=20, help="每轮最多扫描 open PR 数")
    watch.set_defaults(func=run_watch)

    local_merge_closed = sub.add_parser("local-merge-closed", help="扫描已关闭 issue，并合并满足闭环条件的本地分支")
    local_merge_closed.add_argument("--apply", action="store_true", help="执行本地合并并评论；默认只 dry-run")
    local_merge_closed.add_argument("--issue", type=int, help="只处理指定 issue 编号")
    local_merge_closed.add_argument("--limit", type=int, default=20, help="每轮最多扫描 closed issue 数")
    local_merge_closed.set_defaults(func=run_local_merge_closed)

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
