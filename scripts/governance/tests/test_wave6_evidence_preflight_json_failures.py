"""Wave 6 evidence preflight 失败态 JSON 合同测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave6_evidence_preflight_json_contract_separates_top_and_gate_errors(
    capsys,
    monkeypatch,
):
    """Wave 6 preflight JSON 必须把顶层错误和 gate 错误分桶，便于 CI 消费。"""
    import check_wave6_evidence_preflight as check

    original_check_gate = check.check_gate

    def fake_repo_path(path):
        if path == check.PREFLIGHT_DOC:
            return Path("/missing-wave6-preflight-doc")
        return Path("README.md")

    def fake_check_gate(gate, *, preflight_text, just_text):
        if gate.gate_id == "W6.A":
            return check.GateResult(gate.gate_id, gate.title, False, ["W6.A gate error"])
        return original_check_gate(gate, preflight_text=preflight_text, just_text=just_text)

    monkeypatch.setattr(check, "repo_path", fake_repo_path)
    monkeypatch.setattr(check, "read_text", lambda _path: "")
    monkeypatch.setattr(check, "check_gate", fake_check_gate)

    assert check.main(["--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert payload["ok"] is False
    assert payload["top_error_count"] == len(payload["top_errors"])
    assert payload["gate_error_count"] == len(payload["gate_errors"])
    assert payload["error_count"] == len(payload["errors"])
    assert payload["error_count"] == payload["top_error_count"] + payload["gate_error_count"]
    assert payload["top_error_details"] == [
        {"scope": "top", "gate_id": None, "message": error}
        for error in payload["top_errors"]
    ]
    assert len(payload["gate_error_details"]) == len(payload["gate_errors"])
    assert {
        "scope": "gate",
        "gate_id": "W6.A",
        "message": "W6.A gate error",
    } in payload["gate_error_details"]
    assert payload["error_details"] == [
        *payload["top_error_details"],
        *payload["gate_error_details"],
    ]
    assert payload["ok_gate_count"] + payload["failed_gate_count"] == payload["gate_count"]
    assert payload["failed_gate_count"] == len(payload["failed_gates"])
    failed_gates_from_results = [
        gate for gate in payload["gates"] if gate["ok"] is False
    ]
    assert payload["failed_gate_ids"] == [
        gate["gate_id"] for gate in payload["failed_gates"]
    ]
    assert payload["failed_gate_ids"] == [
        gate["gate_id"] for gate in failed_gates_from_results
    ]
    assert payload["failed_gates"] == failed_gates_from_results
    assert all(gate["errors"] for gate in payload["failed_gates"])
    assert "缺少文件" in payload["top_errors"][0]
    assert "W6.A gate error" in payload["gate_errors"]


def test_wave6_evidence_preflight_failure_json_does_not_leak_repo_root(
    tmp_path,
    capsys,
    monkeypatch,
):
    """Wave 6 preflight 失败态 JSON 也不能泄漏本机 repo 绝对路径。"""
    import check_wave6_evidence_preflight as check

    gate = check.GateSpec(
        "W6.X",
        "test gate",
        "docs/runbooks/wave-x-evidence.md",
        "docs/retros/wave-x-evidence.json",
        ("wave-x-record", "wave-x-validate"),
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", (gate,))
    monkeypatch.setattr(check, "REQUIRED_EXECUTION_FILES", ("scripts/governance/missing.py",))

    justfile = tmp_path / check.JUSTFILE
    justfile.write_text("wave-6-evidence-preflight:\n", encoding="utf-8")

    assert check.main(["--json"]) == 1
    payload_text = capsys.readouterr().out
    payload = json.loads(payload_text)

    assert str(tmp_path) not in payload_text
    assert payload["ok"] is False
    assert "缺少文件: docs/runbooks/wave-6-evidence-preflight.md" in payload["top_errors"]
    assert "缺少执行文件: scripts/governance/missing.py" in payload["top_errors"]
    assert "缺少 runbook: docs/runbooks/wave-x-evidence.md" in payload["gate_errors"]
