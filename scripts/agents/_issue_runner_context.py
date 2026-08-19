"""Issue context summaries for issue_runner."""
from typing import Any

from issue_runner import *  # noqa: F403

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

def commonality_summary(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    text = f"{issue.get('title', '')} {issue.get('body', '')} {comment_summary(comments, limit=10)}"
    matched: list[tuple[str, str, str]] = []
    for words, category, scope, action in COMMONALITY_RULES:
        if any(word in text for word in words):
            matched.append((category, scope, action))
    if any(category == "DataGrid 选择状态一致性" for category, _, _ in matched):
        matched = [item for item in matched if item[0] != "管理端动作入口一致性"]
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

def has_selection_state_signal(text: str) -> bool:
    return any(
        word in text
        for word in (
            "全局勾选",
            "取消勾选",
            "自动勾选",
            "勾选",
            "全选",
            "选中",
            "第一个",
            "selectedRowKeys",
        )
    )
