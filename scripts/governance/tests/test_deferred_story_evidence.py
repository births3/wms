"""延期故事证据门禁测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def complete_story(*, evidence_refs=None):
    story = {
        "id": "US-H9-001",
        "title": "打印模板",
        "reason": "等待真实运行证据",
        "resume_when": "补齐可复现证据",
        "frontend_pages": ["h9-print-templates"],
        "e2e_checks": ["pnpm --dir apps/web-admin run test:e2e:h9-dev"],
    }
    if evidence_refs is not None:
        story["evidence_refs"] = evidence_refs
    return story


def test_deferred_story_accepts_minimum_fields_and_existing_evidence(tmp_path):
    from check_deferred_story_evidence import scan_deferred_stories

    evidence = tmp_path / "evidence.txt"
    evidence.write_text("verified", encoding="utf-8")
    result = scan_deferred_stories(
        {"deferred_stories": [complete_story(evidence_refs=["evidence.txt"])]},
        repo_root=tmp_path,
    )

    assert result.ok
    assert result.story_count == 1
    assert result.issues == []


def test_deferred_story_reports_each_missing_required_field():
    from check_deferred_story_evidence import scan_deferred_stories

    result = scan_deferred_stories({"deferred_stories": [{"id": "US-M2-001"}]})

    assert not result.ok
    assert [issue.field for issue in result.issues] == [
        "title",
        "reason",
        "resume_when",
    ]


def test_pure_deferred_story_does_not_require_implementation_evidence():
    from check_deferred_story_evidence import scan_deferred_stories

    result = scan_deferred_stories(
        {
            "deferred_stories": [
                {
                    "id": "US-M2-001",
                    "title": "创建 ASN",
                    "reason": "尚未进入实施波次",
                    "resume_when": "资源就绪后恢复",
                }
            ]
        }
    )

    assert result.ok


def test_frontend_slice_requires_frontend_page_registration():
    from check_deferred_story_evidence import scan_deferred_stories

    story = complete_story()
    story.pop("frontend_pages")
    story["types"] = ["frontend_interaction"]

    result = scan_deferred_stories({"deferred_stories": [story]})

    assert [issue.field for issue in result.issues] == ["frontend_pages"]


def test_implemented_slice_requires_at_least_one_runnable_check():
    from check_deferred_story_evidence import scan_deferred_stories

    story = complete_story()
    story.pop("e2e_checks")
    story["api_paths"] = ["POST /api/v1/inbound/receiving-orders"]

    result = scan_deferred_stories({"deferred_stories": [story]})

    assert [issue.field for issue in result.issues] == ["test_checks/e2e_checks"]


def test_deferred_story_accepts_test_checks_without_e2e_checks():
    from check_deferred_story_evidence import scan_deferred_stories

    story = complete_story()
    story.pop("e2e_checks")
    story["test_checks"] = ["python3 -m pytest scripts/governance/tests -q"]

    assert scan_deferred_stories({"deferred_stories": [story]}).ok


def test_deferred_story_reports_missing_and_outside_evidence_paths(tmp_path):
    from check_deferred_story_evidence import scan_deferred_stories

    result = scan_deferred_stories(
        {
            "deferred_stories": [
                complete_story(evidence_refs=["missing.txt", "../outside.txt", "/tmp/absolute.txt"])
            ]
        },
        repo_root=tmp_path,
    )

    assert [issue.field for issue in result.issues] == ["evidence_refs", "evidence_refs", "evidence_refs"]
    assert all("证据路径不存在或不在仓库内" in issue.message for issue in result.issues)


def test_deferred_story_scan_does_not_modify_matrix():
    from check_deferred_story_evidence import scan_deferred_stories

    matrix = {"deferred_stories": [complete_story()]}
    before = repr(matrix)

    scan_deferred_stories(matrix)

    assert repr(matrix) == before


def test_deferred_story_does_not_silently_skip_non_object_entries():
    from check_deferred_story_evidence import scan_deferred_stories

    result = scan_deferred_stories({"deferred_stories": ["not-a-story"]})

    assert result.story_count == 1
    assert result.issues[0].field == "story"
    assert result.issues[0].message == "延期故事必须是对象"
