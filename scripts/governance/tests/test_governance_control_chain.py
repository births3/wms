"""轻量 G1-G4 治理控制链契约测试。"""

import importlib
import json
import sys
from pathlib import Path
from types import SimpleNamespace


REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPTS_DIR = REPO_ROOT / "scripts" / "governance"
sys.path.insert(0, str(SCRIPTS_DIR))


def test_gate_config_exposes_lightweight_control_chain():
    from _diff import load_gate_config

    config = load_gate_config()

    assert config.model.layers == ["G1", "G2", "G3", "G4"]
    assert config.model.rule_verdicts == ["pass", "fail"]
    assert config.model.execution_statuses == ["passed", "failed", "error", "blocked"]
    assert config.model.allowed_contexts == ["local", "pr", "main", "release", "runtime"]
    assert config.rules
    assert all(rule.rule_ids for rule in config.rules)
    assert all(rule.source for rule in config.rules)
    assert all(rule.contexts for rule in config.rules)
    assert (REPO_ROOT / config.model.default_source).is_file()


def test_pr_context_excludes_external_runtime_evidence():
    from _diff import load_gate_rules, rules_for_execution

    rules = load_gate_rules()
    pr_rules = rules_for_execution(rules, tier="T1", context="pr")
    release_rules = rules_for_execution(rules, tier="T1", context="release")

    target = "docs/retros/wave-6-deploy-evidence.json"
    assert not any(
        rule.matches(target) and "validate_wave6_deploy_evidence" in rule.checks
        for rule in pr_rules
    )
    assert any(
        rule.matches(target) and "validate_wave6_deploy_evidence" in rule.checks
        for rule in release_rules
    )


def test_higher_contexts_keep_lower_tier_diff_gates():
    from _diff import load_gate_rules, rules_for_execution

    rules = load_gate_rules()

    assert any(
        "check_page_size" in rule.checks
        for rule in rules_for_execution(rules, tier="T2", context="main")
    )
    assert any(
        "check_page_size" in rule.checks
        for rule in rules_for_execution(rules, tier="T2", context="release")
    )
    assert rules_for_execution(rules, tier="T3", context="release")


def test_task_check_reports_rule_evidence_for_selected_context(monkeypatch, capsys):
    import task_check
    from _diff import GateRule

    rule = GateRule(
        match="known/**",
        checks=["check_known"],
        tier="T2",
        rule_ids=["GOV-CODE-KNOWN"],
        source="docs/governance.md#lightweight-governance",
        contexts=["pr"],
    )
    monkeypatch.setattr(task_check, "load_gate_rules", lambda: [rule])
    monkeypatch.setattr(task_check, "get_changed_files", lambda **_kwargs: ["known/change.rs"])
    monkeypatch.setattr(
        task_check,
        "run_one",
        lambda name, **_kwargs: task_check.ScriptResult(name, 0, 0, 1),
    )

    assert task_check.main(["--tier", "T2", "--context", "pr", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["context"] == "pr"
    assert payload["triggered"][0]["rule_ids"] == ["GOV-CODE-KNOWN"]
    assert payload["triggered"][0]["sources"] == [
        "docs/governance.md#lightweight-governance"
    ]
    assert payload["triggered"][0]["status"] == "passed"


def test_task_check_reads_context_from_ci_environment(monkeypatch, capsys):
    import task_check
    from _diff import GateRule

    rule = GateRule(
        match="known/**",
        checks=["check_known"],
        tier="T2",
        contexts=["pr"],
    )
    monkeypatch.setenv("WMS_GOV_CONTEXT", "pr")
    monkeypatch.setattr(task_check, "load_gate_rules", lambda: [rule])
    monkeypatch.setattr(task_check, "get_changed_files", lambda **_kwargs: ["known/change.rs"])
    monkeypatch.setattr(
        task_check,
        "run_one",
        lambda name, **_kwargs: task_check.ScriptResult(name, 0, 0, 1),
    )

    assert task_check.main(["--tier", "T2", "--json"]) == 0
    assert json.loads(capsys.readouterr().out)["context"] == "pr"


def test_task_check_reads_base_from_ci_environment(monkeypatch, capsys):
    import task_check

    captured = {}
    monkeypatch.setenv("WMS_GOV_BASE", "origin/main")
    monkeypatch.setattr(task_check, "load_gate_rules", lambda: [])

    def changed_files(**kwargs):
        captured.update(kwargs)
        return []

    monkeypatch.setattr(task_check, "get_changed_files", changed_files)

    assert task_check.main(["--tier", "T2", "--json"]) == 0
    assert captured["base_ref"] == "origin/main"
    assert json.loads(capsys.readouterr().out)["base"] == "origin/main"


def test_task_check_preserves_blocked_child_status(monkeypatch):
    import task_check

    monkeypatch.setattr(
        task_check.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(
            returncode=1,
            stdout='{"ok": false, "status": "blocked"}',
        ),
    )

    result = task_check.run_one(
        "validate_wave4_external_dependencies",
        json_mode=True,
        strict_mode=True,
    )

    assert result.exit_code == 1
    assert result.status == "blocked"


def test_missing_external_evidence_reports_blocked(tmp_path, capsys, monkeypatch):
    validators = (
        "validate_wave3_pda_runtime_evidence",
        "validate_wave4_external_dependencies",
        "validate_wave5_hardware_evidence",
        "validate_wave5_tms_evidence",
        "validate_wave6_deploy_evidence",
    )

    for validator_name in validators:
        validator = importlib.import_module(validator_name)
        missing = tmp_path / f"{validator_name}.json"
        assert validator.main(["--evidence-file", str(missing), "--json"]) == 1
        payload = json.loads(capsys.readouterr().out)
        assert payload["status"] == "blocked"

    wave1 = importlib.import_module("validate_wave1_runtime_evidence")
    assert wave1.main([
        "--h2-file",
        str(tmp_path / "wave1-h2.json"),
        "--w1d-file",
        str(tmp_path / "wave1-w1d.json"),
        "--json",
    ]) == 1
    wave1_payload = json.loads(capsys.readouterr().out)
    assert wave1_payload["status"] == "blocked"
    assert {item["status"] for item in wave1_payload["results"]} == {"blocked"}

    wave2 = importlib.import_module("report_wave2_completion")
    monkeypatch.setattr(
        wave2,
        "collect_items",
        lambda: [
            wave2.EvidenceItem(
                "W2.G-runtime",
                "runtime evidence",
                wave2.PRE_RELEASE_GATE,
                gaps=[wave2.MISSING_RUNTIME_EVIDENCE],
                strict_blocking=False,
            )
        ],
    )
    assert wave2.main(["--require-runtime-evidence", "--json"]) == 1
    assert json.loads(capsys.readouterr().out)["status"] == "blocked"


def test_supplied_invalid_external_evidence_reports_failed(tmp_path, capsys, monkeypatch):
    wave1 = importlib.import_module("validate_wave1_runtime_evidence")
    h2 = tmp_path / "wave1-h2.json"
    w1d = tmp_path / "wave1-w1d.json"
    h2.write_text("{}", encoding="utf-8")
    w1d.write_text("{}", encoding="utf-8")

    assert wave1.main([
        "--h2-file",
        str(h2),
        "--w1d-file",
        str(w1d),
        "--json",
    ]) == 1
    assert json.loads(capsys.readouterr().out)["status"] == "failed"

    wave2 = importlib.import_module("report_wave2_completion")
    monkeypatch.setattr(
        wave2,
        "collect_items",
        lambda: [
            wave2.EvidenceItem(
                "W2.G-runtime",
                "runtime evidence",
                wave2.PRE_RELEASE_GATE,
                gaps=["runtime evidence JSON 无效"],
                strict_blocking=False,
            )
        ],
    )
    assert wave2.main(["--require-runtime-evidence", "--json"]) == 1
    assert json.loads(capsys.readouterr().out)["status"] == "failed"


def test_page_size_is_not_part_of_t1_budget():
    from _diff import load_gate_rules
    from governance_checks import expand_tier_scripts

    assert "check_page_size.py" not in expand_tier_scripts("T1")
    assert "check_page_size.py" in expand_tier_scripts("T2")

    page_size_rules = [rule for rule in load_gate_rules() if "check_page_size" in rule.checks]
    assert page_size_rules
    assert {rule.tier for rule in page_size_rules} == {"T2"}


def test_touched_and_external_rules_have_explicit_traceability():
    try:
        import tomllib
    except ModuleNotFoundError:
        import tomli as tomllib

    data = tomllib.loads(
        (REPO_ROOT / "governance" / "gate-rules.toml").read_text(encoding="utf-8")
    )
    tracked_checks = {
        "check_page_size",
        "validate_wave1_runtime_evidence",
        "report_wave2_completion",
        "validate_wave3_pda_runtime_evidence",
        "validate_wave4_external_dependencies",
        "validate_wave5_hardware_evidence",
        "validate_wave5_tms_evidence",
        "validate_wave6_deploy_evidence",
    }
    tracked_rules = [
        rule
        for rule in data["rules"]
        if tracked_checks.intersection(rule.get("checks", []))
    ]

    assert tracked_rules
    assert all(rule.get("rule_ids") for rule in tracked_rules)
    assert all(rule.get("source") for rule in tracked_rules)
    assert all(
        rule["source"] != data["governance_model"]["default_source"]
        for rule in tracked_rules
    )


def test_gitea_workflow_uses_lightweight_pr_gates():
    workflow = REPO_ROOT / ".gitea" / "workflows" / "governance.yml"
    text = workflow.read_text(encoding="utf-8")

    assert "pull_request:" in text
    assert "just task-check" in text
    assert "--tier T3" in text
    assert "--context pr" in text
    assert "WMS_GOV_CONTEXT: pr" in text
    assert "WMS_GOV_BASE: origin/main" in text
    assert "workflow_dispatch:" in text
    assert "actions/setup-python@v5" in text
    assert "dtolnay/rust-toolchain@stable" in text
    assert "actions/setup-node@v4" in text
    assert "taiki-e/install-action@just" in text
    assert "pip install -r requirements-governance.txt" in text
    assert "pnpm install --frozen-lockfile" in text
    assert "gitleaks_8.21.2_linux_x64.tar.gz" in text
    assert "gitleaks detect --redact --no-banner" in text
    assert "Start PostgreSQL for T3/T4" in text
    assert "postgres:16" in text
    assert "openssl rand -hex 24" in text
    assert 'echo "DATABASE_URL=' in text
    assert '>> "$GITHUB_ENV"' in text
    assert "playwright install --with-deps chromium" in text


def test_governance_scope_web_admin_self_checks_are_in_task_gate():
    package = json.loads(
        (REPO_ROOT / "apps" / "web-admin" / "package.json").read_text(encoding="utf-8")
    )
    command = package["scripts"]["test:self-checks"]
    expected = [
        "di-drug-inspection-slice-self-check.mjs",
        "h1-dock-appointment-change-self-check.mjs",
        "m10-tms-route-plan-page-self-check.mjs",
        "m3-inventory-status-config-self-check.mjs",
        "m9-billing-rule-page-self-check.mjs",
        "te-task-type-slice-self-check.mjs",
    ]

    assert all(f"node self-checks/{name}" in command for name in expected)


def test_governance_docs_only_name_existing_t4_checks():
    from check_governance_consistency import non_diff_doc_script_issues

    governance = (REPO_ROOT / "docs" / "governance.md").read_text(encoding="utf-8")
    test_layers = (REPO_ROOT / "docs" / "adr" / "0006-tdd-and-test-layers.md").read_text(
        encoding="utf-8"
    )
    frontend = (REPO_ROOT / "docs" / "frontend-coding-standards.md").read_text(
        encoding="utf-8"
    )

    assert "check_perf_baseline.py" not in governance
    assert "check_observability_signals.py" not in governance
    assert "report_wave6_pre_release.py" in governance
    assert "check_observability.py" in governance
    assert "check_perf_baseline.py" not in test_layers
    assert "check_observability_test.py" not in test_layers
    assert "| `check_page_size.py` | T2 |" in frontend
    assert non_diff_doc_script_issues(governance, (REPO_ROOT / "justfile").read_text()) == []


def test_ci_python_dependencies_cover_required_governance_tools():
    requirements = (REPO_ROOT / "requirements-governance.txt").read_text(
        encoding="utf-8"
    )

    assert "Markdown==" in requirements
    assert "ruff==" in requirements
    assert "pytest==" in requirements


def test_governance_consistency_rejects_incomplete_control_chain():
    from _diff import GateConfig, GateRule, GovernanceModel
    from check_governance_consistency import governance_model_issues

    config = GateConfig(
        model=GovernanceModel(
            version=2,
            layers=["G1", "G2"],
            decision_precedence=[],
            rule_verdicts=["pass", "warning", "fail"],
            execution_statuses=["passed", "failed"],
            allowed_contexts=["local"],
            default_source="",
            tier_contexts={"T1": ["local"]},
        ),
        rules=[GateRule("docs/**", ["check_docs"], "T1")],
    )

    kinds = {issue.kind for issue in governance_model_issues(config)}

    assert "model_layers" in kinds
    assert "model_version" in kinds
    assert "model_verdicts" in kinds
    assert "model_execution_statuses" in kinds
    assert "model_tier_contexts" in kinds
    assert "rule_source" in kinds
