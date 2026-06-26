"""Shared fixtures for PDA governance tests."""
import json
from pathlib import Path


def valid_wave3_pda_evidence(candidate: str = "react-native") -> dict[str, object]:
    payload: dict[str, object] = {
        "environment": "staging",
        "pda_model": "Honeywell EDA52",
        "android_version": "Android 11",
        "scan_input_method": "physical-scan-key-intent",
        "pda_stack_candidate": candidate,
        "pda_device_ref": "asset://wms-staging/pda/honeywell-eda52-01",
        "spike005_result_ref": "s3://wms-staging-evidence/wave3/pda/spike-005-runtime-20260604.md",
        "m2_scan_log_ref": "ci/staging/wave3-pda-m2-scan/123",
        "m3_scan_log_ref": "ci/staging/wave3-pda-m3-scan/123",
        "offline_replay_log_ref": "ci/staging/wave3-pda-offline-replay/123",
        "idempotency_replay_log_ref": "ci/staging/wave3-pda-idempotency-replay/123",
        "audit_event_query_ref": "ci/staging/wave3-pda-audit-event/123",
        "l7_run_ref": "ci/staging/wave3-pda-l7/123",
        "usability_review_ref": "s3://wms-staging-evidence/wave3/pda/usability-review.md",
        "barcode_samples_scanned": 50,
        "m2_operations_exercised": 1,
        "m3_operations_exercised": 1,
        "offline_replays_exercised": 50,
        "idempotency_replays_exercised": 50,
        "real_pda_used": True,
        "physical_scan_key_verified": True,
        "dev_or_staging_service_verified": True,
        "audit_event_verified": True,
        "l7_review_completed": True,
        "usability_review_completed": True,
    }
    if candidate == "webview-capacitor":
        payload["spike005_result_ref"] = (
            "s3://wms-staging-evidence/wave3/pda/spike-005b-runtime-20260606.md"
        )
        payload["native_shell_ref"] = "ci/staging/wave3-pda-native-shell-webview-capacitor/123"
        payload["native_scan_plugin_ref"] = "ci/staging/wave3-pda-native-scan-plugin/123"
    return payload


def write_accepted_adr(tmp_path: Path, text: str = "- 状态：Accepted\n") -> None:
    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True, exist_ok=True)
    adr.write_text(text, encoding="utf-8")


def write_proposed_adr(tmp_path: Path) -> None:
    adr = tmp_path / "docs/adr/0027-pda-offline-model.md"
    adr.parent.mkdir(parents=True, exist_ok=True)
    adr.write_text("- 状态：Proposed\n", encoding="utf-8")


def write_wave3_pda_evidence(tmp_path: Path, candidate: str = "react-native") -> None:
    evidence = tmp_path / "docs/retros/wave-3-pda-runtime-evidence.json"
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(valid_wave3_pda_evidence(candidate)), encoding="utf-8")


def write_evidence(path: Path, payload: dict[str, object]) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def write_package_json(tmp_path: Path, rel_path: str, payload: dict[str, object]) -> Path:
    manifest = tmp_path / rel_path
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps(payload), encoding="utf-8")
    return manifest


def write_pnpm_lockfile(tmp_path: Path, lines: list[str]) -> Path:
    lockfile = tmp_path / "pnpm-lock.yaml"
    lockfile.write_text("\n".join(lines), encoding="utf-8")
    return lockfile


def write_rn_spike(tmp_path: Path, status: str = "accepted") -> None:
    rn_spike = tmp_path / "docs/spikes/spike-005-rn-scanner.md"
    rn_spike.parent.mkdir(parents=True, exist_ok=True)
    rn_spike.write_text(
        "\n".join([
            f"- 状态：{status}",
            "",
            "## 实测结果",
            "",
            "SPIKE-005 react-native 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json",
            "覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。",
        ]),
        encoding="utf-8",
    )


def write_webview_spike(tmp_path: Path, status: str = "accepted") -> None:
    webview_spike = tmp_path / "docs/spikes/spike-005b-webview-capacitor-pda.md"
    webview_spike.parent.mkdir(parents=True, exist_ok=True)
    webview_spike.write_text(
        "\n".join([
            f"- 状态：{status}",
            "",
            "## 实测结果",
            "",
            "SPIKE-005B webview-capacitor 在真 PDA 上使用 dev/staging runtime evidence 验证。",
            "证据引用：docs/retros/wave-3-pda-runtime-evidence.json",
            "覆盖 offline replay、Idempotency-Key replay、audit_event、L7 和 usability review。",
        ]),
        encoding="utf-8",
    )
