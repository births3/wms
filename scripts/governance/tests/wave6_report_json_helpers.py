"""Shared assertions for Wave 6 report JSON governance tests."""


def patch_wave6_report_io(monkeypatch, report, *, run_validator=None, existing_files=None):
    """Patch Wave 6 report I/O checks to isolate tests from the real workspace."""
    existing = set(report.WAVE6_TOOLING_FILES if existing_files is None else existing_files)

    monkeypatch.setattr(report, "file_exists", lambda path: path in existing)
    monkeypatch.setattr(report, "file_contains", lambda _path, *_needles: True)
    monkeypatch.setattr(report, "run_validator", run_validator or (lambda *_args: (True, "ok")))


def wave6_missing_file_validator(missing_by_command):
    def fake_run_validator(*args):
        command = " ".join(args)
        for validator, evidence_file in missing_by_command.items():
            if validator in command:
                return False, f"missing file: {evidence_file}"
        return True, "ok"

    return fake_run_validator


def wave6_all_missing_evidence_validator():
    def fake_run_validator(*args):
        import json

        command = " ".join(args)
        if "validate_wave1_runtime_evidence.py" in command and "--kind h2" in command:
            return False, "missing file: docs/retros/wave-1-h2-runtime-evidence.json"
        if "validate_wave1_runtime_evidence.py" in command and "--kind w1d" in command:
            return False, "missing file: docs/retros/wave-1-runtime-evidence.json"
        if "report_wave2_completion.py" in command:
            return False, json.dumps(
                {
                    "runtime_blocking_gaps": [
                        {
                            "item_id": "W2.G-runtime",
                            "gaps": [
                                "缺少 docs/retros/wave-2-runtime-evidence.json 真实 dev/staging 配置中心灰度证据",
                            ],
                        },
                    ],
                    "pre_release_gates": [],
                    "blocking_gaps": [],
                },
                ensure_ascii=False,
            )
        validator_missing_files = {
            "validate_wave3_pda_runtime_evidence.py": "docs/retros/wave-3-pda-runtime-evidence.json",
            "validate_wave4_external_dependencies.py": "docs/retros/wave-4-external-dependencies.json",
            "validate_wave5_hardware_evidence.py": "docs/retros/wave-5-hardware-evidence.json",
            "validate_wave5_tms_evidence.py": "docs/retros/wave-5-tms-evidence.json",
            "validate_wave6_deploy_evidence.py": "docs/retros/wave-6-deploy-evidence.json",
        }
        for validator, evidence_file in validator_missing_files.items():
            if validator in command:
                return False, f"missing file: {evidence_file}"
        return True, "ok"

    return fake_run_validator


def wave6_item_ids(items):
    return [item["item_id"] for item in items]


def assert_wave6_report_json_groups_are_consistent(payload):
    item_ids = wave6_item_ids(payload["items"])
    evidence_gate_item_ids = payload["evidence_gate_item_ids"]
    non_evidence_item_ids = payload["non_evidence_item_ids"]
    blocking_item_ids = payload["blocking_item_ids"]
    ignored_item_ids = payload["ignored_item_ids"]
    evidence_blocking_item_ids = payload["evidence_blocking_item_ids"]
    evidence_ignored_item_ids = payload["evidence_ignored_item_ids"]
    non_evidence_blocking_item_ids = payload["non_evidence_blocking_item_ids"]
    non_evidence_ignored_item_ids = payload["non_evidence_ignored_item_ids"]

    assert evidence_gate_item_ids == wave6_item_ids(payload["evidence_gate_items"])
    assert non_evidence_item_ids == wave6_item_ids(payload["non_evidence_items"])
    assert blocking_item_ids == wave6_item_ids(payload["blocking_gaps"])
    assert ignored_item_ids == wave6_item_ids(payload["ignored_gaps"])
    assert evidence_blocking_item_ids == wave6_item_ids(payload["evidence_blocking_gaps"])
    assert evidence_ignored_item_ids == wave6_item_ids(payload["evidence_ignored_gaps"])
    assert non_evidence_blocking_item_ids == wave6_item_ids(payload["non_evidence_blocking_gaps"])
    assert non_evidence_ignored_item_ids == wave6_item_ids(payload["non_evidence_ignored_gaps"])

    assert set(evidence_gate_item_ids).isdisjoint(non_evidence_item_ids)
    assert sorted(evidence_gate_item_ids + non_evidence_item_ids) == sorted(item_ids)

    assert set(blocking_item_ids).isdisjoint(ignored_item_ids)
    assert set(blocking_item_ids) <= set(item_ids)
    assert set(ignored_item_ids) <= set(item_ids)

    assert set(evidence_blocking_item_ids).isdisjoint(non_evidence_blocking_item_ids)
    assert sorted(evidence_blocking_item_ids + non_evidence_blocking_item_ids) == sorted(blocking_item_ids)
    assert set(evidence_ignored_item_ids).isdisjoint(non_evidence_ignored_item_ids)
    assert sorted(evidence_ignored_item_ids + non_evidence_ignored_item_ids) == sorted(ignored_item_ids)


def assert_wave6_missing_evidence_files_are_consistent(payload):
    from check_wave6_evidence_preflight import GATES, gate_commands_by_phase
    import report_wave6_pre_release as report

    missing_item_ids = payload["missing_evidence_item_ids"]
    missing_files = payload["missing_evidence_files"]
    missing_details = payload["missing_evidence_details"]
    blocking_items = payload["evidence_blocking_gaps"]
    commands_by_phase = gate_commands_by_phase()
    evidence_file_by_gate_id = {gate.gate_id: gate.evidence_file for gate in GATES}

    assert payload["missing_evidence_count"] == len(missing_files)
    assert len(missing_item_ids) == len(missing_files)
    assert len(missing_details) == len(missing_files)
    assert missing_item_ids == [item["item_id"] for item in blocking_items]
    assert missing_files == [
        evidence_file_by_gate_id[item["item_id"].split("-", 1)[0]]
        for item in blocking_items
    ]
    assert all(path.startswith("docs/retros/") for path in missing_files)
    assert all(path.endswith(".json") for path in missing_files)
    expected_details = []
    for item in blocking_items:
        gate_id = item["item_id"].split("-", 1)[0]
        detail = {
            "gate_id": gate_id,
            "item_id": item["item_id"],
            "evidence_file": evidence_file_by_gate_id[gate_id],
            "status": item["status"],
            "requirement": item["requirement"],
            "gaps": item["gaps"],
            "external_prerequisites": report.external_prerequisites_for_gate(gate_id),
            "minimum_evidence_refs": report.minimum_evidence_refs_for_gate(gate_id),
            "readiness_commands": report.readiness_commands_for_gate(gate_id),
            "record_check_only_commands": report.record_check_only_commands_for_gate(gate_id),
            "record_commands": report.record_commands_for_gate(gate_id),
            "validate_commands": commands_by_phase[gate_id]["validate"],
        }
        detail.update(report.deployment_choice_metadata_for_detail(detail))
        expected_details.append(detail)

    assert missing_details == expected_details


def expected_wave6_commands_only_titles(payload):
    return [
        f"# {detail['gate_id']} {detail['item_id']}"
        for detail in payload["missing_evidence_details"]
    ]


def expected_wave6_commands_only_lines(payload):
    import report_wave6_pre_release as report

    lines = [
        report.COMMANDS_ONLY_BOUNDARY_LINE,
        report.COMMANDS_ONLY_STRICT_EXIT_LINE,
    ]
    for detail in payload["missing_evidence_details"]:
        lines.append(f"# {detail['gate_id']} {detail['item_id']}")
        lines.append(f"evidence: {detail['evidence_file']}")
        choice_line = report.commands_only_choice_line_for_detail(detail)
        if choice_line is not None:
            lines.append(choice_line)
        for prerequisite in detail["external_prerequisites"]:
            lines.append(f"external-prereq: {prerequisite}")
        for evidence_ref in detail["minimum_evidence_refs"]:
            lines.append(f"minimum-evidence-ref: {evidence_ref}")
        lines.extend(report.commands_only_phase_lines_for_detail(detail))
    return lines


def expected_wave6_report_details(items):
    details = []
    for item in items:
        is_evidence = item["item_id"].startswith("W6.") and "-" in item["item_id"]
        details.append({
            "kind": "evidence" if is_evidence else "non_evidence",
            "gate_id": item["item_id"].split("-", 1)[0] if is_evidence else None,
            "item_id": item["item_id"],
            "status": item["status"],
            "requirement": item["requirement"],
            "gaps": item["gaps"],
        })
    return details


def assert_wave6_report_details_are_consistent(payload):
    assert payload["blocking_details"] == expected_wave6_report_details(
        payload["blocking_gaps"]
    )
    assert payload["ignored_details"] == expected_wave6_report_details(
        payload["ignored_gaps"]
    )
    assert [
        detail["item_id"]
        for detail in payload["blocking_details"]
    ] == payload["blocking_item_ids"]
    assert [
        detail["item_id"]
        for detail in payload["ignored_details"]
    ] == payload["ignored_item_ids"]
    assert all(
        detail["kind"] in {"evidence", "non_evidence"}
        for detail in [*payload["blocking_details"], *payload["ignored_details"]]
    )


def assert_wave6_report_tooling_metadata_is_consistent(payload, report):
    from check_wave6_evidence_preflight import (
        GATES,
        REQUIRED_TOP_LEVEL_FILES,
        WAVE6_CLOSEOUT_JUST_ENTRIES,
        gate_execution_file_map,
        gate_just_entries,
        required_runbooks,
    )

    assert payload["evidence_gate_just_entries"] == gate_just_entries()
    assert payload["evidence_gate_execution_files"] == gate_execution_file_map()
    assert payload["required_top_level_files"] == list(REQUIRED_TOP_LEVEL_FILES)
    assert payload["required_runbooks"] == required_runbooks()
    assert payload["required_execution_files"] == list(report.REQUIRED_EXECUTION_FILES)
    assert payload["validation_commands"] == report.WAVE6_VALIDATION_COMMANDS
    assert payload["closeout_just_entries"] == list(report.WAVE6_CLOSEOUT_JUST_ENTRIES)
    assert payload["schema_version"] == report.SCHEMA_VERSION
    assert payload["available_modes"] == list(report.REPORT_MODES)
    assert payload["writes_runtime_evidence"] is False
    assert payload["closes_gate"] is False
    assert payload["report_command"] == report.REPORT_COMMAND
    assert payload["evidence_only_command"] == (
        "python3 scripts/governance/report_wave6_pre_release.py "
        "--strict --evidence-only --json"
    )
    assert payload["commands_only_command"] == (
        "just wave-6-missing-evidence-commands"
    )

    expected_report_just_entries = list(
        dict.fromkeys(
            [
                *(entry for gate in GATES for entry in gate.just_entries),
                "wave-6-evidence-preflight",
                *WAVE6_CLOSEOUT_JUST_ENTRIES,
            ],
        ),
    )
    assert report.WAVE6_JUST_ENTRIES == expected_report_just_entries
