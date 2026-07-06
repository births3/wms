"""页面行数治理测试。"""
import sys
from pathlib import Path

TEST_DIR = Path(__file__).resolve().parent
SCRIPTS_DIR = TEST_DIR.parent
sys.path.insert(0, str(SCRIPTS_DIR))


def test_governance_test_files_stay_below_800_lines():
    """治理测试文件必须保持可 review 的单文件规模。"""
    max_lines = 800
    # ponytail: existing oversized tests are baseline debt; only shrinking or holding is allowed.
    baseline_debt = {
        "test_wave3_pda_readiness_docs.py": 867,
        "test_wave3_pda_runtime_evidence.py": 1262,
        "test_wave3_pda_runtime_readiness.py": 2966,
    }
    oversized = []
    for path in sorted(TEST_DIR.glob("test_*.py")):
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        allowed = baseline_debt.get(path.name, max_lines)
        if line_count > allowed:
            oversized.append(f"{path.name}: {line_count} lines")

    assert not oversized, "治理测试文件达到 800 行，请拆分：\n" + "\n".join(oversized)


def test_check_page_size_scans_production_web_admin_pages():
    """生产 web-admin 页面也必须受页面行数门禁保护。"""
    import check_page_size

    page = Path("apps/web-admin/src/pages/__page_size_probe.tsx")
    page.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(
        f"export const probe{i} = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD)
    ) + "\n"

    try:
        page.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        page.unlink(missing_ok=True)

    assert any(page.as_posix() in error for error in errors)


def test_check_page_size_scans_shared_business_components():
    """共享业务组件也必须受行数门禁保护。"""
    import check_page_size

    component = Path("packages/ui/src/business/__page_size_probe.tsx")
    component.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(
        f"export const probe{i} = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD)
    ) + "\n"

    try:
        component.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        component.unlink(missing_ok=True)

    assert any(component.as_posix() in error for error in errors)


def test_production_page_changes_trigger_page_size_gate():
    """生产前端页面变更必须触发页面行数门禁。"""
    from _diff import load_gate_rules, match_rules

    rules = load_gate_rules()
    for changed_file in [
        "apps/web-admin/src/pages/auth/LoginPage.tsx",
        "apps/pda-mobile/src/pages/ScanPage.tsx",
    ]:
        triggered = match_rules([changed_file], rules)
        assert "check_page_size" in triggered


def test_shared_business_component_changes_trigger_page_size_gate():
    """共享业务组件变更必须触发行数门禁。"""
    from _diff import load_gate_rules, match_rules

    rules = load_gate_rules()
    triggered = match_rules(["packages/ui/src/business/DataGrid/DataGrid.tsx"], rules)

    assert "check_page_size" in triggered
