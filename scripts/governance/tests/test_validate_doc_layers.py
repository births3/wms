"""validate_doc_layers 治理检查测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_l3_domain_code_sync_skips_story_and_support_docs(tmp_path, monkeypatch):
    """用户故事、澄清和遗留对比文档不要求存在同名后端路径。"""
    import validate_doc_layers

    docs_domain = tmp_path / "docs" / "domain"
    backend = tmp_path / "backend"
    docs_domain.mkdir(parents=True)
    backend.mkdir()
    (backend / "some_real_file.rs").write_text("", encoding="utf-8")

    for name in [
        "user-stories-m2-inbound-asn.md",
        "clarifications.md",
        "todo-legacy-comparison.md",
    ]:
        (docs_domain / name).write_text("# test\n", encoding="utf-8")

    monkeypatch.setattr(validate_doc_layers, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(validate_doc_layers, "DOCS_DIR", tmp_path / "docs")

    issues = []
    validate_doc_layers.check_l3_domain_code_sync(issues)

    assert issues == []


def test_l3_domain_code_sync_keeps_domain_design_warning(tmp_path, monkeypatch):
    """真正的领域设计文档仍执行后端路径弱校验。"""
    import validate_doc_layers

    docs_domain = tmp_path / "docs" / "domain"
    backend = tmp_path / "backend"
    docs_domain.mkdir(parents=True)
    backend.mkdir()
    (backend / "some_real_file.rs").write_text("", encoding="utf-8")
    (docs_domain / "inventory-policy.md").write_text("# test\n", encoding="utf-8")

    monkeypatch.setattr(validate_doc_layers, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(validate_doc_layers, "DOCS_DIR", tmp_path / "docs")

    issues = []
    validate_doc_layers.check_l3_domain_code_sync(issues)

    assert len(issues) == 1
    assert issues[0].file == "docs/domain/inventory-policy.md"
