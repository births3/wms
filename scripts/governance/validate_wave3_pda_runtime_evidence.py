#!/usr/bin/env python3
"""Validate Wave 3 real PDA and L7 evidence JSON."""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from _wave_evidence_validator import blocked_ref_fields, blocked_ref_message

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_EVIDENCE = REPO_ROOT / "docs/retros/wave-3-pda-runtime-evidence.json"

BLOCKED_REF_TOKENS = (
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "local",
    "prod",
    "production",
    "mock",
    "fake",
    "stub",
    "browser",
    "simulator",
    "emulator",
    "phone",
    "camera",
    "example",
)

REQUIRED_REFS = (
    "pda_device_ref",
    "spike005_result_ref",
    "m2_scan_log_ref",
    "m3_scan_log_ref",
    "offline_replay_log_ref",
    "idempotency_replay_log_ref",
    "audit_event_query_ref",
    "l7_run_ref",
    "usability_review_ref",
)

BASE_STRING_FIELDS = (
    "environment",
    "pda_model",
    "android_version",
    "scan_input_method",
    "pda_stack_candidate",
)
TYPED_REF_REQUIREMENTS = (
    ("m2_scan_log_ref", ("m2", "scan"), "M2 scan"),
    ("m3_scan_log_ref", ("m3", "scan"), "M3 scan"),
    ("offline_replay_log_ref", ("offline", "replay"), "offline replay"),
    ("l7_run_ref", ("l7",), "L7"),
    ("usability_review_ref", ("usability", "review"), "usability review"),
)
WEBVIEW_CAPACITOR_REFS = (
    "native_shell_ref",
    "native_scan_plugin_ref",
)
BASE_EVIDENCE_FIELDS = frozenset((
    "environment",
    "pda_model",
    "android_version",
    "scan_input_method",
    "pda_stack_candidate",
    *REQUIRED_REFS,
    "barcode_samples_scanned",
    "m2_operations_exercised",
    "m3_operations_exercised",
    "offline_replays_exercised",
    "idempotency_replays_exercised",
    "real_pda_used",
    "physical_scan_key_verified",
    "dev_or_staging_service_verified",
    "audit_event_verified",
    "l7_review_completed",
    "usability_review_completed",
))
WEBVIEW_CAPACITOR_TYPED_REF_REQUIREMENTS = (
    ("native_shell_ref", ("native", "shell"), "Android native shell"),
    ("native_scan_plugin_ref", ("native", "scan", "plugin"), "native scan plugin"),
)

SPIKE_RESULT_REF_PATTERN = re.compile(r"spike[-_]?005b?", re.IGNORECASE)
SPIKE_005B_RESULT_REF_PATTERN = re.compile(r"spike[-_]?005b", re.IGNORECASE)
SPIKE_005_RESULT_REF_PATTERN = re.compile(r"spike[-_]?005(?!b)", re.IGNORECASE)

PDA_STACK_CANDIDATES = {"react-native", "webview-capacitor"}
MIN_BARCODE_SAMPLES = 50
MIN_REPLAY_COUNT = 50
PHYSICAL_SCAN_METHOD_HINTS = ("scan-key", "keyevent", "intent", "datawedge")
BLOCKED_SCAN_METHOD_TOKENS = ("camera", "phone")
BLOCKED_DEVICE_IDENTITY_TOKENS = (
    "emulator",
    "simulator",
    "browser",
    "phone",
    "mock",
    "fake",
    "stub",
    "example",
)
PLACEHOLDER_TOKENS = (
    "yyyy",
    "<",
    ">",
    "todo",
    "tbd",
    "待填",
    "待确认",
)


def read_json(path: Path) -> tuple[object | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as error:
        return None, f"invalid JSON: {path}: {error}"


def _bad_ref(value: str, *, allow_example_refs: bool) -> bool:
    lowered = value.lower()
    blocked = BLOCKED_REF_TOKENS if not allow_example_refs else BLOCKED_REF_TOKENS[:-1]
    return any(token in lowered for token in blocked)


def _has_environment_token(value: str, environment: str) -> bool:
    return re.search(
        rf"(^|[^0-9a-z]){re.escape(environment)}([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def _positive_int(payload: dict[str, object], key: str) -> bool:
    value = payload.get(key)
    return isinstance(value, int) and not isinstance(value, bool) and value >= 1


def _at_least_int(value: object, minimum: int) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= minimum


def _non_string_fields(payload: dict[str, object], keys: tuple[str, ...]) -> list[str]:
    return [
        key
        for key in keys
        if key in payload and not isinstance(payload.get(key), str)
    ]


def _contains_all(value: str, tokens: tuple[str, ...]) -> bool:
    lowered = value.lower()
    return all(token in lowered for token in tokens)


def _placeholder_fields(payload: dict[str, object], keys: tuple[str, ...]) -> list[str]:
    fields = []
    for key in keys:
        value = payload.get(key)
        if not isinstance(value, str):
            continue
        lowered = value.lower()
        if any(token in lowered for token in PLACEHOLDER_TOKENS):
            fields.append(key)
    return fields


def validate_wave3_pda_runtime_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "Wave 3 PDA evidence 顶层必须是 object"

    non_string_base_fields = _non_string_fields(payload, BASE_STRING_FIELDS)
    if non_string_base_fields:
        return False, f"字符串字段必须是 string: {', '.join(non_string_base_fields)}"

    environment = payload.get("environment", "").lower()
    if environment not in {"dev", "staging"}:
        return False, "environment 必须是真实 dev 或 staging，不能是 local/prod/production/mock/fake/stub/example"

    for key in ("pda_model", "android_version", "scan_input_method"):
        if not str(payload.get(key, "")).strip():
            return False, f"{key} 必须记录真实 PDA 设备信息"

    for key in ("pda_model", "android_version"):
        value = str(payload.get(key, "")).lower()
        if any(token in value for token in BLOCKED_DEVICE_IDENTITY_TOKENS):
            return False, f"{key} 必须是真 PDA 设备信息，不能包含模拟器、浏览器、手机或测试占位线索"

    scan_input_method = str(payload.get("scan_input_method", "")).lower()
    if _bad_ref(scan_input_method, allow_example_refs=allow_example_refs):
        return False, "scan_input_method 不能是 browser/simulator/emulator/phone/camera/mock/fake/stub/example"
    if any(token in scan_input_method for token in BLOCKED_SCAN_METHOD_TOKENS):
        return False, "scan_input_method 不能使用 camera/phone 替代 PDA 实体扫码键"
    if not any(hint in scan_input_method for hint in PHYSICAL_SCAN_METHOD_HINTS):
        return False, "scan_input_method 必须记录 PDA 实体扫码键或厂商扫码通道，例如 scan-key/keyevent/intent/datawedge"

    pda_stack_candidate = str(payload.get("pda_stack_candidate", "")).strip().lower()
    if pda_stack_candidate not in PDA_STACK_CANDIDATES:
        return False, "pda_stack_candidate 必须是 react-native 或 webview-capacitor"

    allowed_fields = BASE_EVIDENCE_FIELDS
    if pda_stack_candidate == "webview-capacitor":
        allowed_fields = BASE_EVIDENCE_FIELDS | frozenset(WEBVIEW_CAPACITOR_REFS)
    unknown_fields = sorted(set(payload) - allowed_fields)
    if unknown_fields:
        return False, (
            f"pda_stack_candidate={pda_stack_candidate} 的 evidence JSON 包含未知字段: "
            f"{', '.join(unknown_fields)}"
        )

    candidate_refs = REQUIRED_REFS
    if pda_stack_candidate == "webview-capacitor":
        candidate_refs = (*REQUIRED_REFS, *WEBVIEW_CAPACITOR_REFS)

    non_string_ref_fields = _non_string_fields(payload, candidate_refs)
    if non_string_ref_fields:
        return False, f"字符串字段必须是 string: {', '.join(non_string_ref_fields)}"

    missing_refs = [key for key in candidate_refs if not payload.get(key)]
    if missing_refs:
        return False, f"缺少必需证据引用: {', '.join(missing_refs)}"

    placeholder_fields = _placeholder_fields(
        payload,
        (
            "pda_model",
            "android_version",
            "scan_input_method",
            "pda_stack_candidate",
            *candidate_refs,
        ),
    )
    if placeholder_fields:
        return False, f"真实 PDA runtime evidence 不能保留模板占位: {', '.join(placeholder_fields)}"

    bad_ref_fields = blocked_ref_fields(
        payload,
        candidate_refs,
        is_bad_ref=_bad_ref,
        allow_example_refs=allow_example_refs,
    )
    if bad_ref_fields:
        return False, blocked_ref_message(
            "local/prod/production/mock/fake/stub/example/browser/simulator/emulator/phone/camera",
            bad_ref_fields,
        )

    pda_device_ref = str(payload.get("pda_device_ref", "")).lower()
    if not pda_device_ref.startswith("asset://") or "/pda/" not in pda_device_ref:
        return False, "pda_device_ref 必须是 PDA 设备资产引用，例如 asset://<环境>/pda/<设备编号>"

    audit_event_query_ref = str(payload.get("audit_event_query_ref", ""))
    if not _contains_all(audit_event_query_ref, ("audit", "event")):
        return False, "audit_event_query_ref 必须指向 H2 audit_event 查询证据"

    idempotency_replay_log_ref = str(payload.get("idempotency_replay_log_ref", ""))
    if not _contains_all(idempotency_replay_log_ref, ("idempotency", "replay")):
        return False, "idempotency_replay_log_ref 必须指向 Idempotency-Key replay 证据"

    for key, tokens, label in TYPED_REF_REQUIREMENTS:
        if not _contains_all(str(payload.get(key, "")), tokens):
            return False, f"{key} 必须指向 {label} 证据"
    if pda_stack_candidate == "webview-capacitor":
        for key, tokens, label in WEBVIEW_CAPACITOR_TYPED_REF_REQUIREMENTS:
            if not _contains_all(str(payload.get(key, "")), tokens):
                return False, f"{key} 必须指向 {label} 证据"

    missing_environment_refs = [
        key
        for key in candidate_refs
        if not _has_environment_token(str(payload.get(key, "")), environment)
    ]
    if missing_environment_refs:
        return False, f"证据引用必须包含 environment 标记 {environment}: {', '.join(missing_environment_refs)}"

    spike_result_ref = str(payload.get("spike005_result_ref", ""))
    if SPIKE_RESULT_REF_PATTERN.search(spike_result_ref) is None:
        return False, "spike005_result_ref 必须指向 SPIKE-005 或 SPIKE-005B 实测结果"
    if pda_stack_candidate == "react-native" and SPIKE_005_RESULT_REF_PATTERN.search(spike_result_ref) is None:
        return False, "pda_stack_candidate=react-native 必须指向 SPIKE-005 实测结果"
    if pda_stack_candidate == "webview-capacitor" and SPIKE_005B_RESULT_REF_PATTERN.search(spike_result_ref) is None:
        return False, "pda_stack_candidate=webview-capacitor 必须指向 SPIKE-005B 实测结果"

    barcode_samples = payload.get("barcode_samples_scanned")
    if not _at_least_int(barcode_samples, MIN_BARCODE_SAMPLES):
        return False, f"barcode_samples_scanned 必须 >= {MIN_BARCODE_SAMPLES}，对齐 SPIKE-005 / 005B 条码样本口径"

    insufficient_replay_counts = [
        key
        for key in ("offline_replays_exercised", "idempotency_replays_exercised")
        if not _at_least_int(payload.get(key), MIN_REPLAY_COUNT)
    ]
    if insufficient_replay_counts:
        return False, (
            f"replay 计数必须 >= {MIN_REPLAY_COUNT}，对齐 SPIKE-005 / 005B 离线任务口径: "
            f"{', '.join(insufficient_replay_counts)}"
        )

    required_counts = (
        "m2_operations_exercised",
        "m3_operations_exercised",
    )
    invalid_counts = [key for key in required_counts if not _positive_int(payload, key)]
    if invalid_counts:
        return False, f"计数必须 >= 1: {', '.join(invalid_counts)}"

    required_flags = (
        "real_pda_used",
        "physical_scan_key_verified",
        "dev_or_staging_service_verified",
        "audit_event_verified",
        "l7_review_completed",
        "usability_review_completed",
    )
    invalid_flags = [key for key in required_flags if payload.get(key) is not True]
    if invalid_flags:
        return False, f"布尔证据必须为 true: {', '.join(invalid_flags)}"

    return True, "Wave 3 PDA runtime evidence 内容有效"


def validate_one(path: Path, *, allow_example_refs: bool) -> tuple[bool, str]:
    payload, error = read_json(path)
    if error:
        return False, error
    ok, message = validate_wave3_pda_runtime_payload(
        payload,
        allow_example_refs=allow_example_refs,
    )
    return ok, f"{path}: {message}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-file", default=DEFAULT_EVIDENCE, type=Path)
    parser.add_argument(
        "--allow-example-refs",
        action="store_true",
        help=(
            "Allow refs containing example domain tokens when validating templates; "
            "template placeholders are still rejected."
        ),
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    ok, message = validate_one(
        args.evidence_file,
        allow_example_refs=args.allow_example_refs,
    )

    if args.json:
        print(json.dumps({
            "ok": ok,
            "path": str(args.evidence_file),
            "evidence_file": str(args.evidence_file),
            "writes_runtime_evidence": False,
            "closes_gate": False,
            "message": message,
        }, ensure_ascii=False, indent=2))
    else:
        mark = "✓" if ok else "✘"
        print(f"{mark} {message}")

    return 0 if ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001
        print(f"script error: {error}", file=sys.stderr)
        sys.exit(2)
