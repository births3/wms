"""Gitea comment helpers for issue_runner."""
import json
import re
from datetime import datetime
from pathlib import Path
from typing import Any

from issue_runner import *  # noqa: F403

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
