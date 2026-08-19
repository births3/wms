#!/usr/bin/env python3
"""WMS session idle closeout runner.

Skills do not run by themselves. This watcher checks local idle signals and,
when requested with --apply, starts a short codex exec closeout report.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = REPO_ROOT / ".codex" / "session-closeout"
STATE_FILE = OUT_DIR / "state.json"
ISSUE_AGENT_ENV_FILE = REPO_ROOT / ".codex" / "issue-agent" / "env"
REPORTS_DIR = OUT_DIR / "reports"
PROMPTS_DIR = OUT_DIR / "prompts"
LOGS_DIR = OUT_DIR / "logs"
CODEX_HOME = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex")))
CODEX_ACTIVITY_FILES = (
    CODEX_HOME / "history.jsonl",
    CODEX_HOME / "logs_2.sqlite",
    CODEX_HOME / "logs_2.sqlite-wal",
    CODEX_HOME / "state_5.sqlite",
    CODEX_HOME / "state_5.sqlite-wal",
)
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
EXCLUDED_DIRS = {
    ".git",
    ".codex",
    ".e2e-artifacts",
    ".pytest_cache",
    "__pycache__",
    "node_modules",
    "target",
    "dist",
    "build",
    "site",
    "test-results",
}
ACTIVE_PATTERNS = (
    re.compile(r"\bcodex\s+exec\b"),
    re.compile(r"\bpython3?\b.*scripts/(governance|agents)/(?!session_closeout_runner\.py)"),
    re.compile(r"\bjust\b.*\b(gov|check|verify|preflight|e2e|test|issue-agent)\b"),
    re.compile(r"\bpnpm\b.*\b(test|build|exec|playwright|tsc)\b"),
    re.compile(r"\bnode\b.*\b(playwright|tsc|vite build)\b"),
    re.compile(r"\bcargo\b.*\b(test|build|clippy|run)\b"),
    re.compile(r"\brustc\b"),
)
LONG_RUNNING_MARKERS = (
    "scripts/agents/issue_runner.py watch",
    "scripts/agents/session_closeout_runner.py watch",
    "cargo run --manifest-path backend/Cargo.toml -p wms-api --bin wms-api",
)


def now_ns() -> int:
    return time.time_ns()


def load_state() -> dict[str, Any]:
    try:
        return json.loads(STATE_FILE.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def save_state(state: dict[str, Any]) -> None:
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(state, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_codex_env(path: Path = ISSUE_AGENT_ENV_FILE) -> dict[str, str]:
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


def codex_exec_env() -> dict[str, str]:
    env = dict(os.environ)
    env.update(read_codex_env())
    return env


def path_is_under(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
    except ValueError:
        return False
    return True


def latest_repo_activity_ns() -> int:
    latest = int(REPO_ROOT.stat().st_mtime_ns)
    for root, dirs, files in os.walk(REPO_ROOT):
        dirs[:] = [name for name in dirs if name not in EXCLUDED_DIRS]
        root_path = Path(root)
        for name in files:
            path = root_path / name
            try:
                latest = max(latest, int(path.stat().st_mtime_ns))
            except OSError:
                continue
    return latest


def latest_codex_activity_ns() -> int:
    latest = 0
    for path in CODEX_ACTIVITY_FILES:
        try:
            latest = max(latest, int(path.stat().st_mtime_ns))
        except OSError:
            continue
    return latest


def latest_activity_ns() -> int:
    return max(latest_repo_activity_ns(), latest_codex_activity_ns())


def active_project_processes() -> list[str]:
    result = subprocess.run(
        ["ps", "-eo", "pid=,args="],
        capture_output=True,
        text=True,
        check=False,
    )
    active: list[str] = []
    self_pid = os.getpid()
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        pid_text, _, command = stripped.partition(" ")
        try:
            pid = int(pid_text)
        except ValueError:
            continue
        if pid == self_pid:
            continue
        if any(marker in command for marker in LONG_RUNNING_MARKERS):
            continue
        if not any(pattern.search(command) for pattern in ACTIVE_PATTERNS):
            continue
        try:
            cwd = Path(f"/proc/{pid}/cwd").resolve()
        except OSError:
            cwd = Path("/")
        if path_is_under(cwd, REPO_ROOT) or str(REPO_ROOT) in command:
            active.append(f"{pid} {command}")
    return active[:20]


def build_prompt(report_path: Path, idle_seconds: int, idle_for: int) -> str:
    return f"""请使用 wms-session-closeout 技能，对当前 WMS 项目最近几轮对话做停止前收口复盘。

要求：
- 只总结，不修改 tracked 文件，不提交，不推送。
- 把完整报告写入 `{report_path}`。
- 报告必须中文，按 wms-session-closeout 的输出格式组织。
- 重点总结：用户指出的偏差、没有检查出来的问题、测试断言层级不足、哪些经验能复用。
- 建议固化只写落点，不直接修改对应 skill / runbook / 脚本。

本次自动触发依据：
- 空闲阈值：{idle_seconds} 秒
- 已空闲：{idle_for} 秒
"""


def run_codex_closeout(prompt: str, *, prompt_path: Path, report_path: Path, log_path: Path, timeout: int) -> int:
    prompt_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    prompt_path.write_text(prompt, encoding="utf-8")
    result = subprocess.run(
        [
            "codex",
            "exec",
            "--dangerously-bypass-approvals-and-sandbox",
            "-C",
            str(REPO_ROOT),
            "-",
        ],
        input=prompt,
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
        env=codex_exec_env(),
        timeout=timeout,
        check=False,
    )
    log_path.write_text((result.stdout or "") + (result.stderr or ""), encoding="utf-8")
    if not report_path.exists():
        report_path.write_text(result.stdout or result.stderr or "codex exec 未生成收口报告。\n", encoding="utf-8")
    return int(result.returncode)


def run_once(args: argparse.Namespace) -> int:
    active = active_project_processes()
    if active:
        print("skip: project processes active", flush=True)
        for item in active:
            print(f"  {item}", flush=True)
        return 0

    activity_ns = latest_activity_ns()
    idle_for = max(0, int((now_ns() - activity_ns) / 1_000_000_000))
    if idle_for < args.idle_seconds:
        print(f"skip: idle {idle_for}s < {args.idle_seconds}s", flush=True)
        return 0

    state = load_state()
    if state.get("last_triggered_activity_ns") == activity_ns:
        print("skip: current idle period already reported", flush=True)
        return 0

    stamp = datetime.now().strftime("%Y%m%d%H%M%S")
    report_path = REPORTS_DIR / f"session-closeout-{stamp}.md"
    prompt_path = PROMPTS_DIR / f"session-closeout-{stamp}.txt"
    log_path = LOGS_DIR / f"session-closeout-{stamp}.log"
    prompt = build_prompt(report_path, args.idle_seconds, idle_for)
    if not args.apply:
        print(f"dry-run: would run closeout report={report_path}", flush=True)
        return 0

    code = run_codex_closeout(
        prompt,
        prompt_path=prompt_path,
        report_path=report_path,
        log_path=log_path,
        timeout=args.codex_timeout,
    )
    state.update(
        {
            "last_triggered_activity_ns": activity_ns,
            "last_report": str(report_path),
            "last_log": str(log_path),
            "last_exit_code": code,
            "last_triggered_at": datetime.now().isoformat(timespec="seconds"),
        }
    )
    save_state(state)
    print(f"closeout: exit={code} report={report_path} log={log_path}", flush=True)
    return code


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


def run_status(_args: argparse.Namespace) -> int:
    state = load_state()
    print(json.dumps(state, ensure_ascii=False, indent=2, sort_keys=True), flush=True)
    return 0


def self_test() -> int:
    env_file = Path(os.environ.get("WMS_SESSION_CLOSEOUT_TEST_ENV", "/tmp/wms-session-closeout-env"))
    env_file.write_text("export https_proxy=http://127.0.0.1:7894\nSECRET=x\n", encoding="utf-8")
    assert read_codex_env(env_file) == {"https_proxy": "http://127.0.0.1:7894"}
    prompt = build_prompt(Path(".codex/session-closeout/reports/demo.md"), 1800, 1900)
    assert "wms-session-closeout" in prompt
    assert "不修改 tracked 文件" in prompt
    assert path_is_under(REPO_ROOT / "scripts", REPO_ROOT)
    print("self-test: ok", flush=True)
    return 0


def add_common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--idle-seconds", type=int, default=1800, help="空闲多少秒后触发，默认 1800")
    parser.add_argument("--apply", action="store_true", help="实际运行 codex exec；默认只 dry-run")
    parser.add_argument("--codex-timeout", type=int, default=600, help="codex exec 最长运行秒数")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="WMS session closeout idle watcher")
    sub = parser.add_subparsers(dest="command", required=True)
    once = sub.add_parser("once", help="执行一轮空闲检查")
    add_common(once)
    once.set_defaults(func=run_once)
    watch = sub.add_parser("watch", help="循环空闲检查")
    add_common(watch)
    watch.add_argument("--interval", type=int, default=60, help="轮询间隔秒数")
    watch.add_argument("--max-iterations", type=int, default=0, help="最多轮询次数；0 表示一直运行")
    watch.set_defaults(func=run_watch)
    status = sub.add_parser("status", help="输出最近一次触发状态")
    status.set_defaults(func=run_status)
    test = sub.add_parser("self-test", help="运行内置测试")
    test.set_defaults(func=lambda _args: self_test())
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
