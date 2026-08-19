"""task_check 兜底与机器可读输出契约。"""

import json
import sys
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS_DIR))


def test_unknown_changed_path_runs_t1_fallback(monkeypatch, capsys):
    import task_check

    monkeypatch.setattr(task_check, "load_gate_rules", lambda: [])
    monkeypatch.setattr(task_check, "get_changed_files", lambda **_kwargs: ["mkdocs.yml"])
    monkeypatch.setattr(task_check, "match_rules", lambda _changed, _rules: {})
    monkeypatch.setattr(
        task_check,
        "run_t1_fallback",
        lambda json_mode: task_check.ScriptResult("governance_t1_fallback", 1, 0, 1),
    )

    assert task_check.main(["--tier", "T2", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["triggered"][0]["name"] == "governance_t1_fallback"


def test_t1_catch_all_rule_does_not_suppress_full_fallback(monkeypatch, capsys):
    import task_check
    from _diff import GateRule

    monkeypatch.setattr(
        task_check,
        "load_gate_rules",
        lambda: [GateRule("**", ["validate_environment"], "T1")],
    )
    monkeypatch.setattr(task_check, "get_changed_files", lambda **_kwargs: ["unknown/new.file"])
    monkeypatch.setattr(
        task_check,
        "run_t1_fallback",
        lambda json_mode: task_check.ScriptResult("governance_t1_fallback", 1, 0, 1),
    )
    monkeypatch.setattr(
        task_check,
        "run_one",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("catch-all must not run alone")),
    )

    assert task_check.main(["--tier", "T1", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["triggered"][0]["name"] == "governance_t1_fallback"


def test_mixed_known_and_unknown_paths_run_specific_check_and_fallback(monkeypatch, capsys):
    import task_check
    from _diff import GateRule

    monkeypatch.setattr(
        task_check,
        "get_changed_files",
        lambda **_kwargs: ["known/change.ts", "unknown/new.file"],
    )
    monkeypatch.setattr(
        task_check,
        "run_t1_fallback",
        lambda json_mode: task_check.ScriptResult("governance_t1_fallback", 0, 0, 1),
    )
    monkeypatch.setattr(
        task_check,
        "run_one",
        lambda name, **_kwargs: task_check.ScriptResult(name, 0, 0, 1),
    )

    for tier in ("T1", "T2"):
        monkeypatch.setattr(
            task_check,
            "load_gate_rules",
            lambda tier=tier: [GateRule("known/**", ["check_known"], tier)],
        )
        assert task_check.main(["--tier", tier, "--json"]) == 0
        payload = json.loads(capsys.readouterr().out)
        assert [item["name"] for item in payload["triggered"]] == [
            "governance_t1_fallback",
            "check_known",
        ]
        assert payload["triggered"][0]["matched_files"] == 1


def test_json_mode_emits_one_aggregate_document(monkeypatch, capsys):
    import task_check

    monkeypatch.setattr(task_check, "load_gate_rules", lambda: [])
    monkeypatch.setattr(task_check, "get_changed_files", lambda **_kwargs: ["docs/error-codes.md"])
    monkeypatch.setattr(
        task_check,
        "match_rules",
        lambda _changed, _rules: {"check_error_codes": ["docs/error-codes.md"]},
    )
    monkeypatch.setattr(
        task_check,
        "run_one",
        lambda *_args, **_kwargs: task_check.ScriptResult("check_error_codes", 0, 0, 1),
    )
    monkeypatch.setattr(
        task_check,
        "run_t1_fallback",
        lambda _json_mode: task_check.ScriptResult("governance_t1_fallback", 0, 0, 1),
    )

    assert task_check.main(["--tier", "T1", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is True
    assert len(payload["triggered"]) == 1
