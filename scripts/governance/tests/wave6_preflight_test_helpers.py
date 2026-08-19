"""Shared fixtures for Wave 6 evidence preflight governance tests."""

COMMON_TERMS = [
    "environment",
    "dev",
    "staging",
    "local",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "example",
]
PLACEHOLDER_TERMS = ["YYYY", "<...>", "TODO", "TBD", "待填", "待确认"]


def make_gate(
    check,
    gate_id="W6.X",
    title="test gate",
    runbook="docs/runbooks/wave-x-evidence.md",
    evidence_file="docs/retros/wave-x-evidence.json",
    just_entries=("wave-x-record", "wave-x-validate"),
):
    return check.GateSpec(
        gate_id,
        title,
        runbook,
        evidence_file,
        just_entries,
    )


def write_files(tmp_path, files):
    for rel_path, text in files.items():
        path = tmp_path / rel_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def collect_overwrite_guard_errors(tmp_path, monkeypatch, files):
    import check_wave6_evidence_preflight as check

    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    write_files(tmp_path, files)
    return check.check_execution_file_overwrite_guards(tuple(files))


def closeout_recommended_order_lines(*entries: str) -> list[str]:
    return [
        "## 推荐执行顺序",
        "```bash",
        *[f"just {entry}" for entry in entries],
        "```",
    ]


def closeout_wrapper_command_lines() -> list[str]:
    return [
        "```bash",
        "just wave-6-status",
        "just wave-6-evidence-check",
        "just wave-6-missing-evidence-commands",
        "just wave-6-complete-check",
        "```",
    ]


def closeout_gate_matrix_lines(gate) -> list[str]:
    return [
        "## 当前 Gate",
        (
            f"| {gate.gate_id} | {gate.evidence_file} | "
            + " | ".join(gate.just_entries)
            + " |"
        ),
    ]


def closeout_text_lines(check, gate) -> list[str]:
    return [
        "just wave-6-evidence-preflight",
        check.PREFLIGHT_DOC,
        *closeout_wrapper_command_lines(),
        *closeout_gate_matrix_lines(gate),
        *closeout_recommended_order_lines(*gate.just_entries),
    ]


def write_closeout_preflight_fixture(
    tmp_path,
    check,
    monkeypatch,
    *,
    preflight_lines=(),
    closeout_lines=(),
    just_text="wave-6-evidence-preflight:\n",
    execution_files=(),
):
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", ())
    monkeypatch.setattr(check, "REQUIRED_EXECUTION_FILES", execution_files)

    files = {
        check.PREFLIGHT_DOC: "\n".join([
            "just wave-6-evidence-preflight",
            "不会写入 runtime evidence",
            "不能关闭 gate",
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
            *preflight_lines,
        ]),
        check.CLOSEOUT_DOC: "\n".join([
            *closeout_lines,
            *closeout_recommended_order_lines(),
        ]),
        check.TODO_DOC: "W6 evidence preflight",
        check.JUSTFILE: just_text,
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": "ADR-0035",
    }
    files.update({path: "" for path in execution_files})
    write_files(tmp_path, files)


def write_single_gate_preflight_fixture(
    tmp_path,
    check,
    monkeypatch,
    *,
    gate=None,
    runbook_lines=(),
    closeout_text=None,
    just_text=(
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
        "wave-x-record:\n"
        "wave-x-validate:\n"
    ),
    execution_files=("scripts/governance/x.py",),
):
    gate = gate or make_gate(check)
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "GATES", (gate,))
    monkeypatch.setattr(check, "REQUIRED_EXECUTION_FILES", execution_files)

    if closeout_text is None:
        closeout_text = (
            f"just wave-6-evidence-preflight\n{check.PREFLIGHT_DOC}\n"
            + "\n".join(closeout_wrapper_command_lines())
            + "\n"
            "## 当前 Gate\n"
            f"{gate.gate_id} {gate.evidence_file} {' '.join(gate.just_entries)}\n"
            "## 推荐执行顺序\n"
            "```bash\n"
            + "\n".join(f"just {entry}" for entry in gate.just_entries)
            + "\n"
            "```\n"
        )

    files = {
        check.PREFLIGHT_DOC: "\n".join([
            "just wave-6-evidence-preflight",
            "不会写入 runtime evidence",
            "不能关闭 gate",
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
            gate.gate_id,
            gate.evidence_file,
            *gate.just_entries,
            "just wave-6-status",
            "just wave-6-evidence-check",
            "just wave-6-missing-evidence-commands",
            "just wave-6-complete-check",
        ]),
        check.CLOSEOUT_DOC: closeout_text,
        check.TODO_DOC: "W6 evidence preflight",
        check.JUSTFILE: just_text,
        "docs/adr/0035-wave-6-pre-release-evidence-closeout.md": "ADR-0035",
        gate.runbook: "\n".join([
            gate.evidence_file,
            *gate.just_entries,
            *COMMON_TERMS,
            *PLACEHOLDER_TERMS,
            *runbook_lines,
        ]),
    }
    files.update({path: "" for path in execution_files})
    write_files(tmp_path, files)
    return gate
