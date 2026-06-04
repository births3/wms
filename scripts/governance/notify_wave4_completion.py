#!/usr/bin/env python3
"""Send Wave 4 completion notification only after the strict gate passes."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import urllib.request
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_WEBHOOK_ENV = "WAVE4_COMPLETION_WEBHOOK_URL"
DEFAULT_MESSAGE = "WMS Wave 4 已完成：just wave-4-complete-check 已通过。"


@dataclass(frozen=True)
class CompletionCheck:
    ok: bool
    output: str


def run_completion_check() -> CompletionCheck:
    completed = subprocess.run(
        ["python3", "scripts/governance/report_wave4_completion.py", "--strict"],
        cwd=REPO_ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return CompletionCheck(
        ok=completed.returncode == 0,
        output=completed.stdout.strip(),
    )


def build_qy_wechat_payload(content: str) -> bytes:
    return json.dumps(
        {
            "msgtype": "text",
            "text": {
                "content": content,
            },
        },
        ensure_ascii=False,
    ).encode("utf-8")


def post_qy_wechat_webhook(
    webhook_url: str,
    content: str,
    *,
    timeout_seconds: float,
) -> tuple[int, str]:
    request = urllib.request.Request(
        webhook_url,
        data=build_qy_wechat_payload(content),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout_seconds) as response:  # noqa: S310
        body = response.read().decode("utf-8", errors="replace")
        return response.status, body


def response_is_ok(status_code: int, body: str) -> bool:
    if status_code < 200 or status_code >= 300:
        return False
    try:
        payload = json.loads(body)
    except json.JSONDecodeError:
        return True
    if not isinstance(payload, dict):
        return True
    return payload.get("errcode", 0) == 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--webhook-env",
        default=DEFAULT_WEBHOOK_ENV,
        help="Environment variable containing the webhook URL.",
    )
    parser.add_argument("--message", default=DEFAULT_MESSAGE)
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Validate the completion gate and payload without sending.",
    )
    args = parser.parse_args(argv)

    completion = run_completion_check()
    if not completion.ok:
        print("Wave 4 未完成，跳过 webhook。")
        if completion.output:
            print(completion.output)
        return 0

    webhook_url = os.environ.get(args.webhook_env, "").strip()
    if not webhook_url:
        print(
            f"Wave 4 已完成，但未设置 {args.webhook_env}，未发送 webhook。",
            file=sys.stderr,
        )
        return 1

    if args.dry_run:
        print("Wave 4 已完成；dry-run 跳过实际 webhook。")
        print(build_qy_wechat_payload(args.message).decode("utf-8"))
        return 0

    try:
        status_code, body = post_qy_wechat_webhook(
            webhook_url,
            args.message,
            timeout_seconds=args.timeout_seconds,
        )
    except Exception as error:  # noqa: BLE001
        print(f"webhook 发送失败: {error}", file=sys.stderr)
        return 1

    if not response_is_ok(status_code, body):
        print(f"webhook 返回异常: HTTP {status_code} {body}", file=sys.stderr)
        return 1

    print("Wave 4 已完成，企微 webhook 已发送。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
