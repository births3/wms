"""核心逻辑单测：关键 helper 函数的边界行为

不依赖真实仓库内容，使用 fixture 构造测试输入。
"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_glossary_word_boundary_chinese():
    """禁用词紧贴中文 → 视为更长词的一部分（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("中") is True
    assert _is_word_char("位") is True


def test_glossary_word_boundary_alphanumeric():
    """禁用词紧贴英文/数字/下划线/连字符 → 视为复合词（不报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char("a") is True
    assert _is_word_char("Z") is True
    assert _is_word_char("0") is True
    assert _is_word_char("_") is True
    assert _is_word_char("-") is True


def test_glossary_word_boundary_separators():
    """空格 / 标点 → 视为词边界（应该报警）。"""
    from check_glossary_consistency import _is_word_char
    assert _is_word_char(" ") is False
    assert _is_word_char(",") is False
    assert _is_word_char("|") is False
    assert _is_word_char("。") is False
    assert _is_word_char("") is False


def test_check_data_appendix_loads():
    """check-data.toml 中的 appendix_references 能被加载。"""
    from _check_data import load_appendix_references
    refs = load_appendix_references()
    # Wave 0 至少有附录 A
    assert len(refs) >= 1
    appendix_a = next((r for r in refs if r.appendix == "附录 A"), None)
    assert appendix_a is not None
    assert appendix_a.defined_in == "user-stories-m1-master-data.md"
    assert "user-stories-m5-cold-chain.md" in appendix_a.expected_in


def test_check_data_exemptions_loads():
    """check-data.toml 中的 approval_source_exemptions 能被加载为 set。"""
    from _check_data import load_approval_source_exemptions
    exemptions = load_approval_source_exemptions()
    assert isinstance(exemptions, set)
    # Wave 0 至少有 M3-003（自身定义审批源）
    assert "US-M3-003" in exemptions
    # 至少 30 条
    assert len(exemptions) >= 25


def test_baseline_health_scan_empty_dir(tmp_path, monkeypatch):
    """baseline 目录为空时不应报错。"""
    import check_baseline_health as bh
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", tmp_path / "baseline-health.json")
    counts, issues = bh.scan_baselines()
    assert counts == {}
    assert issues == []


def test_baseline_health_growth_detection(tmp_path, monkeypatch):
    """baseline 数量超过历史上限 → 报错。"""
    import json
    import check_baseline_health as bh

    # 构造一个有 5 个 ignored 的 baseline 文件
    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [{"id": f"x{i}", "reason": "t", "added_at": "2026-01-01"} for i in range(5)],
    }), encoding="utf-8")

    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)

    snapshot = {"test_check": 3}  # 历史上限 3，当前 5 → 违规
    issues = bh.check_growth({"test_check": 5}, snapshot)
    assert len(issues) == 1
    assert issues[0].kind == "growth"


def test_baseline_health_expired_detection(tmp_path, monkeypatch):
    """expires_at 早于今天 + id 仍在 baseline → 报告过期。"""
    import json
    import check_baseline_health as bh

    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [
            {"id": "old1", "reason": "t", "added_at": "2020-01-01", "expires_at": "2020-06-01"},
        ],
    }), encoding="utf-8")
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    counts, issues = bh.scan_baselines()
    assert counts == {"test_check": 1}
    expired_issues = [i for i in issues if i.kind == "expired"]
    assert len(expired_issues) == 1


def test_baseline_health_default_does_not_write_snapshot(tmp_path, monkeypatch):
    """v0.4.1 行为：默认运行不应产生 snapshot 文件（避免 pre-commit 改 working tree）。"""
    import json
    import check_baseline_health as bh

    # 构造一个比 snapshot 更小的 baseline（理论上可触发自动收缩）
    snapshot_file = tmp_path / "baseline-health.json"
    snapshot_file.write_text(json.dumps({
        "version": 1, "max_counts": {"test_check": 5},
    }), encoding="utf-8")

    bl = tmp_path / "test_check.json"
    bl.write_text(json.dumps({
        "check": "test_check",
        "version": 1,
        "ignored": [{"id": "x", "reason": "t", "added_at": "2026-01-01"}],  # count=1 < 5
    }), encoding="utf-8")

    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", snapshot_file)

    snapshot_before = snapshot_file.read_text()
    bh.main([])  # 默认模式，无 --update-snapshot
    snapshot_after = snapshot_file.read_text()
    assert snapshot_before == snapshot_after, "默认运行不应修改 snapshot 文件"


def test_baseline_health_update_snapshot_writes(tmp_path, monkeypatch):
    """v0.4.1 行为：--update-snapshot 显式调用应写入 snapshot 文件。"""
    import check_baseline_health as bh

    snapshot_file = tmp_path / "baseline-health.json"
    monkeypatch.setattr(bh, "BASELINE_DIR", tmp_path)
    monkeypatch.setattr(bh, "SNAPSHOT_FILE", snapshot_file)

    assert not snapshot_file.exists()
    bh.main(["--update-snapshot"])
    assert snapshot_file.exists(), "--update-snapshot 应创建 snapshot 文件"


def test_governance_consistency_doc_parser():
    """check_governance_consistency 能正确解析 §4.6 表格。"""
    from check_governance_consistency import parse_doc_section
    scripts = parse_doc_section()
    # Wave 1+ 必需脚本
    assert "check_layer_dependency" in scripts
    assert scripts["check_layer_dependency"] == "T2"
    assert "check_unsafe_and_unwrap" in scripts
    # CI 全量的脚本不应被纳入（如 perf_baseline）
    assert "check_perf_baseline" not in scripts


def test_governance_consistency_gate_rules_parser():
    """check_governance_consistency 能正确解析 gate-rules.toml 占位规则。"""
    from check_governance_consistency import parse_gate_rules
    scripts = parse_gate_rules()
    # 已实现的脚本不应出现（如 check_doc_links）
    assert "check_doc_links" not in scripts
    # 占位脚本应出现
    assert "check_layer_dependency" in scripts


def test_commit_convention_valid_message():
    """合法的 conventional commit message 应通过。"""
    from check_commit_convention import validate_message
    msg = "功能(入库)：新增 ASN 状态机校验"
    issue = validate_message("abc1234", msg)
    assert issue.issues == []


def test_commit_convention_unknown_type():
    """未知 type 应报错。"""
    from check_commit_convention import validate_message
    msg = "未知类型(入库)：xxx"
    issue = validate_message("abc1234", msg)
    assert any("unknown type" in i for i in issue.issues)


def test_commit_convention_unknown_scope():
    """未知 scope 应报错。"""
    from check_commit_convention import validate_message
    msg = "功能(未知模块)：xxx"
    issue = validate_message("abc1234", msg)
    assert any("unknown scope" in i for i in issue.issues)


def test_commit_convention_too_long_header():
    """超长 header 应报错。"""
    from check_commit_convention import validate_message
    msg = "功能(入库)：" + "x" * 200
    issue = validate_message("abc1234", msg)
    assert any("too long" in i for i in issue.issues)


def test_environment_python_packages_check():
    """validate_environment 的 Python 包检查能跑（不抛异常）。"""
    from validate_environment import check_python_packages
    results = check_python_packages()
    # 至少检查 pathspec / markdown
    names = [r.name for r in results]
    assert "pathspec" in names
    assert "markdown" in names


def test_file_naming_rust_snake():
    """Rust 文件必须 snake_case。"""
    from check_file_naming import check_file
    assert check_file("backend/crates/api/src/main.rs") is None
    assert check_file("backend/crates/api/src/inbound_handler.rs") is None
    v = check_file("backend/crates/api/src/InboundHandler.rs")
    assert v is not None
    assert v.rule == "rust-file-snake"


def test_file_naming_adr():
    """ADR 必须 NNNN-slug.md。"""
    from check_file_naming import check_file
    assert check_file("docs/adr/0001-tech-stack.md") is None
    v = check_file("docs/adr/tech-stack.md")  # 缺数字编号
    assert v is not None
    assert v.rule == "adr-naming"


def test_file_naming_compliance():
    """合规文档必须 gsp-*.md 或 README.md。"""
    from check_file_naming import check_file
    assert check_file("docs/compliance/gsp-ch5-warehouse.md") is None
    assert check_file("docs/compliance/README.md") is None
    v = check_file("docs/compliance/random-doc.md")
    assert v is not None
    assert v.rule == "compliance-naming"
