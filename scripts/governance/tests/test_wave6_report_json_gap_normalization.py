"""Wave 6 report JSON gap 摘要与路径归一化测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import patch_wave6_report_io


def test_wave6_wave2_runtime_gap_uses_concise_json_summary(monkeypatch):
    """Wave 6 报告嵌入 Wave 2 runtime 缺口时，不能把整段 Wave 2 人类报告塞进 gap。"""
    import report_wave6_pre_release as report

    runtime_gap = "缺少 docs/retros/wave-2-runtime-evidence.json 真实 dev/staging 配置中心灰度证据"

    def fake_run_validator(*args):
        command = " ".join(args)
        if "report_wave2_completion.py" in command:
            assert "--json" in args
            return False, json.dumps(
                {
                    "runtime_blocking_gaps": [
                        {
                            "item_id": "W2.G-runtime",
                            "gaps": [runtime_gap],
                        }
                    ],
                    "pre_release_gates": [
                        {
                            "item_id": "W2.G-runtime",
                            "gaps": [runtime_gap],
                        }
                    ],
                    "blocking_gaps": [],
                },
                ensure_ascii=False,
            )
        return True, "ok"

    patch_wave6_report_io(monkeypatch, report, run_validator=fake_run_validator)

    item = {item.item_id: item for item in report.collect_items()}["W6.C-wave2-runtime"]

    assert item.status == report.MISSING_OR_NEEDS_EXTERNAL_STATE
    assert item.gaps == [f"W2.G-runtime: {runtime_gap}"]
    assert "report_wave2_completion" not in " ".join(item.gaps)


def test_wave6_report_normalizes_validator_absolute_paths(monkeypatch, capsys):
    """Wave 6 report gaps 不应泄漏本机 repo 绝对路径。"""
    import report_wave6_pre_release as report

    absolute_evidence_path = (
        report.REPO_ROOT / "docs/retros/wave-3-pda-runtime-evidence.json"
    )
    bare_repo_root_gap = f"cwd={report.REPO_ROOT}"

    def fake_run_validator(*args):
        if "validate_wave3_pda_runtime_evidence.py" in " ".join(args):
            return False, (
                f"missing file: {absolute_evidence_path}; {bare_repo_root_gap}"
            )
        return True, "ok"

    patch_wave6_report_io(monkeypatch, report, run_validator=fake_run_validator)

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    joined_gaps = json.dumps(payload, ensure_ascii=False)

    assert str(report.REPO_ROOT) not in joined_gaps
    assert (
        "missing file: docs/retros/wave-3-pda-runtime-evidence.json; cwd=."
        in joined_gaps
    )
