"""历史治理债务在 T4 strict 出口必须 fail closed。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_bounded_contexts_strict_blocks_missing_manifests(monkeypatch, capsys):
    import check_bounded_contexts as check

    monkeypatch.setattr(check, "load_manifests", lambda: [])

    assert check.main(["--json"]) == 0
    capsys.readouterr()
    assert check.main(["--strict", "--json"]) == 1


def test_bounded_contexts_current_manifest_scope_is_29():
    import check_bounded_contexts as check

    assert len(check.BOUNDED_CONTEXTS) == 29
    assert {"M1", "M2", "M3", "M4", "M5"} <= check.BOUNDED_CONTEXTS


def test_multi_end_strict_blocks_unclassified_core_module(monkeypatch, capsys):
    import check_multi_end_consistency as check

    stats = [
        check.StoryTagStats(
            file="user-stories-m2-inbound-asn",
            story_id="US-M2-001",
            total_ac=1,
        )
    ]
    monkeypatch.setattr(check, "scan_stories", lambda: stats)

    assert check.main(["--json"]) == 0
    capsys.readouterr()
    assert check.main(["--strict", "--json"]) == 1


def test_observability_strict_blocks_missing_core_kpi(monkeypatch, capsys):
    import check_observability as check

    modules = [check.ModuleKPI(file="user-stories-m2-inbound-asn")]
    monkeypatch.setattr(check, "scan_kpis", lambda: modules)

    assert check.main(["--json"]) == 0
    capsys.readouterr()
    assert check.main(["--strict", "--json"]) == 1


def test_observability_runtime_signals_require_metrics_and_trace_context(tmp_path):
    import check_observability as check

    (tmp_path / "backend").mkdir()
    (tmp_path / "backend" / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")

    issues = check.check_runtime_signals(tmp_path)

    assert {issue.rule for issue in issues} == {
        "metrics_endpoint_missing",
        "otel_dependency_missing",
        "instrumentation_missing",
        "trace_context_missing",
    }
