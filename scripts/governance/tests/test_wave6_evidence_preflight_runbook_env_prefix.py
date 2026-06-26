"""Wave 6 evidence preflight runbook record env var prefix tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_preflight_test_helpers import write_single_gate_preflight_fixture


def test_wave6_evidence_preflight_uses_evidence_entry_for_env_prefix(
    tmp_path,
    monkeypatch,
):
    """只有 *-evidence 采集入口时，环境变量建议不能误用 prereq/readiness 前缀。"""
    import check_wave6_evidence_preflight as check

    gate = check.GateSpec(
        "W6.A",
        "Wave 1 H2 runtime evidence",
        "docs/runbooks/wave-1-runtime-evidence.md",
        "docs/retros/wave-1-h2-runtime-evidence.json",
        (
            "wave-1-runtime-prereq-h2",
            "wave-1-h2-runtime-readiness",
            "wave-1-h2-runtime-evidence",
            "wave-1-runtime-evidence-validate",
        ),
    )
    closeout_text = "\n".join([
        "just wave-6-evidence-preflight",
        check.PREFLIGHT_DOC,
        "```bash",
        "just wave-6-status",
        "just wave-6-evidence-check",
        "just wave-6-missing-evidence-commands",
        "just wave-6-complete-check",
        "```",
        "## 当前 Gate",
        (
            "| W6.A | docs/retros/wave-1-h2-runtime-evidence.json | "
            "wave-1-h2-runtime-evidence | wave-1-runtime-evidence-validate |"
        ),
        "## 推荐执行顺序",
        "```bash",
        "just wave-1-h2-runtime-evidence",
        "just wave-1-runtime-evidence-validate",
        "```",
    ])
    just_text = (
        "wave-6-evidence-preflight:\n"
        "    @python3 scripts/governance/check_wave6_evidence_preflight.py\n"
        "wave-6-status:\n"
        "    @python3 scripts/governance/report_wave6_pre_release.py\n"
        "wave-6-evidence-check:\n"
        "    @python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only\n"
        "wave-6-missing-evidence-commands:\n"
        "    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only\n"
        "wave-6-complete-check:\n"
        "    @python3 scripts/governance/report_wave6_pre_release.py --strict\n"
        "wave-1-runtime-prereq-h2:\n"
        "wave-1-h2-runtime-readiness:\n"
        "wave-1-h2-runtime-evidence:\n"
        "wave-1-runtime-evidence-validate:\n"
    )

    write_single_gate_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        gate=gate,
        runbook_lines=[
            "```bash",
            "just wave-1-h2-runtime-evidence \\",
            "  --smoke-log-ref ci/staging/wave-1-h2-smoke/123",
            "```",
        ],
        closeout_text=closeout_text,
        just_text=just_text,
        execution_files=(),
    )
    monkeypatch.setattr(check, "gate_execution_files", lambda _gate: [])

    top_errors, gate_results = check.collect_results()
    joined_errors = " ".join(gate_results[0].errors)

    assert top_errors == []
    assert "WAVE_1_H2_SMOKE_LOG_REF" in joined_errors
    assert "WAVE_1_RUNTIME_PREREQ_H2_SMOKE_LOG_REF" not in joined_errors
