"""Matrix E2E 截图门禁配置与报告治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
def test_e2e_matrix_m4_manifest_requires_vertical_cjk_detection():
    """随货同行单必须启用中文表格可读性检查。"""
    import check_e2e_matrix_completeness as check

    manifest = {
        "snapshots": [
            {
                "tab": "m4-manifest",
                "url_hash": "#m4-manifest",
                "viewport": "1700x1700",
                "file": "m4-manifest.png",
                "expected_keywords": ["M4-005", "GSP"],
                "related_story": "US-M4-005",
            }
        ]
    }
    scenarios = {
        "defaults": {
            "required_selectors": ["header", "main"],
            "min_keyword_hit_ratio": 0.7,
            "max_horizontal_overflow_px": 8,
            "forbid_console_errors": True,
            "detect_text_overflow": True,
            "detect_control_overlap": True,
            "detect_vertical_cjk_table": False,
            "click_strategy": "first-main-button",
            "capture_states": ["initial", "after-interaction"],
        },
        "devices": {},
        "overrides": [
            {
                "name": "print",
                "match_globs": ["m4-manifest"],
                "detect_vertical_cjk_table": True,
            }
        ],
    }

    errors, _warnings = check.validate_matrix_config(manifest, scenarios)

    assert errors == []
    assert check.resolve_policy(scenarios, "m4-manifest")["detect_vertical_cjk_table"] is True


def test_matrix_e2e_report_rejects_partial_when_full_required():
    """full 门禁不能用 partial 报告冒充。"""
    import check_matrix_e2e_report as check

    payload = {
        "schema_version": 1,
        "mode": "partial",
        "expected_count": 1,
        "actual_count": 1,
        "passed_count": 1,
        "failed_count": 0,
        "missing_count": 0,
        "screenshot_missing_count": 0,
        "playwright_exit": 0,
        "results": [],
    }

    errors, _warnings = check.validate_report_payload(payload, require_full=True)

    assert any("mode=partial" in error for error in errors)
    assert any("expected_count" in error for error in errors)
