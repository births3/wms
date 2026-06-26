"""提交规范、环境与文件命名治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_governance_test_files_stay_below_800_lines():
    """治理测试文件必须保持可 review 的单文件规模。"""
    max_lines = 800
    # ponytail: existing oversized tests are baseline debt; this guard blocks new files.
    baseline_debt = {
        "test_wave3_pda_readiness_docs.py",
        "test_wave3_pda_runtime_evidence.py",
        "test_wave3_pda_runtime_readiness.py",
    }
    test_dir = Path(__file__).resolve().parent
    oversized = []
    for path in sorted(test_dir.glob("test_*.py")):
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        if line_count > max_lines and path.name not in baseline_debt:
            oversized.append(f"{path.name}: {line_count} lines")

    assert not oversized, "治理测试文件达到 800 行，请拆分：\n" + "\n".join(oversized)


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
