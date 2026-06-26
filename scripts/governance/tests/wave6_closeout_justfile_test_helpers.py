"""Wave 6 closeout justfile 测试 helper。"""
from wave6_preflight_test_helpers import write_closeout_preflight_fixture


CLOSEOUT_JUST_ENTRIES = [
    "just wave-6-status",
    "just wave-6-evidence-check",
    "just wave-6-missing-evidence-commands",
    "just wave-6-complete-check",
]

EXPECTED_JUST_TEXT = (
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
)


def write_closeout_justfile_fixture(
    tmp_path,
    check,
    monkeypatch,
    *,
    preflight_lines=None,
    closeout_lines=None,
    just_text=EXPECTED_JUST_TEXT,
):
    if preflight_lines is None:
        preflight_lines = CLOSEOUT_JUST_ENTRIES
    if closeout_lines is None:
        closeout_lines = [
            "just wave-6-evidence-preflight",
            "```bash",
            *CLOSEOUT_JUST_ENTRIES,
            "```",
            check.PREFLIGHT_DOC,
            "## 当前 Gate",
        ]

    write_closeout_preflight_fixture(
        tmp_path,
        check,
        monkeypatch,
        preflight_lines=preflight_lines,
        closeout_lines=closeout_lines,
        just_text=just_text,
    )
