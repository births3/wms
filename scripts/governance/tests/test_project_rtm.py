import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_project_rtm as check


def seed_story(docs: Path) -> Path:
    domain = docs / "domain"
    domain.mkdir(parents=True)
    story = domain / "user-stories-m2.md"
    story.write_text("## US-M2-001：创建 ASN\n", encoding="utf-8")
    return story


def valid_doc() -> str:
    return """# 项目级 RTM 标准与索引

## 1. 维护原则

- 用户故事 ID 以 `docs/domain/user-stories-*.md` 为唯一来源。

## 2. RTM 分层

| RTM | 目的 | 维护位置 | 门禁 |
|---|---|---|---|
| 故事总 RTM | A | B | C |

## 3. 故事总 RTM

| 模块/能力 | 用户故事源 | 故事数量 | 当前 RTM |
|---|---|---:|---|
| M2 | [user-stories-m2.md](domain/user-stories-m2.md) | 1 | 后端实现 RTM |

## 4. 前端体验 RTM

| 范围 | 需求来源 | 前端入口 | 设计/截图证据 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|
| M2 | US-M2-001 | apps/web-admin | docs | 部分覆盖 | 真实截图缺失 | 补真实截图 |

## 5. 后端实现 RTM

| 范围 | 需求来源 | API / 契约 | Handler / Service | Domain / Repository / Migration | 测试 / 证据 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|---|---|
| M2 | US-M2-001 | backend/crates/api/src/lib.rs | backend/crates/api/src/inbound.rs | backend/crates/api/src/wave3_repository.rs | backend/crates/api/tests/wave3_postgres.rs | 已覆盖 | 无 | 保持同步 |

## 6. 测试证据 RTM

| 范围 | 需求来源 | 验证命令 | 证据对象 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|
| M2 | US-M2-001 | just gov-t1 | check_project_rtm.py | 已覆盖 | 无 | 保持同步 |

## 7. 合规风险 RTM

| 范围 | 需求来源 | 合规/风险来源 | 控制措施 | 证据对象 | 当前结论 | 缺口说明 | 补齐路径 |
|---|---|---|---|---|---|---|---|
| M2 | US-M2-001 | GSP | 审计 | evidence | 已覆盖 | 无 | 保持同步 |
"""


def test_project_rtm_accepts_required_matrices(tmp_path):
    docs = tmp_path / "docs"
    seed_story(docs)
    rtm = docs / "requirements-traceability-matrix.md"
    rtm.write_text(valid_doc(), encoding="utf-8")

    assert check.validate_doc(rtm, docs / "domain") == []


def test_project_rtm_rejects_unknown_story_id(tmp_path):
    docs = tmp_path / "docs"
    seed_story(docs)
    rtm = docs / "requirements-traceability-matrix.md"
    rtm.write_text(valid_doc().replace("US-M2-001 | apps/web-admin", "US-M2-999 | apps/web-admin"), encoding="utf-8")

    issues = check.validate_doc(rtm, docs / "domain")

    assert ("前端体验 RTM", "第 1 行未知用户故事编号: US-M2-999") in [
        (issue.rtm, issue.detail) for issue in issues
    ]


def test_project_rtm_requires_all_story_files_in_story_index(tmp_path):
    docs = tmp_path / "docs"
    seed_story(docs)
    (docs / "domain" / "user-stories-m3.md").write_text("## US-M3-001：库存查询\n", encoding="utf-8")
    rtm = docs / "requirements-traceability-matrix.md"
    rtm.write_text(valid_doc(), encoding="utf-8")

    issues = check.validate_doc(rtm, docs / "domain")

    assert ("故事总 RTM", "缺少用户故事文件引用: user-stories-m3.md") in [
        (issue.rtm, issue.detail) for issue in issues
    ]


def test_project_rtm_requires_backend_references(tmp_path):
    docs = tmp_path / "docs"
    seed_story(docs)
    rtm = docs / "requirements-traceability-matrix.md"
    rtm.write_text(valid_doc().replace("backend/", "server/"), encoding="utf-8")

    issues = check.validate_doc(rtm, docs / "domain")

    assert ("后端实现 RTM", "后端矩阵未引用 backend/ 路径") in [
        (issue.rtm, issue.detail) for issue in issues
    ]


def test_project_rtm_requires_gap_details_for_partial_rows(tmp_path):
    docs = tmp_path / "docs"
    seed_story(docs)
    rtm = docs / "requirements-traceability-matrix.md"
    rtm.write_text(
        valid_doc()
        .replace("真实截图缺失", "无")
        .replace("补真实截图", "-"),
        encoding="utf-8",
    )

    issues = check.validate_doc(rtm, docs / "domain")

    assert ("前端体验 RTM", "第 1 行为 部分覆盖 但缺口说明为空") in [
        (issue.rtm, issue.detail) for issue in issues
    ]
    assert ("前端体验 RTM", "第 1 行为 部分覆盖 但补齐路径为空") in [
        (issue.rtm, issue.detail) for issue in issues
    ]
