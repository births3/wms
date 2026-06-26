"""治理入口 strict 语义传递测试。"""
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
