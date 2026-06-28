import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_system_dictionary_alignment as check


def write_valid_docs(root: Path) -> None:
    docs = root / "docs"
    domain = docs / "domain"
    domain.mkdir(parents=True)
    (domain / "user-stories-m1-master-data-warehouse.md").write_text(
        """# M1

## US-M1-011：系统字典中心

dict_code item_code param_schema JSONB params JSONB scope_mode global_only
owner_extensible owner_override override_policy effective_from effective_to
M-QL H2-005 M-PM document_type purchase_inbound sales_return
purchase_return_outbound sales_outbound direction workflow_template batch_policy
""",
        encoding="utf-8",
    )
    (docs / "m2-inbound-web-design-plan.md").write_text(
        """# M2

US-M1-011 document_type direction = inbound batch_policy
""",
        encoding="utf-8",
    )
    (docs / "m4-outbound-web-design-plan.md").write_text(
        """# M4

US-M1-011 document_type direction = outbound purchase_return_outbound
""",
        encoding="utf-8",
    )
    (docs / "requirements-traceability-matrix.md").write_text(
        """# RTM

US-M1-011 系统字典 check_system_dictionary_alignment.py
""",
        encoding="utf-8",
    )


def point_checker_to(root: Path, monkeypatch) -> None:
    monkeypatch.setattr(check, "REPO_ROOT", root)
    monkeypatch.setattr(
        check,
        "M1_STORY",
        root / "docs/domain/user-stories-m1-master-data-warehouse.md",
    )
    monkeypatch.setattr(check, "M2_PLAN", root / "docs/m2-inbound-web-design-plan.md")
    monkeypatch.setattr(check, "M4_PLAN", root / "docs/m4-outbound-web-design-plan.md")
    monkeypatch.setattr(
        check,
        "PROJECT_RTM",
        root / "docs/requirements-traceability-matrix.md",
    )
    monkeypatch.setattr(
        check,
        "REQUIRED_RTM_TERMS",
        {
            check.M2_PLAN: ("US-M1-011", "document_type", "direction = inbound", "batch_policy"),
            check.M4_PLAN: (
                "US-M1-011",
                "document_type",
                "direction = outbound",
                "purchase_return_outbound",
            ),
            check.PROJECT_RTM: (
                "US-M1-011",
                "系统字典",
                "check_system_dictionary_alignment.py",
            ),
        },
    )


def test_system_dictionary_alignment_accepts_current_repository_docs():
    assert check.validate() == []


def test_system_dictionary_alignment_rejects_missing_owner_override(tmp_path, monkeypatch):
    write_valid_docs(tmp_path)
    point_checker_to(tmp_path, monkeypatch)
    m1 = tmp_path / "docs/domain/user-stories-m1-master-data-warehouse.md"
    m1.write_text(
        m1.read_text(encoding="utf-8").replace("owner_override", "owner-default"),
        encoding="utf-8",
    )

    issues = check.validate()

    assert any(
        issue.file == "docs/domain/user-stories-m1-master-data-warehouse.md"
        and "owner_override" in issue.detail
        for issue in issues
    )


def test_system_dictionary_alignment_rejects_missing_m2_direction_rule(tmp_path, monkeypatch):
    write_valid_docs(tmp_path)
    point_checker_to(tmp_path, monkeypatch)
    m2 = tmp_path / "docs/m2-inbound-web-design-plan.md"
    m2.write_text(
        m2.read_text(encoding="utf-8").replace("direction = inbound", "direction = receiving"),
        encoding="utf-8",
    )

    issues = check.validate()

    assert any(
        issue.file == "docs/m2-inbound-web-design-plan.md"
        and "direction = inbound" in issue.detail
        for issue in issues
    )


def test_system_dictionary_alignment_json_payload_has_smoke_fields(capsys):
    assert check.main(["--json"]) == 0

    payload = capsys.readouterr().out

    assert '"check": "check_system_dictionary_alignment"' in payload
    assert '"tier": "T1"' in payload
    assert '"category": "文档治理"' in payload
    assert '"ok": true' in payload
