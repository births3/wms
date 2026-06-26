"""Wave 6 evidence preflight scope 边界词测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_evidence_preflight_forbidden_boundary_terms_include_production():
    """Wave 6 preflight 禁用边界必须同时覆盖 prod 与 production。"""
    import check_wave6_evidence_preflight as check

    assert "prod" in check.FORBIDDEN_BOUNDARY_TERMS
    assert "production" in check.FORBIDDEN_BOUNDARY_TERMS


def test_wave6_evidence_preflight_detects_scope_doc_prod_without_production(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 必须检查所有范围文档中的 prod / production 边界。"""
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", ())
    monkeypatch.setattr(check, "REQUIRED_EXECUTION_FILES", ())

    base_files = {
        check.PREFLIGHT_DOC: "\n".join([
            "just wave-6-evidence-preflight",
            "不会写入 runtime evidence",
            "不能关闭 gate",
            "environment",
            "dev",
            "staging",
            "local",
            "prod",
            "production",
            "mock",
            "fake",
            "stub",
            "example",
            "YYYY",
            "<...>",
            "TODO",
            "TBD",
            "待填",
            "待确认",
        ]),
        check.CLOSEOUT_DOC: f"just wave-6-evidence-preflight\n{check.PREFLIGHT_DOC}\n## 当前 Gate\n",
        check.TODO_DOC: "W6 evidence preflight",
        check.JUSTFILE: "wave-6-evidence-preflight:\n",
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": (
            "evidence 禁止 local/mock/fake/stub/example/prod/production"
        ),
        "ROADMAP.md": (
            "每个 evidence 引用不得用 localhost / stub / mock / fake / example / prod / production 替代"
        ),
        "docs/retros/wave-1-retro.md": (
            "runtime evidence 禁止用 localhost、stub、mock、fake、example、prod 或 production 证据替代"
        ),
    }
    for rel_path, text in base_files.items():
        path = tmp_path / rel_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    for boundary_doc in check.WAVE6_EVIDENCE_BOUNDARY_DOCS:
        clean_text = base_files.get(boundary_doc, "W6 evidence preflight")
        dirty_text = (
            "W6 evidence preflight\n"
            "Wave 6 runtime evidence 不能用 localhost / stub / mock / fake / example / prod 替代"
        )
        (tmp_path / boundary_doc).write_text(dirty_text, encoding="utf-8")

        top_errors, gate_results = check.collect_results()

        assert gate_results == []
        joined_errors = " ".join(top_errors)
        assert boundary_doc in joined_errors
        assert "production" in joined_errors

        (tmp_path / boundary_doc).write_text(clean_text, encoding="utf-8")


def test_wave6_evidence_preflight_ignores_scope_doc_prod_outside_evidence_context(
    tmp_path,
    monkeypatch,
):
    """Wave 6 preflight 不应把普通 prod 环境说明误报为 evidence 边界违规。"""
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)

    for boundary_doc in check.WAVE6_EVIDENCE_BOUNDARY_DOCS:
        path = tmp_path / boundary_doc
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "prod 环境用于发布窗口说明，不涉及 runtime evidence 或 gate 边界替代。",
            encoding="utf-8",
        )

    assert check.check_scope_boundary_docs() == []
