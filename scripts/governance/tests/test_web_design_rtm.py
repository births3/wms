import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_web_design_rtm as check


def seed_story(docs: Path) -> None:
    (docs / "domain").mkdir()
    (docs / "domain" / "user-stories-m2.md").write_text(
        "## US-M2-001：创建 ASN\n",
        encoding="utf-8",
    )


def test_web_design_rtm_rejects_missing_action_state_evidence(tmp_path):
    docs = tmp_path / "docs"
    docs.mkdir()
    seed_story(docs)
    plan = docs / "m2-web-design-plan.md"
    plan.write_text(
        """# 方案

## 1 字段 RTM

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|
| A | B | US-M2-001 | D |
""",
        encoding="utf-8",
    )

    files, issues = check.validate_all(docs, tmp_path)

    assert files == [plan]
    assert {issue.rtm for issue in issues} == {"动作 RTM", "状态 RTM", "证据 RTM"}


def test_web_design_rtm_accepts_four_rtm_sections(tmp_path):
    docs = tmp_path / "docs"
    docs.mkdir()
    seed_story(docs)
    plan = docs / "m2-web-design-plan.md"
    plan.write_text(
        """# 方案

## 1 字段 RTM

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|
| A | B | US-M2-001 | D |

## 2 动作 RTM

| 动作 | 需求来源 | 前端入口 | API / 契约 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |

## 3 状态 RTM

| 状态流转 | 需求来源 | 触发动作 | 当前结论 |
|---|---|---|---|
| A | US-M2-001 | C | D |

## 4 证据 RTM

| 证据对象 | 需求来源 | 真实截图 | 动作验证 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |
""",
        encoding="utf-8",
    )

    _files, issues = check.validate_all(docs, tmp_path)

    assert issues == []


def test_web_design_rtm_rejects_table_without_data_rows(tmp_path):
    docs = tmp_path / "docs"
    docs.mkdir()
    seed_story(docs)
    plan = docs / "m2-web-design-plan.md"
    plan.write_text(
        """# 方案

## 1 字段 RTM

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|

## 2 动作 RTM

| 动作 | 需求来源 | 前端入口 | API / 契约 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |

## 3 状态 RTM

| 状态流转 | 需求来源 | 触发动作 | 当前结论 |
|---|---|---|---|
| A | US-M2-001 | C | D |

## 4 证据 RTM

| 证据对象 | 需求来源 | 真实截图 | 动作验证 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |
""",
        encoding="utf-8",
    )

    _files, issues = check.validate_all(docs, tmp_path)

    assert [(issue.rtm, issue.detail) for issue in issues] == [("字段 RTM", "缺少数据行")]


def test_web_design_rtm_requires_story_ids_in_source_column(tmp_path):
    docs = tmp_path / "docs"
    docs.mkdir()
    (docs / "domain").mkdir()
    (docs / "domain" / "user-stories-m2.md").write_text(
        "## US-M2-001：创建 ASN\n",
        encoding="utf-8",
    )
    plan = docs / "m2-web-design-plan.md"
    plan.write_text(
        """# 方案

## 1 字段 RTM

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|
| A | B | 业务需求 | D |

## 2 动作 RTM

| 动作 | 需求来源 | 前端入口 | API / 契约 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |

## 3 状态 RTM

| 状态流转 | 需求来源 | 触发动作 | 当前结论 |
|---|---|---|---|
| A | US-M2-001 | C | D |

## 4 证据 RTM

| 证据对象 | 需求来源 | 真实截图 | 动作验证 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |
""",
        encoding="utf-8",
    )

    _files, issues = check.validate_all(docs, tmp_path)

    assert ("字段 RTM", "第 1 行需求来源缺少用户故事编号") in [
        (issue.rtm, issue.detail) for issue in issues
    ]


def test_web_design_rtm_rejects_unknown_story_ids(tmp_path):
    docs = tmp_path / "docs"
    docs.mkdir()
    (docs / "domain").mkdir()
    (docs / "domain" / "user-stories-m2.md").write_text(
        "## US-M2-001：创建 ASN\n",
        encoding="utf-8",
    )
    plan = docs / "m2-web-design-plan.md"
    plan.write_text(
        """# 方案

## 1 字段 RTM

| 页面 | 字段 | 需求来源 | 契约 |
|---|---|---|---|
| A | B | US-M2-999 | D |

## 2 动作 RTM

| 动作 | 需求来源 | 前端入口 | API / 契约 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |

## 3 状态 RTM

| 状态流转 | 需求来源 | 触发动作 | 当前结论 |
|---|---|---|---|
| A | US-M2-001 | C | D |

## 4 证据 RTM

| 证据对象 | 需求来源 | 真实截图 | 动作验证 | 当前结论 |
|---|---|---|---|---|
| A | US-M2-001 | C | D | E |
""",
        encoding="utf-8",
    )

    _files, issues = check.validate_all(docs, tmp_path)

    assert ("字段 RTM", "第 1 行未知用户故事编号: US-M2-999") in [
        (issue.rtm, issue.detail) for issue in issues
    ]
