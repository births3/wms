#!/usr/bin/env python3
"""Record Wave 3 real PDA and L7 runtime evidence."""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from _wave_evidence_recorder import check_only_result
from check_wave3_pda_runtime_readiness import missing_env_var_owner_details
from validate_wave3_pda_runtime_evidence import (
    DEFAULT_EVIDENCE,
    validate_wave3_pda_runtime_payload,
)

from _wave3_pda_runtime_evidence_templates import *  # noqa: F403


def build_payload(args: argparse.Namespace) -> dict[str, Any]:
    payload = {
        "environment": args.environment,
        "pda_model": args.pda_model,
        "android_version": args.android_version,
        "scan_input_method": args.scan_input_method,
        "pda_stack_candidate": args.pda_stack_candidate,
        "pda_device_ref": args.pda_device_ref,
        "spike005_result_ref": args.spike005_result_ref,
        "m2_scan_log_ref": args.m2_scan_log_ref,
        "m3_scan_log_ref": args.m3_scan_log_ref,
        "offline_replay_log_ref": args.offline_replay_log_ref,
        "idempotency_replay_log_ref": args.idempotency_replay_log_ref,
        "audit_event_query_ref": args.audit_event_query_ref,
        "l7_run_ref": args.l7_run_ref,
        "usability_review_ref": args.usability_review_ref,
        "barcode_samples_scanned": args.barcode_samples_scanned,
        "m2_operations_exercised": args.m2_operations_exercised,
        "m3_operations_exercised": args.m3_operations_exercised,
        "offline_replays_exercised": args.offline_replays_exercised,
        "idempotency_replays_exercised": args.idempotency_replays_exercised,
        "real_pda_used": args.real_pda_used,
        "physical_scan_key_verified": args.physical_scan_key_verified,
        "dev_or_staging_service_verified": args.dev_or_staging_service_verified,
        "audit_event_verified": args.audit_event_verified,
        "l7_review_completed": args.l7_review_completed,
        "usability_review_completed": args.usability_review_completed,
    }
    if args.native_shell_ref:
        payload["native_shell_ref"] = args.native_shell_ref
    if args.native_scan_plugin_ref:
        payload["native_scan_plugin_ref"] = args.native_scan_plugin_ref
    return payload


def write_payload(path: Path, payload: dict[str, Any], *, force: bool) -> tuple[bool, str]:
    ok, message = validate_wave3_pda_runtime_payload(payload)
    if not ok:
        return False, message

    if path.exists() and not force:
        return False, f"{path} already exists; pass --force to overwrite"

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"


def check_payload(payload: dict[str, Any]) -> tuple[bool, str]:
    ok, message = validate_wave3_pda_runtime_payload(payload)
    if ok:
        return True, f"check-only passed: {message}"
    return False, message


def missing_required_args(args: argparse.Namespace) -> list[str]:
    missing: list[str] = []
    for field in STRING_ARGS:
        value = str(getattr(args, field, "") or "").strip()
        if not value:
            missing.append(f"--{field.replace('_', '-')}")
    for field in INT_ARGS:
        if getattr(args, field, None) is None:
            missing.append(f"--{field.replace('_', '-')}")
    if args.pda_stack_candidate == "webview-capacitor":
        if not (args.native_shell_ref or "").strip():
            missing.append("--native-shell-ref")
        if not (args.native_scan_plugin_ref or "").strip():
            missing.append("--native-scan-plugin-ref")
    return missing


def false_flag_env_vars(args: argparse.Namespace) -> list[str]:
    return [
        env_name
        for field, env_name in ENV_FLAG_ARGS.items()
        if getattr(args, field) is not True
    ]


def apply_env_args(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    for field, env_name in ENV_STRING_ARGS.items():
        value = os.environ.get(env_name)
        if value is not None:
            setattr(args, field, value.strip())

    for field, env_name in ENV_INT_ARGS.items():
        value = os.environ.get(env_name)
        if value is None:
            continue
        try:
            parsed = int(value.strip())
        except ValueError:
            issues.append(f"{env_name} must be an integer")
            continue
        if parsed <= 0:
            issues.append(f"{env_name} must be > 0")
            continue
        setattr(args, field, parsed)

    for field, env_name in ENV_FLAG_ARGS.items():
        raw_value = os.environ.get(env_name, "")
        value = raw_value.strip().lower()
        if value in TRUE_ENV_VALUES:
            setattr(args, field, True)
        elif value in FALSE_ENV_VALUES:
            setattr(args, field, False)
        else:
            issues.append(f"{env_name} must be true or false")
    return issues


def print_export_template() -> None:
    print(EXPORT_TEMPLATE.rstrip())


def print_package_template() -> None:
    print(PACKAGE_TEMPLATE.rstrip())


def display_evidence_file(path: Path) -> str:
    if path.resolve() == DEFAULT_EVIDENCE.resolve():
        return DEFAULT_EVIDENCE_DISPLAY
    return str(path)


def package_template_payload(evidence_file: Path) -> dict[str, object]:
    return {
        "ok": True,
        "mode": "wave3-pda-evidence-package-template",
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_evidence_file(evidence_file),
        "sections": [
            {
                "id": section["id"],
                "title": section["title"],
                "fields": list(section["fields"]),
            }
            for section in PACKAGE_TEMPLATE_SECTIONS
        ],
        "mapping_variables": list(PACKAGE_MAPPING_VARIABLES),
        "blocked_flags_until_refs_present": list(PACKAGE_BLOCKED_FLAGS),
        "owner_actions": [
            {
                "owner": str(action["owner"]),
                "action": str(action["action"]),
                "required_env_vars": list(action["required_env_vars"]),
                "acceptance": str(action["acceptance"]),
                "can_write_runtime_evidence": bool(action["can_write_runtime_evidence"]),
            }
            for action in PACKAGE_OWNER_ACTIONS
        ],
        "record_gate_after_owner_actions": list(PACKAGE_RECORD_GATE_AFTER_OWNER_ACTIONS),
        "warnings": list(PACKAGE_TEMPLATE_WARNINGS),
    }


def intake_template_evidence() -> dict[str, object]:
    payload: dict[str, object] = {
        "environment": "staging",
        "pda_model": "",
        "android_version": "",
        "scan_input_method": "",
        "pda_stack_candidate": "react-native",
        "pda_device_ref": "",
        "spike005_result_ref": "",
        "m2_scan_log_ref": "",
        "m3_scan_log_ref": "",
        "offline_replay_log_ref": "",
        "idempotency_replay_log_ref": "",
        "audit_event_query_ref": "",
        "l7_run_ref": "",
        "usability_review_ref": "",
        "barcode_samples_scanned": 50,
        "m2_operations_exercised": 1,
        "m3_operations_exercised": 1,
        "offline_replays_exercised": 50,
        "idempotency_replays_exercised": 50,
        "real_pda_used": False,
        "physical_scan_key_verified": False,
        "dev_or_staging_service_verified": False,
        "audit_event_verified": False,
        "l7_review_completed": False,
        "usability_review_completed": False,
        "native_shell_ref": "",
        "native_scan_plugin_ref": "",
    }
    return payload


def intake_template_payload(evidence_file: Path) -> dict[str, object]:
    return {
        "ok": True,
        "mode": "wave3-pda-runtime-evidence-intake-template",
        "schema_version": 1,
        "kind": INTAKE_KIND,
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": display_evidence_file(evidence_file),
        "instructions": [
            "Fill evidence with real dev/staging + real PDA refs.",
            "Run with --from-intake-file <path> --check-only --json before record.",
            "Do not paste trace-code API keys into this intake file.",
            "Set truth flags to true only after matching real evidence refs are present.",
            (
                "Empty string values and false truth flags mean field evidence is still "
                "missing and must be filled by the assigned owner."
            ),
        ],
        "required_evidence_fields": list(STRING_ARGS) + list(INT_ARGS) + list(ENV_FLAG_ARGS),
        "webview_capacitor_evidence_fields": [
            "native_shell_ref",
            "native_scan_plugin_ref",
        ],
        "evidence": intake_template_evidence(),
        "record_gate_after_intake": list(INTAKE_RECORD_GATE_AFTER_INTAKE),
    }


def write_intake_template(
    path: Path,
    payload: dict[str, object],
    *,
    force: bool = False,
) -> tuple[bool, str]:
    if path.exists() and not force:
        return False, f"{path} already exists; pass --intake-template-force to overwrite"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return True, f"wrote {path}"


def load_intake_evidence(path: Path) -> dict[str, object]:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"failed to read intake file {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"intake file must be valid JSON: {error}") from error

    if not isinstance(raw, dict):
        raise ValueError("intake file must contain a JSON object")
    if type(raw.get("schema_version")) is not int or raw["schema_version"] != 1:
        raise ValueError("intake schema_version is required and must be 1")
    if raw.get("kind") != INTAKE_KIND:
        raise ValueError(f"intake kind is required and must be {INTAKE_KIND}")
    if raw.get("writes_runtime_evidence") is not False:
        raise ValueError("intake writes_runtime_evidence is required and must be false")
    if raw.get("closes_gate") is not False:
        raise ValueError("intake closes_gate is required and must be false")
    if "evidence" not in raw:
        raise ValueError("intake evidence is required")
    evidence = raw["evidence"]
    if not isinstance(evidence, dict):
        raise ValueError("intake evidence must be a JSON object")
    unknown_fields = sorted(set(evidence) - INTAKE_EVIDENCE_FIELDS)
    if unknown_fields:
        raise ValueError(
            "intake evidence contains unknown fields: "
            f"{', '.join(unknown_fields)}",
        )
    non_string_fields = [
        field
        for field in sorted(INTAKE_STRING_FIELDS & set(evidence))
        if not isinstance(evidence[field], str)
    ]
    if non_string_fields:
        raise ValueError(
            "intake evidence string fields must be JSON strings: "
            + "; ".join(f"{field} must be a JSON string" for field in non_string_fields),
        )
    non_int_fields = [
        field
        for field in sorted(INTAKE_INT_FIELDS & set(evidence))
        if type(evidence[field]) is not int
    ]
    if non_int_fields:
        raise ValueError(
            "intake evidence integer fields must be JSON integers: "
            + "; ".join(f"{field} must be a JSON integer" for field in non_int_fields),
        )
    non_bool_fields = [
        field
        for field in sorted(INTAKE_BOOL_FIELDS & set(evidence))
        if not isinstance(evidence[field], bool)
    ]
    if non_bool_fields:
        raise ValueError(
            "intake evidence boolean fields must be JSON booleans: "
            + "; ".join(f"{field} must be a JSON boolean" for field in non_bool_fields),
        )
    return evidence


def apply_intake_args(args: argparse.Namespace) -> list[str]:
    issues: list[str] = []
    evidence = load_intake_evidence(args.from_intake_file)

    for field in (*STRING_ARGS, "native_shell_ref", "native_scan_plugin_ref"):
        if field not in evidence:
            continue
        value = evidence[field]
        setattr(args, field, value.strip())

    for field in INT_ARGS:
        if field not in evidence:
            continue
        parsed = evidence[field]
        if parsed <= 0:
            issues.append(f"{field} must be > 0")
            continue
        setattr(args, field, parsed)

    for field in ENV_FLAG_ARGS:
        if field not in evidence:
            continue
        setattr(args, field, evidence[field])

    return issues


def report_input_error(
    parser: argparse.ArgumentParser,
    args: argparse.Namespace,
    message: str,
    *,
    missing_args: list[str] | None = None,
) -> int:
    if args.json:
        payload = check_only_result(False, message, args.output)
        payload["check_only"] = bool(args.check_only)
        if missing_args:
            payload["missing_args"] = missing_args
            missing_env_vars = [
                CLI_ARG_TO_ENV[arg]
                for arg in missing_args
                if arg in CLI_ARG_TO_ENV
            ]
            payload["missing_env_vars"] = missing_env_vars
            payload["missing_env_var_owners"] = missing_env_var_owner_details(
                missing_env_vars,
            )
            false_flags = false_flag_env_vars(args)
            if false_flags:
                payload["false_flag_env_vars"] = false_flags
                payload["false_flag_env_var_owners"] = missing_env_var_owner_details(
                    false_flags,
                )
        print(
            json.dumps(
                payload,
                ensure_ascii=False,
                indent=2,
            ),
        )
        return 2
    parser.error(message)
    return 2


def record_result(ok: bool, message: str, evidence_file: Path) -> dict[str, object]:
    return {
        "ok": ok,
        "check_only": False,
        "writes_runtime_evidence": ok,
        "closes_gate": False,
        "evidence_file": str(evidence_file),
        "message": message,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--export-template",
        action="store_true",
        help="Print a shell template for collecting real Wave 3 PDA evidence refs.",
    )
    parser.add_argument(
        "--export-package-template",
        action="store_true",
        help="Print a Markdown evidence package template without writing evidence.",
    )
    parser.add_argument(
        "--export-intake-template",
        action="store_true",
        help="Print a JSON intake template for field evidence without writing evidence.",
    )
    parser.add_argument(
        "--intake-template-output",
        type=Path,
        help=(
            "Write the JSON intake template to this path. Only valid with "
            "--export-intake-template; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--intake-template-force",
        action="store_true",
        help=(
            "Overwrite an existing --intake-template-output file. Only valid with "
            "--export-intake-template; does not write runtime evidence."
        ),
    )
    parser.add_argument(
        "--check-only",
        action="store_true",
        help="Validate fields, refs, and boundaries without writing evidence.",
    )
    parser.add_argument(
        "--from-env",
        action="store_true",
        help="Read WAVE_3_PDA_* variables from the exported evidence template.",
    )
    parser.add_argument(
        "--from-intake-file",
        type=Path,
        help="Read Wave 3 PDA evidence fields from a JSON field intake file.",
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--environment", choices=["dev", "staging"])
    parser.add_argument("--pda-model")
    parser.add_argument("--android-version")
    parser.add_argument("--scan-input-method")
    parser.add_argument(
        "--pda-stack-candidate",
        choices=["react-native", "webview-capacitor"],
    )
    parser.add_argument("--pda-device-ref")
    parser.add_argument("--spike005-result-ref")
    parser.add_argument("--m2-scan-log-ref")
    parser.add_argument("--m3-scan-log-ref")
    parser.add_argument("--offline-replay-log-ref")
    parser.add_argument("--idempotency-replay-log-ref")
    parser.add_argument("--audit-event-query-ref")
    parser.add_argument("--l7-run-ref")
    parser.add_argument("--usability-review-ref")
    parser.add_argument("--native-shell-ref")
    parser.add_argument("--native-scan-plugin-ref")
    parser.add_argument("--barcode-samples-scanned", type=int)
    parser.add_argument("--m2-operations-exercised", type=int)
    parser.add_argument("--m3-operations-exercised", type=int)
    parser.add_argument("--offline-replays-exercised", type=int)
    parser.add_argument("--idempotency-replays-exercised", type=int)
    parser.add_argument("--real-pda-used", action="store_true")
    parser.add_argument("--physical-scan-key-verified", action="store_true")
    parser.add_argument("--dev-or-staging-service-verified", action="store_true")
    parser.add_argument("--audit-event-verified", action="store_true")
    parser.add_argument("--l7-review-completed", action="store_true")
    parser.add_argument("--usability-review-completed", action="store_true")
    args = parser.parse_args(argv)

    if args.intake_template_output and not args.export_intake_template:
        return report_input_error(
            parser,
            args,
            "--intake-template-output requires --export-intake-template",
        )
    if args.intake_template_force and not args.intake_template_output:
        return report_input_error(
            parser,
            args,
            "--intake-template-force requires --intake-template-output",
        )

    if args.export_template:
        print_export_template()
        return 0
    if args.export_package_template:
        if args.json:
            print(json.dumps(
                package_template_payload(args.output),
                ensure_ascii=False,
                indent=2,
            ))
        else:
            print_package_template()
        return 0
    if args.export_intake_template:
        payload = intake_template_payload(args.output)
        payload["writes_intake_template"] = False
        if args.intake_template_output:
            payload["intake_template_output"] = str(args.intake_template_output)
            file_payload = {
                **payload,
                "writes_intake_template": True,
            }
            ok_to_write, write_message = write_intake_template(
                args.intake_template_output,
                file_payload,
                force=args.intake_template_force,
            )
            payload["writes_intake_template"] = ok_to_write
            payload["message"] = write_message
            if not ok_to_write:
                payload["ok"] = False
            else:
                payload = file_payload | {
                    "message": write_message,
                }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0 if payload["ok"] else 1

    if args.from_env and args.from_intake_file:
        return report_input_error(
            parser,
            args,
            "--from-env and --from-intake-file cannot be used together",
        )

    if args.from_intake_file:
        try:
            intake_issues = apply_intake_args(args)
        except ValueError as error:
            return report_input_error(parser, args, str(error))
        if intake_issues:
            return report_input_error(parser, args, "; ".join(intake_issues))

    if args.from_env:
        env_issues = apply_env_args(args)
        if env_issues:
            return report_input_error(parser, args, "; ".join(env_issues))

    missing = missing_required_args(args)
    if missing:
        return report_input_error(
            parser,
            args,
            f"the following arguments are required: {', '.join(missing)}",
            missing_args=missing,
        )

    payload = build_payload(args)
    if args.check_only:
        ok, message = check_payload(payload)
        if ok:
            message = (
                f"{message}; no PDA runtime evidence JSON written; "
                "W6.D gate remains open"
            )
        false_flags = false_flag_env_vars(args)
    else:
        ok, message = write_payload(args.output, payload, force=args.force)
        false_flags = false_flag_env_vars(args)
    if args.json:
        payload = (
            check_only_result(ok, message, args.output)
            if args.check_only
            else record_result(ok, message, args.output)
        )
        if not ok and false_flags:
            payload["false_flag_env_vars"] = false_flags
            payload["false_flag_env_var_owners"] = missing_env_var_owner_details(
                false_flags,
            )
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        mark = "✓" if ok else "✘"
        stream = sys.stdout if ok else sys.stderr
        print(f"{mark} {message}", file=stream)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
