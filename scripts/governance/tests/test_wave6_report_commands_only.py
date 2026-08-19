"""Wave 6 pre-release report commands-only governance tests."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from wave6_report_json_helpers import (
    expected_wave6_commands_only_lines,
    expected_wave6_commands_only_titles,
    patch_wave6_report_io,
    wave6_missing_file_validator,
)


def _patch_three_missing_evidence_gates(monkeypatch, report):
    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator(
            {
                "validate_wave1_runtime_evidence.py --kind h2": (
                    "docs/retros/wave-1-h2-runtime-evidence.json"
                ),
                "validate_wave3_pda_runtime_evidence.py": (
                    "docs/retros/wave-3-pda-runtime-evidence.json"
                ),
                "validate_wave6_deploy_evidence.py": (
                    "docs/retros/wave-6-deploy-evidence.json"
                ),
            },
        ),
    )


def test_wave6_commands_only_prints_missing_evidence_command_checklist(monkeypatch, capsys):
    """commands-only 只输出缺失 evidence gate 的采集命令清单。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output = capsys.readouterr().out

    assert "# W6.D W6.D-wave3-pda-l7" in output
    assert "evidence: docs/retros/wave-3-pda-runtime-evidence.json" in output
    assert "external-prereq: 幂等 replay 条件" in output
    assert "minimum-evidence-ref: L7 执行记录" in output
    assert (
        "readiness: just wave-3-pda-runtime-evidence-record --export-template"
        in output
    )
    assert "readiness: just wave-3-pda-materials-checklist --json" in output
    assert "readiness: just wave-3-pda-field-work-request" in output
    assert "readiness: just wave-3-pda-field-execution-summary --json" in output
    assert "readiness: just wave-3-pda-field-precheck-summary --from-env --json" in output
    assert "readiness: just wave-3-pda-field-owner-gap-actions --json" in output
    assert "readiness: just wave-3-pda-field-handoff-bundle --json" in output
    assert "readiness: just wave-3-pda-evidence-package-template" in output
    assert "readiness: just wave-3-pda-service-precheck --from-env --json" in output
    assert "readiness: just wave-3-pda-trace-code-openapi-precheck --from-env --json" in output
    assert "readiness: just wave-3-pda-runtime-readiness --from-env --json" in output
    assert "record: just wave-3-pda-runtime-evidence-record --from-env --json" in output
    assert "record: just wave-3-pda-intake-record --json" in output
    assert "validate: just wave-3-pda-runtime-evidence-validate" in output
    assert "report_wave6_pre_release (流程治理" not in output
    assert "status:" not in output
    assert "W6-retro" not in output


def test_wave6_commands_only_output_declares_static_boundary(monkeypatch, capsys):
    """commands-only 文本必须声明只读边界，避免被误读为关 gate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave3_pda_runtime_evidence.py": (
                "docs/retros/wave-3-pda-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output = capsys.readouterr().out

    assert "只读命令清单" in output
    assert "不会写入 runtime evidence" in output
    assert "不能关闭 evidence gate" in output
    assert "--strict 返回非零" in output
    assert "阻塞信号" in output
    assert "不代表命令写入或关闭 gate" in output


def test_wave6_commands_only_marks_w1d_rollback_as_deployment_choice(
    monkeypatch,
    capsys,
):
    """W6.B rollback 两套部署命令必须提示按实际部署形态二选一。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()
    title_index = output_lines.index("# W6.B W6.B-wave1-rollback-runtime")

    assert output_lines[title_index + 2] == (
        "choice: W6.B rollback 按实际部署形态二选一：k8s 或 docker-compose"
    )
    assert output_lines.count(output_lines[title_index + 2]) == 1


def test_wave6_commands_only_groups_w1d_rollback_paths(
    monkeypatch,
    capsys,
):
    """W6.B rollback 命令清单必须把 k8s / compose 两条路径分组展示。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    k8s_index = output_lines.index("path: k8s")
    compose_index = output_lines.index("path: docker-compose")
    validate_index = output_lines.index("validate: just wave-1-runtime-evidence-validate")

    assert output_lines.index(
        "readiness: just wave-1-runtime-prereq-rollback-k8s",
    ) > k8s_index
    assert output_lines.index(
        "record: just wave-1-rollback-runtime-evidence-k8s",
    ) > k8s_index
    assert output_lines.index(
        "readiness: just wave-1-runtime-prereq-rollback-compose",
    ) > compose_index
    assert output_lines.index(
        "record: just wave-1-rollback-runtime-evidence-compose",
    ) > compose_index
    assert k8s_index < compose_index < validate_index


def test_wave6_commands_only_keeps_w1d_commands_inside_each_path_group(
    monkeypatch,
    capsys,
):
    """W6.B 每条 readiness/record 命令必须落在对应 path 分组内。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    k8s_index = output_lines.index("path: k8s")
    compose_index = output_lines.index("path: docker-compose")
    validate_index = output_lines.index("validate: just wave-1-runtime-evidence-validate")
    k8s_group = output_lines[k8s_index:compose_index]
    compose_group = output_lines[compose_index:validate_index]

    assert "readiness: just wave-1-runtime-prereq-rollback-k8s" in k8s_group
    assert "readiness: just wave-1-rollback-runtime-readiness-k8s" in k8s_group
    assert "record: just wave-1-rollback-runtime-evidence-k8s" in k8s_group
    assert all("-compose" not in line for line in k8s_group)

    assert "readiness: just wave-1-runtime-prereq-rollback-compose" in compose_group
    assert "readiness: just wave-1-rollback-runtime-readiness-compose" in compose_group
    assert "record: just wave-1-rollback-runtime-evidence-compose" in compose_group
    assert all("-k8s" not in line for line in compose_group)


def test_wave6_commands_only_choice_line_is_scoped_to_w6b(monkeypatch, capsys):
    """W6.B choice 提示不能污染其他缺失 evidence gate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave1_runtime_evidence.py --kind h2": (
                "docs/retros/wave-1-h2-runtime-evidence.json"
            ),
            "validate_wave1_runtime_evidence.py --kind w1d": (
                "docs/retros/wave-1-runtime-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()
    choice_line = report.W6B_ROLLBACK_DEPLOYMENT_CHOICE_LINE
    w6a_index = output_lines.index("# W6.A W6.A-wave1-h2-runtime")
    w6b_index = output_lines.index("# W6.B W6.B-wave1-rollback-runtime")

    assert choice_line not in output_lines[w6a_index:w6b_index]
    assert output_lines.count(choice_line) == 1


def test_wave6_commands_only_order_matches_missing_evidence_details(
    monkeypatch,
    capsys,
):
    """commands-only 多 gate 顺序必须对齐 JSON missing_evidence_details。"""
    import json

    import report_wave6_pre_release as report

    _patch_three_missing_evidence_gates(monkeypatch, report)

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)
    detail_titles = expected_wave6_commands_only_titles(payload)

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_titles = [
        line
        for line in capsys.readouterr().out.splitlines()
        if line.startswith("# ")
    ]

    assert detail_titles == [
        "# W6.A W6.A-wave1-h2-runtime",
        "# W6.D W6.D-wave3-pda-l7",
        "# W6.H W6.H-gray-release",
    ]
    assert output_titles == detail_titles


def test_wave6_commands_only_lists_w6h_materials_audit_before_readiness_and_record(
    monkeypatch,
    capsys,
):
    """W6.H 命令清单必须先取得 audit_event_query_ref，再 readiness/record。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave6_deploy_evidence.py": (
                "docs/retros/wave-6-deploy-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    export_template_line = "readiness: just wave-6-deploy-materials --export-template"
    materials_line = "readiness: just wave-6-deploy-materials --from-env --json"
    readiness_line = "readiness: just wave-6-deploy-readiness --from-env --json"
    audit_check_only_line = "record-check-only: just wave-6-deploy-audit --from-env --check-only"
    record_check_only_line = (
        "record-check-only: just wave-6-deploy-evidence-record --from-env --check-only --json"
    )
    audit_line = "record: just wave-6-deploy-audit --from-env"
    record_line = "record: just wave-6-deploy-evidence-record --from-env --json"
    validate_line = "validate: just wave-6-deploy-evidence-validate"

    assert export_template_line in output_lines
    assert materials_line in output_lines
    assert readiness_line in output_lines
    assert audit_check_only_line in output_lines
    assert record_check_only_line in output_lines
    assert audit_line in output_lines
    assert record_line in output_lines
    assert validate_line in output_lines
    assert output_lines.index(export_template_line) < output_lines.index(materials_line)
    assert output_lines.index(materials_line) < output_lines.index(audit_check_only_line)
    assert output_lines.index(audit_check_only_line) < output_lines.index(audit_line)
    assert output_lines.index(audit_line) < output_lines.index(readiness_line)
    assert output_lines.index(readiness_line) < output_lines.index(record_check_only_line)
    assert output_lines.index(readiness_line) < output_lines.index(record_line)
    assert output_lines.index(record_check_only_line) < output_lines.index(record_line)
    assert output_lines.index(record_line) < output_lines.index(validate_line)


def test_wave6_commands_only_lists_w6e_readiness_before_record(monkeypatch, capsys):
    """W6.E 命令清单必须先只读 readiness，再 record / validate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave4_external_dependencies.py": (
                "docs/retros/wave-4-external-dependencies.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    export_template_line = (
        "readiness: just wave-4-external-dependencies-record --export-template"
    )
    readiness_line = (
        "readiness: just wave-4-external-dependencies-readiness --from-env --json"
    )
    record_line = "record: just wave-4-external-dependencies-record --from-env --json"
    validate_line = "validate: just wave-4-external-dependencies-validate"

    assert export_template_line in output_lines
    assert readiness_line in output_lines
    assert record_line in output_lines
    assert validate_line in output_lines
    assert output_lines.index(export_template_line) < output_lines.index(readiness_line)
    assert output_lines.index(readiness_line) < output_lines.index(record_line)
    assert output_lines.index(record_line) < output_lines.index(validate_line)


def test_wave6_commands_only_lists_recorder_check_only_for_w6d_to_w6g(
    monkeypatch,
    capsys,
):
    """W6.D/E/F/G 命令清单必须直接列出 record 前 check-only 预检。"""
    import report_wave6_pre_release as report

    cases = [
        (
            "validate_wave3_pda_runtime_evidence.py",
            "docs/retros/wave-3-pda-runtime-evidence.json",
            "readiness: just wave-3-pda-runtime-readiness --from-env --json",
            "readiness: just wave-3-pda-runtime-evidence-record --export-template",
            "record-check-only: just wave-3-pda-runtime-evidence-record --from-env --check-only --json",
            "record: just wave-3-pda-runtime-evidence-record --from-env --json",
            "validate: just wave-3-pda-runtime-evidence-validate",
        ),
        (
            "validate_wave4_external_dependencies.py",
            "docs/retros/wave-4-external-dependencies.json",
            "readiness: just wave-4-external-dependencies-readiness --from-env --json",
            "readiness: just wave-4-external-dependencies-record --export-template",
            "record-check-only: just wave-4-external-dependencies-record --from-env --check-only --json",
            "record: just wave-4-external-dependencies-record --from-env --json",
            "validate: just wave-4-external-dependencies-validate",
        ),
        (
            "validate_wave5_hardware_evidence.py",
            "docs/retros/wave-5-hardware-evidence.json",
            "readiness: just wave-5-hardware-readiness --from-env --json",
            "readiness: just wave-5-hardware-materials --export-template",
            "record-check-only: just wave-5-hardware-evidence-record --from-env --check-only --json",
            "record: just wave-5-hardware-evidence-record --from-env --json",
            "validate: just wave-5-hardware-evidence-validate",
        ),
        (
            "validate_wave5_tms_evidence.py",
            "docs/retros/wave-5-tms-evidence.json",
            "readiness: just wave-5-tms-readiness --from-env --json",
            "readiness: just wave-5-tms-materials --export-template",
            "record-check-only: just wave-5-tms-evidence-record --from-env --check-only --json",
            "record: just wave-5-tms-evidence-record --from-env --json",
            "validate: just wave-5-tms-evidence-validate",
        ),
    ]

    for validator, evidence_file, readiness, export_template, check_only, record, validate in cases:
        patch_wave6_report_io(
            monkeypatch,
            report,
            run_validator=wave6_missing_file_validator({validator: evidence_file}),
        )

        assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
        output_lines = capsys.readouterr().out.splitlines()

        assert export_template in output_lines
        assert readiness in output_lines
        assert check_only in output_lines
        assert record in output_lines
        assert validate in output_lines
        if validator == "validate_wave3_pda_runtime_evidence.py":
            assert "readiness: just wave-3-pda-materials-checklist --json" in output_lines
            assert "readiness: just wave-3-pda-field-work-request" in output_lines
            assert "readiness: just wave-3-pda-field-execution-summary --json" in output_lines
            assert "record-check-only: just wave-3-pda-intake-check --json" in output_lines
            assert "record: just wave-3-pda-intake-record --json" in output_lines
            field_precheck_summary = (
                "readiness: just wave-3-pda-field-precheck-summary --from-env --json"
            )
            field_owner_gap_actions = (
                "readiness: just wave-3-pda-field-owner-gap-actions --json"
            )
            field_handoff_bundle = (
                "readiness: just wave-3-pda-field-handoff-bundle --json"
            )
            assert field_precheck_summary in output_lines
            assert field_owner_gap_actions in output_lines
            assert field_handoff_bundle in output_lines
            assert "readiness: just wave-3-pda-evidence-package-template" in output_lines
            service_precheck = "readiness: just wave-3-pda-service-precheck --from-env --json"
            trace_code_precheck = (
                "readiness: just wave-3-pda-trace-code-openapi-precheck --from-env --json"
            )
            assert service_precheck in output_lines
            assert trace_code_precheck in output_lines
            assert output_lines.index(
                "readiness: just wave-3-pda-materials-checklist --json",
            ) < output_lines.index("readiness: just wave-3-pda-field-work-request")
            assert output_lines.index(
                "readiness: just wave-3-pda-field-work-request",
            ) < output_lines.index("readiness: just wave-3-pda-field-execution-summary --json")
            assert output_lines.index(
                "readiness: just wave-3-pda-field-execution-summary --json",
            ) < output_lines.index(field_precheck_summary)
            assert output_lines.index(field_precheck_summary) < output_lines.index(
                field_owner_gap_actions,
            )
            assert output_lines.index(field_owner_gap_actions) < output_lines.index(
                field_handoff_bundle,
            )
            assert output_lines.index(field_handoff_bundle) < output_lines.index(
                "readiness: just wave-3-pda-evidence-package-template",
            )
            assert output_lines.index(
                "readiness: just wave-3-pda-evidence-package-template",
            ) < output_lines.index(service_precheck)
            assert output_lines.index(service_precheck) < output_lines.index(
                trace_code_precheck,
            )
            assert output_lines.index(trace_code_precheck) < output_lines.index(
                export_template,
            )
        assert output_lines.index(export_template) < output_lines.index(readiness)
        assert output_lines.index(readiness) < output_lines.index(check_only)
        assert output_lines.index(check_only) < output_lines.index(record)
        assert output_lines.index(record) < output_lines.index(validate)


def test_wave6_commands_only_lists_w6f_materials_before_readiness_and_record(
    monkeypatch,
    capsys,
):
    """W6.F 硬件命令清单必须先 materials/readiness，再 record/validate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave5_hardware_evidence.py": (
                "docs/retros/wave-5-hardware-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    export_template_line = "readiness: just wave-5-hardware-materials --export-template"
    materials_line = "readiness: just wave-5-hardware-materials --from-env --json"
    readiness_line = "readiness: just wave-5-hardware-readiness --from-env --json"
    check_only_line = (
        "record-check-only: "
        "just wave-5-hardware-evidence-record --from-env --check-only --json"
    )
    record_line = "record: just wave-5-hardware-evidence-record --from-env --json"
    validate_line = "validate: just wave-5-hardware-evidence-validate"

    assert export_template_line in output_lines
    assert materials_line in output_lines
    assert readiness_line in output_lines
    assert check_only_line in output_lines
    assert record_line in output_lines
    assert validate_line in output_lines
    assert output_lines.index(export_template_line) < output_lines.index(materials_line)
    assert output_lines.index(materials_line) < output_lines.index(readiness_line)
    assert output_lines.index(readiness_line) < output_lines.index(check_only_line)
    assert output_lines.index(check_only_line) < output_lines.index(record_line)
    assert output_lines.index(record_line) < output_lines.index(validate_line)


def test_wave6_commands_only_lists_w6g_materials_before_readiness_and_record(
    monkeypatch,
    capsys,
):
    """W6.G TMS 命令清单必须先 materials/readiness，再 record/validate。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(
        monkeypatch,
        report,
        run_validator=wave6_missing_file_validator({
            "validate_wave5_tms_evidence.py": (
                "docs/retros/wave-5-tms-evidence.json"
            ),
        }),
    )

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    export_template_line = "readiness: just wave-5-tms-materials --export-template"
    materials_line = "readiness: just wave-5-tms-materials --from-env --json"
    readiness_line = "readiness: just wave-5-tms-readiness --from-env --json"
    check_only_line = (
        "record-check-only: "
        "just wave-5-tms-evidence-record --from-env --check-only --json"
    )
    record_line = "record: just wave-5-tms-evidence-record --from-env --json"
    validate_line = "validate: just wave-5-tms-evidence-validate"

    assert export_template_line in output_lines
    assert materials_line in output_lines
    assert readiness_line in output_lines
    assert check_only_line in output_lines
    assert record_line in output_lines
    assert validate_line in output_lines
    assert output_lines.index(export_template_line) < output_lines.index(materials_line)
    assert output_lines.index(materials_line) < output_lines.index(readiness_line)
    assert output_lines.index(readiness_line) < output_lines.index(check_only_line)
    assert output_lines.index(check_only_line) < output_lines.index(record_line)
    assert output_lines.index(record_line) < output_lines.index(validate_line)


def test_wave6_commands_only_lines_match_missing_evidence_details_commands(
    monkeypatch,
    capsys,
):
    """commands-only 的每行命令必须逐项对齐 JSON missing_evidence_details。"""
    import json

    import report_wave6_pre_release as report

    _patch_three_missing_evidence_gates(monkeypatch, report)

    assert report.main(["--strict", "--evidence-only", "--json"]) == 1
    payload = json.loads(capsys.readouterr().out)

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 1
    output_lines = capsys.readouterr().out.splitlines()

    assert output_lines == expected_wave6_commands_only_lines(payload)


def test_wave6_commands_only_evidence_only_ignores_retro(monkeypatch, capsys):
    """只缺 retro 时 commands-only evidence-only 不输出 retro 命令也不阻塞。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 0
    output = capsys.readouterr().out

    assert "Wave 6 missing evidence commands: none" in output
    assert "W6-retro" not in output
    assert "docs/retros/wave-6-retro.md" not in output


def test_wave6_commands_only_none_output_is_single_line(monkeypatch, capsys):
    """无缺失 evidence 时 commands-only 只能输出一行 none，避免误复制空清单。"""
    import report_wave6_pre_release as report

    patch_wave6_report_io(monkeypatch, report)

    assert report.main(["--commands-only", "--strict", "--evidence-only"]) == 0
    output_lines = capsys.readouterr().out.splitlines()

    assert output_lines == [report.COMMANDS_ONLY_NONE_LINE]
    assert "只读命令清单" in output_lines[0]
    assert "不会写入 runtime evidence" in output_lines[0]
    assert "不能关闭 evidence gate" in output_lines[0]
