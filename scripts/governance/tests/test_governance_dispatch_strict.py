"""治理入口 strict 语义传递测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_governance_checks_passes_strict_to_openapi(monkeypatch):
    """T2 全量入口运行 OpenAPI 同步脚本时必须传 --strict。"""
    import subprocess
    import governance_checks as checks

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    monkeypatch.setattr(checks.subprocess, "run", fake_run)

    result = checks.run_script("check_openapi_in_sync.py", json_mode=True)

    assert result.exit_code == 0
    assert "--strict" in captured["cmd"]
    assert "--json" in captured["cmd"]


def test_governance_checks_t2_failure_is_aggregated(monkeypatch):
    """T2 子脚本失败必须向调度器总退出码传播。"""
    import governance_checks as checks

    monkeypatch.setattr(
        checks,
        "expand_tier_scripts",
        lambda tier: ["check_doc_links.py", "check_openapi_in_sync.py"],
    )

    def fake_run_script(name, *, json_mode):
        exit_code = 1 if name == "check_openapi_in_sync.py" else 0
        return checks.ScriptResult(name=name, exit_code=exit_code, duration_ms=1)

    monkeypatch.setattr(checks, "run_script", fake_run_script)

    assert checks.main(["--tier", "T2"]) == 1


def test_task_check_strict_passes_strict_to_openapi(monkeypatch):
    """diff-driven strict 模式也必须把严格语义传给 OpenAPI 同步脚本。"""
    import subprocess
    import task_check

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return subprocess.CompletedProcess(cmd, 0)

    monkeypatch.setattr(task_check.subprocess, "run", fake_run)

    result = task_check.run_one("check_openapi_in_sync", json_mode=True, strict_mode=True)

    assert result.exit_code == 0
    assert "--strict" in captured["cmd"]
    assert "--json" in captured["cmd"]


def test_task_check_strict_requires_wave2_runtime_evidence(monkeypatch):
    """Wave 2 runtime evidence diff gate 必须用严格 runtime evidence 语义运行。"""
    import subprocess
    import task_check

    captured = {}

    def fake_run(cmd, **kwargs):
        captured["cmd"] = cmd
        return subprocess.CompletedProcess(cmd, 0)

    monkeypatch.setattr(task_check.subprocess, "run", fake_run)

    result = task_check.run_one("report_wave2_completion", json_mode=True, strict_mode=True)

    assert result.exit_code == 0
    assert "--strict" in captured["cmd"]
    assert "--require-runtime-evidence" in captured["cmd"]
    assert "--json" in captured["cmd"]


def test_task_check_strict_script_args_are_explicitly_scoped():
    """strict 附加参数只允许需要特殊语义的脚本。"""
    import task_check

    assert task_check.STRICT_SCRIPT_ARGS == {
        "check_openapi_in_sync": ["--strict"],
        "report_wave2_completion": ["--strict", "--require-runtime-evidence"],
    }


def test_tier_entrypoints_reject_placeholders_and_swallowed_failures():
    """主 Tier 入口不能继续保留占位命令或用 || true 吞掉失败。"""
    from check_governance_consistency import tier_entrypoint_issues

    issues = tier_entrypoint_issues(
        """
_t1-fmt:
    @echo "format placeholder"
_t3-governance-l3:
    @python3 scripts/governance/governance_checks.py --tier T3 || :
""",
        required={"_t1-fmt", "_t3-governance-l3"},
    )

    assert {issue.kind for issue in issues} == {"tier_placeholder", "tier_failure_swallowed"}


def test_preflight_runs_t3_diff_governance():
    justfile = (Path(__file__).resolve().parents[3] / "justfile").read_text(encoding="utf-8")

    assert "python3 scripts/governance/task_check.py --tier T3 --strict" in justfile


def test_tier_entrypoints_reject_semicolon_true():
    from check_governance_consistency import tier_entrypoint_issues

    issues = tier_entrypoint_issues(
        """
_t1-fmt:
    @cargo fmt --all -- --check; true
""",
        required={"_t1-fmt"},
    )

    assert [issue.kind for issue in issues] == ["tier_failure_swallowed"]


def test_tier_entrypoints_reject_other_failure_swallowing_forms():
    from check_governance_consistency import tier_entrypoint_issues

    for command in ("cargo fmt || echo ignored", "cargo fmt; exit 0", "set +e\n    @cargo fmt"):
        issues = tier_entrypoint_issues(
            f"_t1-fmt:\n    @{command}\n",
            required={"_t1-fmt"},
        )
        assert "tier_failure_swallowed" in {issue.kind for issue in issues}


def test_t4_uses_existing_wave6_strict_recipe():
    from check_governance_consistency import JUSTFILE

    just_text = JUSTFILE.read_text(encoding="utf-8")

    assert "@just wave-6-complete-check" in just_text
    assert "@just wave-6-status --strict" not in just_text


def test_tier_entrypoints_accept_real_fail_closed_commands():
    from check_governance_consistency import tier_entrypoint_issues

    issues = tier_entrypoint_issues(
        """
_t1-fmt:
    @cargo fmt --manifest-path backend/Cargo.toml --all -- --check
_t3-governance-l3:
    @python3 scripts/governance/governance_checks.py --tier T3
""",
        required={"_t1-fmt", "_t3-governance-l3"},
    )

    assert issues == []


def test_governance_dispatch_json_is_single_document(monkeypatch, capsys):
    import governance_checks as checks

    monkeypatch.setattr(checks, "expand_tier_scripts", lambda tier: ["check_doc_links.py"])
    monkeypatch.setattr(
        checks,
        "run_script",
        lambda name, *, json_mode: checks.ScriptResult(name=name, exit_code=0, duration_ms=1),
    )

    assert checks.main(["--tier", "T1", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is True
    assert payload["scripts"][0]["name"] == "check_doc_links.py"


def test_governance_dispatch_preserves_script_error_exit_code(monkeypatch, capsys):
    import governance_checks as checks

    monkeypatch.setattr(checks, "expand_tier_scripts", lambda tier: ["check_doc_links.py"])
    monkeypatch.setattr(
        checks,
        "run_script",
        lambda name, *, json_mode: checks.ScriptResult(name=name, exit_code=2, duration_ms=1),
    )

    assert checks.main(["--tier", "T1", "--json"]) == 2
    assert json.loads(capsys.readouterr().out)["ok"] is False
