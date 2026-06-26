"""Wave 6 evidence preflight scope 文档同步测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import COMMON_TERMS, PLACEHOLDER_TERMS, write_files


def scope_doc_fixture_files(check, scope_text: str) -> dict[str, str]:
    preflight_terms = [
        "just wave-6-evidence-preflight",
        "不会写入 runtime evidence",
        "不能关闭 gate",
        *COMMON_TERMS,
        *PLACEHOLDER_TERMS,
    ]
    return {
        check.PREFLIGHT_DOC: "\n".join([
            *preflight_terms,
            "W6.X",
            "docs/retros/wave-x-evidence.json",
            "wave-x-record",
            "wave-x-validate",
            "W6.Y",
            "docs/retros/wave-y-evidence.json",
            "wave-y-record",
            "wave-y-validate",
        ]),
        check.CLOSEOUT_DOC: "\n".join([
            "just wave-6-evidence-preflight",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
            "| W6.X | docs/retros/wave-x-evidence.json | wave-x-record | wave-x-validate |",
            "| W6.Y | docs/retros/wave-y-evidence.json | wave-y-record | wave-y-validate |",
        ]),
        check.TODO_DOC: f"W6 evidence preflight\n{scope_text}",
        check.JUSTFILE: "\n".join([
            "wave-6-evidence-preflight:",
            "wave-x-record:",
            "wave-x-validate:",
            "wave-y-record:",
            "wave-y-validate:",
        ]),
        "docs/architecture-dependencies.md": scope_text,
        "ROADMAP.md": scope_text,
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": scope_text,
        "docs/runbooks/wave-x-evidence.md": "\n".join([
            "docs/retros/wave-x-evidence.json",
            "wave-x-record",
            "wave-x-validate",
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
        ]),
        "docs/runbooks/wave-y-evidence.md": "\n".join([
            "docs/retros/wave-y-evidence.json",
            "wave-y-record",
            "wave-y-validate",
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
        ]),
    }


def test_wave6_evidence_preflight_detects_scope_doc_missing_gate(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须发现范围文档漏登记某个 W6 gate。"""
    import check_wave6_evidence_preflight as check

    gates = (
        check.GateSpec(
            "W6.X",
            "test gate X",
            "docs/runbooks/wave-x-evidence.md",
            "docs/retros/wave-x-evidence.json",
            ("wave-x-record", "wave-x-validate"),
        ),
        check.GateSpec(
            "W6.Y",
            "test gate Y",
            "docs/runbooks/wave-y-evidence.md",
            "docs/retros/wave-y-evidence.json",
            ("wave-y-record", "wave-y-validate"),
        ),
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", gates)
    monkeypatch.setattr(check, "REQUIRED_EXECUTION_FILES", ())

    scope_text = "W6.X W6.Y"
    base_files = scope_doc_fixture_files(check, scope_text)
    write_files(tmp_path, base_files)

    for scope_doc in check.WAVE6_SCOPE_GATE_DOCS:
        clean_text = base_files[scope_doc]
        (tmp_path / scope_doc).write_text(
            "Wave 6：预发布证据与外部依赖收口\nW6.X",
            encoding="utf-8",
        )

        top_errors, gate_results = check.collect_results()

        assert all(result.ok for result in gate_results)
        joined_errors = " ".join(top_errors)
        assert scope_doc in joined_errors
        assert "W6.Y" in joined_errors

        (tmp_path / scope_doc).write_text(clean_text, encoding="utf-8")


def test_wave6_evidence_preflight_ignores_scope_docs_without_wave6_gate(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 只检查已进入 W6 gate 语境的范围文档。"""
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    for scope_doc in check.WAVE6_SCOPE_GATE_DOCS:
        path = tmp_path / scope_doc
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("Wave 6 预发布收口说明，暂未展开具体 gate 清单。", encoding="utf-8")

    assert check.check_scope_gate_docs_sync() == []
