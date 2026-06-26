#!/usr/bin/env python3
"""check_pda_production_gate.py — PDA 生产 app 启动门禁检查

类别：4. 流程治理
Tier：T1（< 10s）
输入：docs/adr/0027-pda-offline-model.md + docs/retros/wave-3-pda-runtime-evidence.json + apps/pda-mobile/ + package.json scripts/dependencies
输出：人类可读 + --json
退出码：
  0  未提前启动生产 PDA app；若 ADR-0027 已 Accepted，则 runtime evidence validator 通过
  1  ADR-0027 未 Accepted 时 apps/pda-mobile 出现生产文件、生产 RN / Expo / EAS / Capacitor 依赖或生产 PDA scripts；或 ADR-0027 Accepted 但缺少真 PDA runtime evidence
  2  脚本自身错误

背景：
  ADR-0027 明确：生产 PDA app 只有在 ADR-0027 Accepted 后启动；
  ADR-0027 Accepted 的前置条件是真 PDA + dev/staging evidence。
  runtime evidence 指向的 PDA 技术栈候选必须有对应 accepted Spike 实测结果。
  如果 SPIKE-005 与 SPIKE-005B 都 accepted，ADR-0027 还必须记录同口径对比结论。
  Accepted 前只允许 readiness、runbook、validator 和 spike 级 PoC。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path

from validate_wave3_pda_runtime_evidence import validate_one


_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
ADR_0027 = "docs/adr/0027-pda-offline-model.md"
WAVE3_PDA_EVIDENCE = "docs/retros/wave-3-pda-runtime-evidence.json"
SPIKE_005_RN = "docs/spikes/spike-005-rn-scanner.md"
SPIKE_005B_WEBVIEW = "docs/spikes/spike-005b-webview-capacitor-pda.md"
PDA_APP_DIR = "apps/pda-mobile"
PNPM_WORKSPACE = "pnpm-workspace.yaml"
PNPM_LOCKFILE = "pnpm-lock.yaml"
STATUS_RE = re.compile(r"^- 状态[:：]\s*(.+?)\s*$", re.MULTILINE)
LOCKFILE_PACKAGE_SECTIONS = ("packages", "snapshots")
PRODUCTION_DEPENDENCY_SECTIONS = (
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
)
BLOCKED_PDA_DEPENDENCY_PREFIXES = (
    "@capacitor/",
    "@capacitor-community/",
    "@expo/",
    "@react-native/",
    "@react-native-community/",
    "expo-",
    "react-native",
)
BLOCKED_PDA_DEPENDENCY_EXACT_NAMES = (
    "eas-cli",
    "expo",
)
CAPACITOR_CLI_CMD_RE = r"(?:(?:capacitor|cap)(?:@[^\s]+)?|@capacitor/cli(?:@[^\s]+)?)"
REACT_NATIVE_CLI_CMD_RE = (
    r"(?:react-native(?:@[^\s]+)?|@react-native/cli(?:@[^\s]+)?|"
    r"@react-native-community/cli(?:@[^\s]+)?)"
)
EXPO_CLI_CMD_RE = r"(?:expo(?:@[^\s]+)?|@expo/cli(?:@[^\s]+)?)"
EAS_CLI_CMD_RE = r"(?:eas(?:@[^\s]+)?|eas-cli(?:@[^\s]+)?)"
BLOCKED_PDA_SCRIPT_PATTERNS = (
    re.compile(r"(?:^|[^\w/.-])apps/pda-mobile(?:$|[^\w/.-])"),
    re.compile(r"(?:^|[^\w/.-])@wms/pda-mobile(?:$|[^\w/.-])"),
    re.compile(
        rf"(?:^|[^\w/.-]){CAPACITOR_CLI_CMD_RE}"
        rf"\s+(?:add|build|copy|open|run|sync)\s+android\b",
    ),
    re.compile(
        rf"(?:^|[^\w/.-]){REACT_NATIVE_CLI_CMD_RE}"
        rf"\s+(?:build-android|run-android|start)\b",
    ),
    re.compile(rf"(?:^|[^\w/.-]){EXPO_CLI_CMD_RE}\s+prebuild\b"),
    re.compile(rf"(?:^|[^\w/.-]){EXPO_CLI_CMD_RE}\s+run:android\b"),
    re.compile(rf"(?:^|[^\w/.-]){EAS_CLI_CMD_RE}\s+build\b"),
)
PDA_STACK_SCRIPT_PATTERNS = (
    (
        "webview-capacitor",
        re.compile(
            rf"(?:^|[^\w/.-]){CAPACITOR_CLI_CMD_RE}"
            rf"\s+(?:add|build|copy|open|run|sync)\s+android\b",
        ),
    ),
    (
        "react-native",
        re.compile(
            rf"(?:^|[^\w/.-]){REACT_NATIVE_CLI_CMD_RE}"
            rf"\s+(?:build-android|run-android|start)\b",
        ),
    ),
    ("react-native", re.compile(rf"(?:^|[^\w/.-]){EXPO_CLI_CMD_RE}\s+prebuild\b")),
    ("react-native", re.compile(rf"(?:^|[^\w/.-]){EXPO_CLI_CMD_RE}\s+run:android\b")),
    ("react-native", re.compile(rf"(?:^|[^\w/.-]){EAS_CLI_CMD_RE}\s+build\b")),
)
IGNORED_PACKAGE_ROOTS = ("spikes/", "node_modules/")
ACCEPTED_SPIKE_RESULT_COMMON_TOKENS = (
    "## 实测结果",
    "真 PDA",
    "dev/staging",
    "runtime evidence",
    "docs/retros/wave-3-pda-runtime-evidence.json",
    "offline replay",
    "Idempotency-Key",
    "audit_event",
    "L7",
    "usability review",
)
PLACEHOLDER_TEXT_TOKENS = ("待填", "待确认", "todo", "tbd", "yyyy")
TEMPLATE_PLACEHOLDER_RE = re.compile(
    r"<\s*(?:\.{3}|[^>\n]*(?:待填|待确认|真实|环境|设备|编号|run-id|todo|tbd|yyyy)[^>\n]*)\s*>",
    re.IGNORECASE,
)
PDA_SPIKE_RESULT_REQUIREMENTS = (
    ("SPIKE-005", SPIKE_005_RN, "spike005", ("SPIKE-005", "react-native")),
    ("SPIKE-005B", SPIKE_005B_WEBVIEW, "spike005b", ("SPIKE-005B", "webview-capacitor")),
)
PDA_STACK_TO_SPIKE = {
    "react-native": ("SPIKE-005", "spike005"),
    "webview-capacitor": ("SPIKE-005B", "spike005b"),
}


@dataclass
class Result:
    adr_status: str | None
    evidence_message: str | None
    pda_files: list[str]
    blocked_dependency_files: list[str]
    blocked_dependencies: list[str]
    blocked_script_files: list[str]
    blocked_script_entries: list[str]
    incompatible_dependency_entries: list[str]
    incompatible_script_entries: list[str]
    incompatible_lockfile_dependency_entries: list[str]
    incompatible_lockfile_package_entries: list[str]
    spike_statuses: dict[str, str | None]
    workspace_pda_entries: list[str]
    workspace_errors: list[str]
    lockfile_dependency_entries: list[str]
    lockfile_package_entries: list[str]
    lockfile_pda_importers: list[str]
    lockfile_spike_importers: list[str]
    errors: list[str]

    @property
    def ok(self) -> bool:
        return not self.errors


def read_text(rel_path: str) -> str:
    path = REPO_ROOT / rel_path
    return path.read_text(encoding="utf-8") if path.exists() else ""


def adr_status() -> str | None:
    match = STATUS_RE.search(read_text(ADR_0027))
    return match.group(1).strip() if match else None


def adr_accepted(status: str | None) -> bool:
    return bool(status and status.startswith("Accepted"))


def spike_status(rel_path: str) -> str | None:
    match = STATUS_RE.search(read_text(rel_path))
    return match.group(1).strip().lower() if match else None


def pda_spike_statuses() -> dict[str, str | None]:
    return {
        "spike005": spike_status(SPIKE_005_RN),
        "spike005b": spike_status(SPIKE_005B_WEBVIEW),
    }


def both_pda_spikes_accepted(statuses: dict[str, str | None]) -> bool:
    return statuses.get("spike005") == "accepted" and statuses.get("spike005b") == "accepted"


def pda_comparison_recorded() -> bool:
    text = read_text(ADR_0027).lower()
    required = (
        "## 同口径对比结论",
        "spike-005",
        "spike-005b",
        "react-native",
        "webview-capacitor",
    )
    return all(token in text for token in required)


def section_after_heading(text: str, heading: str) -> str:
    start = text.find(heading)
    if start < 0:
        return ""
    next_heading = text.find("\n## ", start + len(heading))
    return text[start:] if next_heading < 0 else text[start:next_heading]


def placeholder_terms_in_text(text: str) -> list[str]:
    lowered = text.lower()
    terms = [token for token in PLACEHOLDER_TEXT_TOKENS if token in lowered]
    if TEMPLATE_PLACEHOLDER_RE.search(text):
        terms.append("<...>")
    return sorted(set(terms))


def accepted_pda_spike_result_errors(statuses: dict[str, str | None]) -> list[str]:
    errors: list[str] = []
    for label, path, status_key, candidate_tokens in PDA_SPIKE_RESULT_REQUIREMENTS:
        if statuses.get(status_key) != "accepted":
            continue
        text = section_after_heading(read_text(path), "## 实测结果")
        missing = [
            token
            for token in (*ACCEPTED_SPIKE_RESULT_COMMON_TOKENS, *candidate_tokens)
            if token not in text
        ]
        if missing:
            errors.append(
                f"{label} 状态为 accepted 时必须追加 ## 实测结果，记录真 PDA + dev/staging "
                "runtime evidence、离线 replay、幂等 replay、audit_event、L7 和 usability review；"
                f"缺少: {', '.join(missing)}"
            )
            continue
        placeholders = placeholder_terms_in_text(text)
        if placeholders:
            errors.append(
                f"{label} 的 ## 实测结果 仍包含占位内容，不能作为 accepted 证据: "
                f"{', '.join(placeholders)}"
            )
    return errors


def production_pda_files() -> list[str]:
    root = REPO_ROOT / PDA_APP_DIR
    if not root.exists():
        return []
    files = []
    for path in root.rglob("*"):
        if not path.is_file() or path.name == ".gitkeep":
            continue
        files.append(path.relative_to(REPO_ROOT).as_posix())
    return sorted(files)


def _is_ignored_package_manifest(rel_path: str) -> bool:
    return any(rel_path.startswith(prefix) for prefix in IGNORED_PACKAGE_ROOTS)


def _is_blocked_pda_dependency(name: str) -> bool:
    return name in BLOCKED_PDA_DEPENDENCY_EXACT_NAMES or any(
        name == prefix or name.startswith(prefix)
        for prefix in BLOCKED_PDA_DEPENDENCY_PREFIXES
    )


def _pda_stack_for_dependency(name: str) -> str | None:
    if name.startswith(("@capacitor/", "@capacitor-community/")):
        return "webview-capacitor"
    if name in {"eas-cli", "expo"} or name.startswith((
        "@expo/",
        "@react-native/",
        "@react-native-community/",
        "expo-",
        "react-native",
    )):
        return "react-native"
    return None


def production_pda_dependencies() -> tuple[list[str], list[str]]:
    files: set[str] = set()
    entries: list[str] = []
    for path in sorted(REPO_ROOT.rglob("package.json")):
        rel = path.relative_to(REPO_ROOT).as_posix()
        if _is_ignored_package_manifest(rel):
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(payload, dict):
            continue
        for section in PRODUCTION_DEPENDENCY_SECTIONS:
            dependencies = payload.get(section, {})
            if not isinstance(dependencies, dict):
                continue
            for dependency in sorted(str(name) for name in dependencies):
                if _is_blocked_pda_dependency(dependency):
                    files.add(rel)
                    entries.append(f"{rel}:{section}:{dependency}")
    return sorted(files), entries


def _is_blocked_pda_script(command: str) -> bool:
    normalized = " ".join(command.split())
    return any(pattern.search(f" {normalized} ") for pattern in BLOCKED_PDA_SCRIPT_PATTERNS)


def _pda_stack_for_script(command: str) -> str | None:
    normalized = " ".join(command.split())
    for candidate, pattern in PDA_STACK_SCRIPT_PATTERNS:
        if pattern.search(f" {normalized} "):
            return candidate
    return None


def production_pda_scripts() -> tuple[list[str], list[str]]:
    files: set[str] = set()
    entries: list[str] = []
    for path in sorted(REPO_ROOT.rglob("package.json")):
        rel = path.relative_to(REPO_ROOT).as_posix()
        if _is_ignored_package_manifest(rel):
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(payload, dict):
            continue
        scripts = payload.get("scripts", {})
        if not isinstance(scripts, dict):
            continue
        for name, command in sorted(scripts.items()):
            if not isinstance(command, str) or not _is_blocked_pda_script(command):
                continue
            files.add(rel)
            entries.append(f"{rel}:scripts:{name}:{command}")
    return sorted(files), entries


def blocked_workspace_entries() -> tuple[list[str], list[str]]:
    workspace = REPO_ROOT / PNPM_WORKSPACE
    if not workspace.exists():
        return [], []
    pda_entries: list[str] = []
    spike_entries: list[str] = []
    in_packages = False
    for raw_line in workspace.read_text(encoding="utf-8").splitlines():
        line = _strip_yaml_inline_comment(raw_line.strip())
        if not line or line.startswith("#"):
            continue
        if line.startswith("packages:"):
            in_packages = True
            continue
        if in_packages and not raw_line.startswith((" ", "\t", "-")):
            in_packages = False
        if not in_packages or not line.startswith("-"):
            continue
        pattern = line[1:].strip().strip("\"'")
        normalized_pattern = _normalize_workspace_path(pattern)
        if normalized_pattern == PDA_APP_DIR:
            pda_entries.append(f"{PNPM_WORKSPACE}:{pattern}")
        if _is_spike_workspace_path(normalized_pattern):
            spike_entries.append(f"{PNPM_WORKSPACE}:{pattern}")
    return pda_entries, spike_entries


def _strip_yaml_inline_comment(value: str) -> str:
    quote: str | None = None
    for index, char in enumerate(value):
        if quote:
            if char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
            continue
        if char == "#" and (index == 0 or value[index - 1].isspace()):
            return value[:index].strip()
    return value.strip()


def _clean_yaml_key(line: str) -> str:
    return _strip_yaml_inline_comment(line).strip().rstrip(":").strip("\"'")


def _normalize_workspace_path(value: str) -> str:
    normalized = value.strip().strip("\"'")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized.rstrip("/")


def _is_spike_workspace_path(value: str) -> bool:
    return value == "spikes" or value.startswith("spikes/")


def _lockfile_package_name(key: str) -> str:
    normalized = key.strip().lstrip("/")
    if normalized.startswith("@"):
        return normalized.rsplit("@", 1)[0]
    return normalized.split("@", 1)[0]


def blocked_lockfile_entries() -> tuple[list[str], list[str], list[str], list[str]]:
    lockfile = REPO_ROOT / PNPM_LOCKFILE
    if not lockfile.exists():
        return [], [], [], []

    dependency_entries: list[str] = []
    package_entries: list[str] = []
    pda_importers: list[str] = []
    spike_importers: list[str] = []
    in_importers = False
    lockfile_package_section: str | None = None
    current_importer: str | None = None
    current_section: str | None = None

    for raw_line in lockfile.read_text(encoding="utf-8").splitlines():
        line = _strip_yaml_inline_comment(raw_line.strip())
        if not line or line.startswith("#"):
            continue
        if line == "importers:":
            in_importers = True
            lockfile_package_section = None
            current_importer = None
            current_section = None
            continue
        section_key = line[:-1] if line.endswith(":") else ""
        if raw_line and not raw_line.startswith((" ", "\t")) and section_key in LOCKFILE_PACKAGE_SECTIONS:
            in_importers = False
            lockfile_package_section = section_key
            current_importer = None
            current_section = None
            continue
        if lockfile_package_section and raw_line and not raw_line.startswith((" ", "\t")):
            lockfile_package_section = None
        if in_importers and raw_line and not raw_line.startswith((" ", "\t")):
            in_importers = False
            current_importer = None
            current_section = None
        if lockfile_package_section:
            if raw_line.startswith("  ") and not raw_line.startswith("    ") and line.endswith(":"):
                package_key = _clean_yaml_key(line)
                package_name = _lockfile_package_name(package_key)
                if _is_blocked_pda_dependency(package_name):
                    package_entries.append(f"{PNPM_LOCKFILE}:{lockfile_package_section}:{package_key}")
            continue
        if not in_importers:
            continue

        if raw_line.startswith("  ") and not raw_line.startswith("    ") and line.endswith(":"):
            current_importer = _clean_yaml_key(line)
            current_section = None
            normalized_importer = _normalize_workspace_path(current_importer)
            if normalized_importer == PDA_APP_DIR:
                pda_importers.append(f"{PNPM_LOCKFILE}:{current_importer}")
            if _is_spike_workspace_path(normalized_importer):
                spike_importers.append(f"{PNPM_LOCKFILE}:{current_importer}")
            continue
        if current_importer is None:
            continue
        if raw_line.startswith("    ") and not raw_line.startswith("      ") and line.endswith(":"):
            section = _clean_yaml_key(line)
            current_section = section if section in PRODUCTION_DEPENDENCY_SECTIONS else None
            continue
        if current_section and raw_line.startswith("      ") and not raw_line.startswith("        ") and line.endswith(":"):
            dependency = _clean_yaml_key(line)
            if _is_blocked_pda_dependency(dependency):
                dependency_entries.append(f"{PNPM_LOCKFILE}:{current_importer}:{current_section}:{dependency}")

    return dependency_entries, package_entries, pda_importers, spike_importers


def validate_wave3_evidence() -> tuple[bool, str]:
    return validate_one(REPO_ROOT / WAVE3_PDA_EVIDENCE, allow_example_refs=False)


def pda_runtime_stack_candidate() -> str | None:
    path = REPO_ROOT / WAVE3_PDA_EVIDENCE
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    if not isinstance(payload, dict):
        return None
    candidate = payload.get("pda_stack_candidate")
    return str(candidate).strip().lower() if candidate else None


def _dependency_name_from_entry(entry: str) -> str:
    return entry.rsplit(":", 1)[-1]


def _lockfile_package_name_from_entry(entry: str) -> str:
    return _lockfile_package_name(entry.rsplit(":", 1)[-1])


def _script_command_from_entry(entry: str) -> str:
    return entry.split(":", 3)[-1]


def incompatible_dependency_entries(entries: list[str], runtime_candidate: str | None) -> list[str]:
    if runtime_candidate not in PDA_STACK_TO_SPIKE:
        return []
    incompatible = []
    for entry in entries:
        dependency_stack = _pda_stack_for_dependency(_dependency_name_from_entry(entry))
        if dependency_stack and dependency_stack != runtime_candidate:
            incompatible.append(entry)
    return incompatible


def incompatible_lockfile_package_entries(entries: list[str], runtime_candidate: str | None) -> list[str]:
    if runtime_candidate not in PDA_STACK_TO_SPIKE:
        return []
    incompatible = []
    for entry in entries:
        package_stack = _pda_stack_for_dependency(_lockfile_package_name_from_entry(entry))
        if package_stack and package_stack != runtime_candidate:
            incompatible.append(entry)
    return incompatible


def incompatible_script_entries(entries: list[str], runtime_candidate: str | None) -> list[str]:
    if runtime_candidate not in PDA_STACK_TO_SPIKE:
        return []
    incompatible = []
    for entry in entries:
        script_stack = _pda_stack_for_script(_script_command_from_entry(entry))
        if script_stack and script_stack != runtime_candidate:
            incompatible.append(entry)
    return incompatible


def runtime_candidate_spike_status_error(statuses: dict[str, str | None]) -> str | None:
    candidate = pda_runtime_stack_candidate()
    if candidate not in PDA_STACK_TO_SPIKE:
        return None
    label, status_key = PDA_STACK_TO_SPIKE[candidate]
    if statuses.get(status_key) == "accepted":
        return None
    return (
        f"ADR-0027 Accepted 的 runtime evidence 指向 pda_stack_candidate={candidate}，"
        f"但对应 {label} 状态不是 accepted；必须先完成该 Spike 的真机实测结果"
    )


def collect_result() -> Result:
    status = adr_status()
    files = production_pda_files()
    dependency_files, dependencies = production_pda_dependencies()
    script_files, script_entries = production_pda_scripts()
    spike_statuses = pda_spike_statuses()
    workspace_pda_entries, workspace_errors = blocked_workspace_entries()
    (
        lockfile_dependencies,
        lockfile_package_entries,
        lockfile_pda_importers,
        lockfile_spike_importers,
    ) = blocked_lockfile_entries()
    evidence_message: str | None = None
    runtime_candidate: str | None = None
    incompatible_dependencies: list[str] = []
    incompatible_scripts: list[str] = []
    incompatible_lockfile_dependencies: list[str] = []
    incompatible_lockfile_packages: list[str] = []
    errors: list[str] = []

    if status is None:
        errors.append(f"{ADR_0027} 缺少状态字段，无法判断 PDA 生产 app 启动门禁")
    errors.extend(accepted_pda_spike_result_errors(spike_statuses))
    if adr_accepted(status):
        evidence_ok, evidence_message = validate_wave3_evidence()
        if not evidence_ok:
            errors.append(
                "ADR-0027 Accepted 必须先通过 Wave 3 PDA runtime evidence validator："
                f"{evidence_message}"
            )
        else:
            runtime_candidate = pda_runtime_stack_candidate()
            candidate_error = runtime_candidate_spike_status_error(spike_statuses)
            if candidate_error:
                errors.append(candidate_error)
            incompatible_dependencies = incompatible_dependency_entries(dependencies, runtime_candidate)
            incompatible_scripts = incompatible_script_entries(script_entries, runtime_candidate)
            incompatible_lockfile_dependencies = incompatible_dependency_entries(
                lockfile_dependencies,
                runtime_candidate,
            )
            incompatible_lockfile_packages = incompatible_lockfile_package_entries(
                lockfile_package_entries,
                runtime_candidate,
            )
            if (
                incompatible_dependencies
                or incompatible_scripts
                or incompatible_lockfile_dependencies
                or incompatible_lockfile_packages
            ):
                errors.append(
                    f"ADR-0027 Accepted 的 runtime evidence 指向 pda_stack_candidate={runtime_candidate}，"
                    "生产 PDA 技术栈依赖 / native 打包脚本必须与该候选一致；不匹配项："
                    f"{', '.join(incompatible_dependencies + incompatible_scripts + incompatible_lockfile_dependencies + incompatible_lockfile_packages)}"
                )
        if both_pda_spikes_accepted(spike_statuses) and not pda_comparison_recorded():
            errors.append(
                "SPIKE-005 与 SPIKE-005B 均为 accepted 时，ADR-0027 Accepted 必须追加 "
                "## 同口径对比结论，记录 react-native 与 webview-capacitor 的同设备、"
                "同条码样本、同 M2/M3 dev/staging 测试数据对比"
            )
    if files and not adr_accepted(status):
        errors.append(
            f"{PDA_APP_DIR} 出现生产文件，但 ADR-0027 尚未 Accepted；"
            "生产 PDA app 必须等 SPIKE-005 / 005B 真机验证和 ADR-0027 Accepted 后启动"
        )
    if dependencies and not adr_accepted(status):
        errors.append(
            "生产 workspace 出现 PDA 技术栈依赖，但 ADR-0027 尚未 Accepted；"
            "RN / Expo / EAS / Capacitor 生产依赖只能在真机验证和 ADR-0027 Accepted 后引入："
            f"{', '.join(dependencies)}"
        )
    if script_entries and not adr_accepted(status):
        errors.append(
            "生产 workspace 出现 PDA app 或 Android native 打包脚本，但 ADR-0027 尚未 Accepted；"
            "生产 PDA 启动脚本只能在真机验证和 ADR-0027 Accepted 后引入："
            f"{', '.join(script_entries)}"
        )
    if lockfile_dependencies and not adr_accepted(status):
        errors.append(
            "pnpm lockfile importers 出现 PDA 技术栈依赖，但 ADR-0027 尚未 Accepted；"
            "请移除生产 workspace 的 RN / Expo / EAS / Capacitor lockfile 入口："
            f"{', '.join(lockfile_dependencies)}"
        )
    if lockfile_package_entries and not adr_accepted(status):
        errors.append(
            "pnpm lockfile packages / snapshots 区残留 PDA 技术栈包，但 ADR-0027 尚未 Accepted；"
            "请重新生成 lockfile，移除 RN / Expo / EAS / Capacitor 包条目："
            f"{', '.join(lockfile_package_entries)}"
        )
    if lockfile_pda_importers and not adr_accepted(status):
        errors.append(
            "pnpm lockfile 出现 apps/pda-mobile importer，但 ADR-0027 尚未 Accepted；"
            "生产 PDA app 必须等真机验证和 ADR-0027 Accepted 后进入 workspace："
            f"{', '.join(lockfile_pda_importers)}"
        )
    if workspace_pda_entries and not adr_accepted(status):
        errors.append(
            "pnpm workspace 显式加入 apps/pda-mobile，但 ADR-0027 尚未 Accepted；"
            "生产 PDA app 必须等真机验证和 ADR-0027 Accepted 后进入 workspace："
            f"{', '.join(workspace_pda_entries)}"
        )
    if workspace_errors:
        errors.append(
            "pnpm workspace 不能纳入 spikes/ PoC；SPIKE-005B 依赖必须留在 spike 边界内，"
            f"不能进入生产 workspace：{', '.join(workspace_errors)}"
        )
    if lockfile_spike_importers:
        errors.append(
            "pnpm lockfile 不能包含 spikes/ importer；SPIKE-005B PoC 不得进入生产 workspace lockfile："
            f"{', '.join(lockfile_spike_importers)}"
        )

    return Result(
        status,
        evidence_message,
        files,
        dependency_files,
        dependencies,
        script_files,
        script_entries,
        incompatible_dependencies,
        incompatible_scripts,
        incompatible_lockfile_dependencies,
        incompatible_lockfile_packages,
        spike_statuses,
        workspace_pda_entries,
        workspace_errors,
        lockfile_dependencies,
        lockfile_package_entries,
        lockfile_pda_importers,
        lockfile_spike_importers,
        errors,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    result = collect_result()

    if args.json:
        print(json.dumps({
            "check": "check_pda_production_gate",
            "tier": "T1",
            "category": "流程治理",
            "adr_status": result.adr_status,
            "evidence_message": result.evidence_message,
            "pda_files": result.pda_files,
            "blocked_dependency_files": result.blocked_dependency_files,
            "blocked_dependencies": result.blocked_dependencies,
            "blocked_script_files": result.blocked_script_files,
            "blocked_script_entries": result.blocked_script_entries,
            "incompatible_dependency_entries": result.incompatible_dependency_entries,
            "incompatible_script_entries": result.incompatible_script_entries,
            "incompatible_lockfile_dependency_entries": result.incompatible_lockfile_dependency_entries,
            "incompatible_lockfile_package_entries": result.incompatible_lockfile_package_entries,
            "spike_statuses": result.spike_statuses,
            "workspace_pda_entries": result.workspace_pda_entries,
            "workspace_errors": result.workspace_errors,
            "lockfile_dependency_entries": result.lockfile_dependency_entries,
            "lockfile_package_entries": result.lockfile_package_entries,
            "lockfile_pda_importers": result.lockfile_pda_importers,
            "lockfile_spike_importers": result.lockfile_spike_importers,
            "errors": result.errors,
            "ok": result.ok,
        }, ensure_ascii=False, indent=2))
    else:
        print("check_pda_production_gate (T1, 流程治理)")
        print(f"  · ADR-0027 status: {result.adr_status or '?'}")
        if result.evidence_message:
            print(f"  · Wave 3 PDA evidence: {result.evidence_message}")
        if result.pda_files:
            print(f"  · PDA production files: {len(result.pda_files)}")
            for rel in result.pda_files:
                print(f"    - {rel}")
        else:
            print("  · PDA production files: 0")
        if result.blocked_dependencies:
            print(f"  · PDA production dependencies: {len(result.blocked_dependencies)}")
            for entry in result.blocked_dependencies:
                print(f"    - {entry}")
        if result.blocked_script_entries:
            print(f"  · PDA production scripts: {len(result.blocked_script_entries)}")
            for entry in result.blocked_script_entries:
                print(f"    - {entry}")
        if result.incompatible_dependency_entries:
            print(f"  · Incompatible PDA dependencies: {len(result.incompatible_dependency_entries)}")
            for entry in result.incompatible_dependency_entries:
                print(f"    - {entry}")
        if result.incompatible_script_entries:
            print(f"  · Incompatible PDA scripts: {len(result.incompatible_script_entries)}")
            for entry in result.incompatible_script_entries:
                print(f"    - {entry}")
        if result.incompatible_lockfile_dependency_entries:
            print(
                "  · Incompatible lockfile PDA dependencies: "
                f"{len(result.incompatible_lockfile_dependency_entries)}"
            )
            for entry in result.incompatible_lockfile_dependency_entries:
                print(f"    - {entry}")
        if result.incompatible_lockfile_package_entries:
            print(
                "  · Incompatible lockfile PDA packages: "
                f"{len(result.incompatible_lockfile_package_entries)}"
            )
            for entry in result.incompatible_lockfile_package_entries:
                print(f"    - {entry}")
        print(
            "  · PDA spike statuses: "
            f"SPIKE-005={result.spike_statuses.get('spike005') or '?'}, "
            f"SPIKE-005B={result.spike_statuses.get('spike005b') or '?'}"
        )
        if result.workspace_errors:
            print(f"  · Workspace spike entries: {len(result.workspace_errors)}")
            for entry in result.workspace_errors:
                print(f"    - {entry}")
        if result.workspace_pda_entries:
            print(f"  · Workspace PDA entries: {len(result.workspace_pda_entries)}")
            for entry in result.workspace_pda_entries:
                print(f"    - {entry}")
        if result.lockfile_dependency_entries:
            print(f"  · Lockfile PDA dependencies: {len(result.lockfile_dependency_entries)}")
            for entry in result.lockfile_dependency_entries:
                print(f"    - {entry}")
        if result.lockfile_package_entries:
            print(f"  · Lockfile PDA packages: {len(result.lockfile_package_entries)}")
            for entry in result.lockfile_package_entries:
                print(f"    - {entry}")
        if result.lockfile_pda_importers:
            print(f"  · Lockfile PDA importers: {len(result.lockfile_pda_importers)}")
            for entry in result.lockfile_pda_importers:
                print(f"    - {entry}")
        if result.lockfile_spike_importers:
            print(f"  · Lockfile spike importers: {len(result.lockfile_spike_importers)}")
            for entry in result.lockfile_spike_importers:
                print(f"    - {entry}")
        if result.ok:
            print("  ✓ PDA 生产 app 启动门禁通过")
        else:
            for error in result.errors:
                print(f"  ✘ {error}")

    return 0 if result.ok else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"script error: {exc}", file=sys.stderr)
        sys.exit(2)
