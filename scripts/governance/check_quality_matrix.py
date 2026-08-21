#!/usr/bin/env python3
"""check_quality_matrix.py — 全链路质量矩阵检查

类别：4. 流程治理
Tier：T1（< 10s，纯静态扫描）
输入：governance/quality-matrix.toml、docs/governance/quality-matrix.md、docs/domain/user-stories-*.md、shared/openapi/openapi.json
输出：人类可读 + --json；--write-doc 可刷新 MkDocs 展示页
退出码：
  0  质量矩阵结构、状态和展示页一致
  1  矩阵缺失、状态不合规或展示页漂移
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

try:
    import tomllib as toml
except ModuleNotFoundError:
    import tomli as toml

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MATRIX = REPO_ROOT / "governance" / "quality-matrix.toml"
ADVERSARIAL_CATALOG = REPO_ROOT / "governance" / "adversarial-catalog.toml"
DOC = REPO_ROOT / "docs" / "governance" / "quality-matrix.md"
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
OPENAPI_YAML_FILES = [
    REPO_ROOT / "shared" / "openapi" / "customer-portal-openapi.yaml",
]
STORY_GLOB = "docs/domain/user-stories-*.md"
NAVIGATION_CHECK_SOURCES = {
    "pnpm --dir apps/web-admin run test:e2e:shell-dev": REPO_ROOT
    / "prototypes/e2e/web-admin-shell.spec.ts",
    "node apps/web-admin/self-checks/h1-api-key-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/h1-api-key-slice-self-check.mjs",
    "node apps/web-admin/self-checks/m3-batch-management-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m3-batch-management-slice-self-check.mjs",
    "node apps/web-admin/self-checks/m3-location-history-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m3-location-history-slice-self-check.mjs",
    "node apps/web-admin/self-checks/m3-inventory-status-config-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m3-inventory-status-config-self-check.mjs",
    "node apps/web-admin/self-checks/m3-ops-pages-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m3-ops-pages-self-check.mjs",
    "node apps/web-admin/self-checks/m1-product-source-actions-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-product-source-actions-self-check.mjs",
    "node apps/web-admin/self-checks/m1-zone-real-api-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-zone-real-api-self-check.mjs",
    "node apps/web-admin/self-checks/m1-location-contract-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-location-contract-self-check.mjs",
    "node apps/web-admin/self-checks/m1-dock-management-navigation-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-dock-management-navigation-self-check.mjs",
    "node apps/web-admin/self-checks/m1-lpn-container-navigation-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-lpn-container-navigation-self-check.mjs",
    "node apps/web-admin/self-checks/m1-device-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-device-self-check.mjs",
    "node apps/web-admin/self-checks/m1-device-dashboard-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m1-device-dashboard-self-check.mjs",
    "node apps/web-admin/self-checks/te-task-type-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/te-task-type-slice-self-check.mjs",
    "node apps/web-admin/self-checks/mte-task-execution-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/mte-task-execution-slice-self-check.mjs",
    "node apps/web-admin/self-checks/hal-alert-definition-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/hal-alert-definition-slice-self-check.mjs",
    "node apps/web-admin/self-checks/hal-alert-runtime-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/hal-alert-runtime-slice-self-check.mjs",
    "node apps/web-admin/self-checks/h8-erp-interface-table-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/h8-erp-interface-table-slice-self-check.mjs",
    "node apps/web-admin/self-checks/h9-delivery-note-aggregation-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/h9-delivery-note-aggregation-self-check.mjs",
    "node apps/web-admin/self-checks/m2-inbound-page-helpers-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m2-inbound-page-helpers-self-check.mjs",
    "node apps/web-admin/self-checks/m2-putaway-strategy-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m2-putaway-strategy-self-check.mjs",
    "node apps/web-admin/self-checks/m2-inbound-documents-page-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m2-inbound-documents-page-self-check.mjs",
    "node apps/web-admin/self-checks/di-drug-inspection-slice-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/di-drug-inspection-slice-self-check.mjs",
    "node apps/web-admin/self-checks/mdi-document-workflow-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/mdi-document-workflow-self-check.mjs",
    "node apps/web-admin/self-checks/m4-outbound-datagrid-actions-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m4-outbound-datagrid-actions-self-check.mjs",
}

DIMENSIONS = (
    "requirement",
    "fields",
    "frontend",
    "api",
    "backend",
    "database",
    "security",
    "audit",
    "tests",
    "evidence",
    "docs",
    "governance",
)
STRICT_STATUSES = {"verified", "not_applicable"}
ALLOWED_MODULES = {
    "M1",
    "M2",
    "M3",
    "M4",
    "DOCK",
    "H1",
    "H2",
    "H3",
    "H4",
    "H5",
    "H6",
    "H7",
    "H8",
    "H9",
    "H10",
    "TE",
    "AL",
    "DI",
    "RC",
}
STORY_TYPE_LAYERS = {
    "read_only": {"L1", "L2", "L3", "L8"},
    "write": {"L1", "L2", "L3", "L4", "L5", "L8", "L11"},
    "inventory_change": {"L1", "L2", "L3", "L4", "L5", "L8", "L10", "L11"},
    "concurrent_resource": {"L1", "L2", "L3", "L4", "L5", "L6", "L8", "L11"},
    "critical_path": {f"L{index}" for index in range(1, 12)},
    "api_change": {"L2", "L9"},
    "frontend_interaction": {"L1", "L3", "L7"},
    "config_rule": {"L1", "L2", "L3", "L4", "L8", "L9"},
    "audit_compliance": {"L5", "L8", "L10", "L11"},
    "integration": {"L2", "L3", "L4", "L9", "L10"},
    "permission": {"L8"},
    "offline_sync": {f"L{index}" for index in range(1, 12)},
    "monitoring": {"L1", "L2", "L3", "L7", "L10"},
    "runtime_guard": {"L1", "L2", "L4", "L5", "L7", "L8", "L9", "L10", "L11"},
    "pda_runtime": {f"L{index}" for index in range(1, 12)},
    "hardware_runtime": {f"L{index}" for index in range(1, 12)},
    "external_runtime": {f"L{index}" for index in range(1, 12)},
    "release_runtime": {f"L{index}" for index in range(1, 12)},
}
S4_TYPES = {"pda_runtime", "hardware_runtime", "external_runtime", "release_runtime", "offline_sync"}
S3_TYPES = {"inventory_change", "concurrent_resource", "critical_path", "audit_compliance"}


@dataclass(frozen=True)
class Issue:
    story_id: str
    dimension: str
    message: str


from _quality_matrix_adversarial import check_adversarial_checks  # noqa: E402
from _quality_matrix_adversarial import (  # noqa: E402,F401
    derive_required_attack_classes,
    load_adversarial_catalog,
)


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def load_matrix() -> dict[str, Any]:
    return toml.loads(MATRIX.read_text(encoding="utf-8"))


def story_files() -> set[str]:
    return {rel(path) for path in REPO_ROOT.glob(STORY_GLOB)}


def openapi_paths() -> set[str]:
    payload = json.loads(OPENAPI_JSON.read_text(encoding="utf-8"))
    paths = payload.get("paths")
    operations = {
        f"{method.upper()} {path}"
        for path, operations in paths.items()
        if isinstance(operations, dict)
        for method in operations
    } if isinstance(paths, dict) else set()
    for contract in OPENAPI_YAML_FILES:
        if contract.exists():
            operations.update(yaml_openapi_paths(contract))
    return operations


def yaml_openapi_paths(contract: Path) -> set[str]:
    """读取仓库内简单 OpenAPI YAML 的 paths/method，避免治理脚本新增运行时依赖。"""
    operations: set[str] = set()
    in_paths = False
    current_path: str | None = None
    for line in contract.read_text(encoding="utf-8").splitlines():
        if line == "paths:":
            in_paths = True
            continue
        if not in_paths:
            continue
        if line and not line.startswith(" "):
            break
        path_match = re.fullmatch(r"  (/\S+):\s*", line)
        if path_match:
            current_path = path_match.group(1)
            continue
        method_match = re.fullmatch(
            r"    (get|put|post|delete|patch|options|head|trace):\s*",
            line,
            flags=re.IGNORECASE,
        )
        if current_path and method_match:
            operations.add(f"{method_match.group(1).upper()} {current_path}")
    return operations


def layer_sort_key(layer: str) -> int:
    return int(layer[1:]) if layer.startswith("L") and layer[1:].isdigit() else 999


def derive_required_layers(types: list[str]) -> list[str]:
    layers: set[str] = set()
    for story_type in types:
        layers.update(STORY_TYPE_LAYERS.get(story_type, set()))
    return sorted(layers, key=layer_sort_key)


def derive_acceptance_level(types: list[str]) -> str:
    """按最高风险故事类型推导验收深度，避免手工降级。"""
    type_set = set(types)
    if type_set & S4_TYPES:
        return "S4"
    if type_set & S3_TYPES:
        return "S3"
    if "write" in type_set or "integration" in type_set:
        return "S2"
    return "S1"


def check_module_completion(matrix: dict[str, Any], module: str) -> list[Issue]:
    """模块完成要求无延期故事，且已完成写故事覆盖对抗攻击类。"""
    issues = [
        Issue(
            str(story.get("id", "<missing>")),
            "module_completion",
            f"模块 {module} 仍有延期故事: {story.get('title', '-')}",
        )
        for story in matrix.get("deferred_stories", [])
        if isinstance(story, dict) and story.get("module") == module
    ]
    for story in matrix.get("stories", []):
        if isinstance(story, dict) and story.get("module") == module:
            issues.extend(check_adversarial_checks(story, require_coverage=True))
    return issues


def check_deferred_story(story: dict[str, Any]) -> list[Issue]:
    """已声明类型的延期故事只能使用可推导验收要求的已知类型。"""
    story_id = str(story.get("id", "<missing>"))
    types = story.get("types")
    if types is None:
        return []
    if not isinstance(types, list) or not types or not all(isinstance(item, str) for item in types):
        return [Issue(story_id, "types", "types 必须是非空字符串数组")]
    issues = [
        Issue(story_id, "types", f"未知故事类型: {story_type}")
        for story_type in sorted(set(types) - set(STORY_TYPE_LAYERS))
    ]
    issues.extend(check_e2e_checks(story, verified=False))
    return issues


def _package_scripts(repo_root: Path) -> dict[str, str]:
    package = repo_root / "apps" / "web-admin" / "package.json"
    if not package.is_file():
        return {}
    payload = json.loads(package.read_text(encoding="utf-8"))
    scripts = payload.get("scripts", {})
    return scripts if isinstance(scripts, dict) else {}


def _playwright_config_path(command: str, *, repo_root: Path) -> Path | None:
    match = re.search(r"--config(?:=|\s+)([^\s]+)", command)
    if not match:
        return None
    config = match.group(1).strip("'\"")
    if "prototypes" in command:
        return repo_root / "prototypes" / config
    return repo_root / config


def _real_playwright_spec(config_path: Path, *, repo_root: Path) -> Path | None:
    if not config_path.is_file():
        return None
    config_text = config_path.read_text(encoding="utf-8")
    match = re.search(r"(web-admin-[A-Za-z0-9-]+-real)\\?\.spec", config_text)
    if match:
        return repo_root / "prototypes" / "e2e" / f"{match.group(1)}.spec.ts"
    stem = config_path.name.removeprefix("playwright-").removesuffix("-config.ts")
    if stem.endswith("-real"):
        return repo_root / "prototypes" / "e2e" / f"{stem}.spec.ts"
    return None


def _is_real_e2e(command: str, config_path: Path | None) -> bool:
    config_name = config_path.name if config_path else ""
    return "-real" in command.lower() or "-real" in config_name.lower()


def _is_dev_e2e(command: str, script: str = "") -> bool:
    text = f"{command} {script}"
    return bool(
        re.search(r"test:e2e:[^\s]*dev\b", text)
        or re.search(r"playwright-[^\s]*-dev-config", text)
        or re.search(r"WMS_WEB_ADMIN_DEV_MOCK\s*[:=]\s*[\"']?1\b", text)
    )


def _check_screenshot_mapping(story: dict[str, Any], *, repo_root: Path, has_real_e2e: bool) -> list[Issue]:
    if not has_real_e2e or not story.get("frontend_pages"):
        return []
    story_id = str(story.get("id", "<missing>"))
    records = story.get("e2e_screenshots")
    evidence_refs = story.get("evidence_refs", [])
    if not isinstance(evidence_refs, list):
        evidence_refs = []
    if not isinstance(records, list) or not records:
        if any(isinstance(ref, str) and ref.endswith(".png") for ref in evidence_refs):
            return []
        return [Issue(story_id, "evidence", "真实 E2E 故事必须登记 e2e_screenshots 或 PNG evidence_refs")]
    issues: list[Issue] = []
    has_real_record = False
    for record in records:
        if not isinstance(record, dict):
            issues.append(Issue(story_id, "evidence", "e2e_screenshots 每项必须是对象"))
            continue
        page = record.get("page")
        spec = record.get("spec")
        screenshot = record.get("screenshot")
        if not isinstance(page, str) or page not in story.get("frontend_pages", []):
            issues.append(Issue(story_id, "evidence", f"e2e_screenshots.page 未覆盖 frontend_pages: {page}"))
        if not isinstance(spec, str) or not spec.endswith(".spec.ts"):
            issues.append(Issue(story_id, "evidence", "e2e_screenshots.spec 必须是 Playwright .spec.ts"))
        else:
            has_real_record = has_real_record or spec.endswith("-real.spec.ts")
        if isinstance(spec, str) and spec.endswith(".spec.ts") and not (repo_root / spec).is_file():
            issues.append(Issue(story_id, "evidence", f"截图 spec 不存在: {spec}"))
        if not isinstance(screenshot, str) or not screenshot.startswith("artifacts/screenshot-portal/real-web/") or not screenshot.endswith(".png"):
            issues.append(Issue(story_id, "evidence", "e2e_screenshots.screenshot 必须是 real-web PNG 路径"))
        for value in (spec, screenshot):
            if isinstance(value, str) and value not in evidence_refs:
                issues.append(Issue(story_id, "evidence", f"截图证据未进入 evidence_refs: {value}"))
    if not has_real_record:
        issues.append(Issue(story_id, "evidence", "真实 E2E 故事的 e2e_screenshots 必须至少包含一条 *-real.spec.ts"))
    return issues


def check_e2e_checks(
    story: dict[str, Any], *, repo_root: Path = REPO_ROOT, verified: bool = False
) -> list[Issue]:
    """检查 E2E 命令可解析到现有脚本/config，且 dev mock 不能单独支撑 verified。"""
    checks = story.get("e2e_checks")
    if checks is None:
        return []
    story_id = str(story.get("id", "<missing>"))
    if not isinstance(checks, list) or not all(isinstance(item, str) for item in checks):
        return [Issue(story_id, "evidence", "e2e_checks 必须是字符串数组")]
    scripts = _package_scripts(repo_root)
    issues: list[Issue] = []
    has_real_e2e = False
    has_dev_e2e = False
    for command in checks:
        script_match = re.search(r"pnpm\s+--dir\s+apps/web-admin\s+run\s+(test:e2e:[^\s]+)", command)
        script = ""
        config_path: Path | None = None
        if script_match:
            script_name = script_match.group(1)
            script = scripts.get(script_name, "")
            if not script:
                issues.append(Issue(story_id, "evidence", f"e2e_checks package script 不存在: {script_name}"))
            config_path = _playwright_config_path(script, repo_root=repo_root)
            if config_path is None or not config_path.is_file():
                issues.append(Issue(story_id, "evidence", f"package script 未解析到现有 Playwright config: {script_name}"))
        elif "playwright test" in command:
            config_path = _playwright_config_path(command, repo_root=repo_root)
            if config_path is None or not config_path.is_file():
                issues.append(Issue(story_id, "evidence", "Playwright 命令未解析到现有 config"))
        elif re.search(r"\bjust\s+[^\s]+", command):
            target = re.search(r"\bjust\s+([^\s]+)", command).group(1)
            just_text = (repo_root / "justfile").read_text(encoding="utf-8") if (repo_root / "justfile").is_file() else ""
            if not re.search(rf"(?m)^{re.escape(target)}\s*:", just_text):
                issues.append(Issue(story_id, "evidence", f"just 目标不存在: {target}"))
        elif re.search(r"\bnode\s+([^\s]+)", command):
            target = re.search(r"\bnode\s+([^\s]+)", command).group(1).strip("'\"")
            if not (repo_root / target).is_file():
                issues.append(Issue(story_id, "evidence", f"node 检查文件不存在: {target}"))

        if _is_real_e2e(command, config_path):
            has_real_e2e = True
            if config_path and config_path.is_file():
                spec_path = _real_playwright_spec(config_path, repo_root=repo_root)
                if spec_path is None or not spec_path.is_file():
                    issues.append(Issue(story_id, "evidence", f"real Playwright config 未找到 *-real.spec.ts: {config_path.name}"))
                elif re.search(r"\b(?:page|context)\.route\s*\(", spec_path.read_text(encoding="utf-8")):
                    issues.append(Issue(story_id, "evidence", f"real spec 禁止业务路由拦截: {rel(spec_path)}"))
        if _is_dev_e2e(command, script):
            has_dev_e2e = True
    if verified and has_dev_e2e and not has_real_e2e:
        issues.append(Issue(story_id, "evidence", "verified 故事不能只用 shell-dev/dev mock 作为真实 E2E 证据"))
    issues.extend(_check_screenshot_mapping(story, repo_root=repo_root, has_real_e2e=has_real_e2e))
    return issues


def check_story(story: dict[str, Any], *, story_files: set[str], openapi_paths: set[str]) -> list[Issue]:
    issues: list[Issue] = []
    story_id = str(story.get("id", "<missing>"))
    types = story.get("types", [])
    if not isinstance(types, list) or not all(isinstance(item, str) for item in types) or not types:
        issues.append(Issue(story_id, "types", "types 必须是非空字符串数组"))
        types = []
    unknown_types = sorted(set(types) - set(STORY_TYPE_LAYERS))
    for story_type in unknown_types:
        issues.append(Issue(story_id, "types", f"未知故事类型: {story_type}"))

    if not isinstance(story.get("title"), str) or not story["title"].strip():
        issues.append(Issue(story_id, "requirement", "title 必须填写"))
    module = story.get("module")
    if module not in ALLOWED_MODULES:
        issues.append(Issue(story_id, "requirement", "quality matrix 只允许已登记的业务模块和横向模块"))
    id_module = story_id.split("-", 2)[1] if story_id.startswith("US-") and len(story_id.split("-", 2)) > 1 else None
    if module in ALLOWED_MODULES and id_module != module:
        issues.append(Issue(story_id, "requirement", f"story id 模块 {id_module} 与 module {module} 不一致"))
    requirement = story.get("requirement")
    story_file = story.get("story_file")
    if story_file is None and isinstance(requirement, dict):
        story_file = requirement.get("story_file")
    if not isinstance(story_file, str) or not story_file:
        issues.append(Issue(story_id, "requirement", "story_file 必须填写"))
    elif story_files and story_file not in story_files:
        issues.append(Issue(story_id, "requirement", f"story_file 不存在: {story_file}"))

    dimensions = story.get("dimensions", {})
    reasons = story.get("not_applicable_reasons", {})
    if not isinstance(dimensions, dict):
        issues.append(Issue(story_id, "dimensions", "dimensions 必须是对象"))
        dimensions = {}
    if not isinstance(reasons, dict):
        issues.append(Issue(story_id, "dimensions", "not_applicable_reasons 必须是对象"))
        reasons = {}

    for dimension in DIMENSIONS:
        status = dimension_status(story, dimension)
        if status is None:
            issues.append(Issue(story_id, dimension, "缺少维度状态"))
            continue
        if status not in STRICT_STATUSES:
            issues.append(Issue(story_id, dimension, f"状态 {status} 不允许；只能是 verified 或 not_applicable"))
        if status == "not_applicable" and not str(reasons.get(dimension, "")).strip():
            issues.append(Issue(story_id, dimension, "not_applicable 必须填写 reason"))

    required_layers = derive_required_layers(types)
    tests = story.get("tests", {})
    declared_layers = tests.get("required_layers") if isinstance(tests, dict) else None
    covered_layers = tests.get("covered_layers") if isinstance(tests, dict) else None
    if declared_layers != required_layers:
        issues.append(Issue(story_id, "tests", f"required_layers 应为 {', '.join(required_layers)}"))
    if dimension_status(story, "tests") == "verified":
        if not isinstance(covered_layers, list) or sorted(covered_layers, key=layer_sort_key) != required_layers:
            issues.append(Issue(story_id, "tests", "tests=verified 时 covered_layers 必须覆盖全部 required_layers"))

    for item in story.get("api_paths", []):
        if not isinstance(item, str) or " " not in item:
            issues.append(Issue(story_id, "api", f"api_paths 项格式必须是 'METHOD /path': {item}"))
            continue
        method, path = item.split(" ", 1)
        operation = f"{method.upper()} {path}"
        if openapi_paths and operation not in openapi_paths:
            issues.append(Issue(story_id, "api", f"OpenAPI 缺少 operation: {operation}"))

    issues.extend(check_e2e_checks(story, verified=True))
    issues.extend(check_adversarial_checks(story, require_coverage=False))

    navigation_checks = story.get("navigation_checks")
    if navigation_checks is None:
        return issues
    if not isinstance(navigation_checks, list) or not all(
        isinstance(item, str) for item in navigation_checks
    ):
        issues.append(Issue(story_id, "evidence", "navigation_checks 必须是字符串数组"))
    else:
        frontend_pages = story.get("frontend_pages", [])
        navigation_sources: list[str] = []
        for command in navigation_checks:
            source = NAVIGATION_CHECK_SOURCES.get(command)
            if source is None:
                issues.append(Issue(story_id, "evidence", f"未登记的导航检查命令: {command}"))
                continue
            navigation_sources.append(source.read_text(encoding="utf-8") if source.exists() else "")
        for page in frontend_pages if isinstance(frontend_pages, list) else []:
            if isinstance(page, str) and not any(
                f'"{page}"' in source_text or f"'{page}'" in source_text
                for source_text in navigation_sources
            ):
                issues.append(Issue(story_id, "evidence", f"导航检查未覆盖页面: {page}"))

    return issues


def dimension_status(story: dict[str, Any], dimension: str) -> str | None:
    direct = story.get(dimension)
    if isinstance(direct, dict) and isinstance(direct.get("status"), str):
        return direct["status"]
    dimensions = story.get("dimensions")
    if isinstance(dimensions, dict) and isinstance(dimensions.get(dimension), str):
        return dimensions[dimension]
    return None


def is_runtime_screenshot_ref(value: str) -> bool:
    """gitignore 的 Playwright 截图产物，T1 不查磁盘是否存在。"""
    normalized = value.replace("\\", "/")
    if not normalized.endswith(".png"):
        return False
    return normalized.startswith("artifacts/") or "/.e2e-artifacts/" in f"/{normalized}"


def check_evidence_profiles(
    matrix: dict[str, Any], stories: list[dict[str, Any]], *, repo_root: Path = REPO_ROOT
) -> list[Issue]:
    issues: list[Issue] = []
    profiles = matrix.get("evidence_profiles", {})
    if not isinstance(profiles, dict):
        return [Issue("<matrix>", "evidence", "evidence_profiles 必须是对象")]
    for story in stories:
        story_id = str(story.get("id", "<missing>"))
        module = str(story.get("module", ""))
        profile = profiles.get(module)
        if not isinstance(profile, dict):
            issues.append(Issue(story_id, "evidence", f"模块 {module} 缺少 evidence_profiles.{module}"))
            continue
        for field in ("backend_files", "database_objects", "test_checks", "evidence_refs"):
            values = profile.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) and value.strip() for value in values):
                issues.append(Issue(story_id, "evidence", f"evidence_profiles.{module}.{field} 必须是非空字符串数组"))
        for field in ("backend_files", "evidence_refs"):
            for value in profile.get(field, []) if isinstance(profile.get(field), list) else []:
                if not isinstance(value, str):
                    continue
                if field == "evidence_refs" and is_runtime_screenshot_ref(value):
                    continue
                if not (repo_root / value).exists():
                    issues.append(Issue(story_id, "evidence", f"证据文件不存在: {value}"))
    return issues


def scan(*, complete_module: str | None = None) -> list[Issue]:
    matrix = load_matrix()
    stories = matrix.get("stories")
    if not isinstance(stories, list):
        return [Issue("<matrix>", "stories", "governance/quality-matrix.toml 缺少 stories 数组")]
    issues: list[Issue] = []
    known_story_files = story_files()
    known_openapi_paths = openapi_paths()
    seen: set[str] = set()
    for raw_story in stories:
        if not isinstance(raw_story, dict):
            issues.append(Issue("<matrix>", "stories", "stories 每项必须是对象"))
            continue
        story_id = str(raw_story.get("id", "<missing>"))
        if story_id in seen:
            issues.append(Issue(story_id, "requirement", "故事 ID 重复"))
        seen.add(story_id)
        issues.extend(check_story(raw_story, story_files=known_story_files, openapi_paths=known_openapi_paths))
    deferred = matrix.get("deferred_stories", [])
    if not isinstance(deferred, list):
        issues.append(Issue("<matrix>", "deferred_stories", "deferred_stories 必须是数组"))
    else:
        for raw_story in deferred:
            if not isinstance(raw_story, dict):
                issues.append(Issue("<matrix>", "deferred_stories", "deferred_stories 每项必须是对象"))
                continue
            issues.extend(check_deferred_story(raw_story))
    issues.extend(check_evidence_profiles(matrix, [story for story in stories if isinstance(story, dict)]))
    if complete_module:
        issues.extend(check_module_completion(matrix, complete_module))

    expected_doc = build_markdown(matrix)
    if DOC.exists() and DOC.read_text(encoding="utf-8") != expected_doc:
        issues.append(Issue("<doc>", "docs", f"{rel(DOC)} 与 quality-matrix.toml 不同步；运行 --write-doc"))
    elif not DOC.exists():
        issues.append(Issue("<doc>", "docs", f"{rel(DOC)} 不存在；运行 --write-doc"))
    return issues


def build_markdown(matrix: dict[str, Any]) -> str:
    stories = [story for story in matrix.get("stories", []) if isinstance(story, dict)]
    deferred = [story for story in matrix.get("deferred_stories", []) if isinstance(story, dict)]
    total = len(stories) + len(deferred)
    completion_rate = (len(stories) / total * 100) if total else 0
    module_counts = Counter(story.get("module", "-") for story in [*stories, *deferred])
    completed_by_module = Counter(story.get("module", "-") for story in stories)
    deferred_by_module = Counter(story.get("module", "-") for story in deferred)
    lines = [
        "# 全链路质量矩阵",
        "",
        "> 本文件由 `governance/quality-matrix.toml` 生成。不要手工改表格；修改事实源后运行 `python3 scripts/governance/check_quality_matrix.py --write-doc`。",
        "",
        "## 范围",
        "",
        "- 强门禁范围：M1、M2、M3、M4 和已进入执行的 H 层横向能力。",
        "- 状态只允许 `verified` 或 `not_applicable`；不适用必须在事实源写原因。",
        "- S2 测试层由故事类型自动推导。",
        "- 对抗攻击类 A1-A8 由故事类型推导，映射 L4/L6/L8/L11；T1 不强制填写，`--complete-module` 才检查覆盖。",
        "- 验收深度由故事类型自动推导：S1 查询/展示，S2 普通写操作，S3 库存/并发/关键路径/GSP，S4 PDA/离线/硬件/外部系统/发布。",
        "",
        "## 状态摘要",
        "",
        "| 指标 | 数量 |",
        "|---|---:|",
        f"| 故事总数 | {total} |",
        f"| 已完成（已验证） | {len(stories)} |",
        f"| 未完成 / 延期 | {len(deferred)} |",
        f"| 完成率 | {completion_rate:.1f}% |",
        "",
        "> “已完成”表示故事已进入 `stories` 并通过矩阵维度门禁；延期故事中的局部代码、页面或测试切片不计入完成。",
        "",
        "## 模块进度",
        "",
        "| 模块 | 已完成 | 未完成 / 延期 | 总数 |",
        "|---|---:|---:|---:|",
    ]
    for module in sorted(module_counts):
        lines.append(
            f"| {module} | {completed_by_module[module]} | {deferred_by_module[module]} | {module_counts[module]} |"
        )
    lines.extend(
        [
            "",
            "## 已完成故事",
            "",
            "| 故事 | 模块 | 验收层级 |",
            "|---|---|---|",
        ]
    )
    for story in stories:
        types = story.get("types", [])
        lines.append(
            f"| {story.get('id', '-')} {story.get('title', '-')} | {story.get('module', '-')} | "
            f"{derive_acceptance_level(types) if isinstance(types, list) else '-'} |"
        )
    if deferred:
        lines.extend(
            [
                "",
                "## 未完成 / 延期故事",
                "",
                "| 故事 | 模块 | 验收层级 | 测试层 | 当前原因 |",
                "|---|---|---|---|---|",
            ]
        )
        for story in deferred:
            types = story.get("types")
            has_types = isinstance(types, list) and bool(types)
            layers = derive_required_layers(types) if has_types else []
            lines.append(
                "| {id} {title} | {module} | {level} | {layers} | {reason} |".format(
                    id=story.get("id", "-"),
                    title=story.get("title", "-"),
                    module=story.get("module", "-"),
                    level=derive_acceptance_level(types) if has_types else "-",
                    layers="、".join(layers) or "-",
                    reason=story.get("reason", "-"),
                )
            )
    lines.extend(
        [
            "",
            "## 验证故事详细矩阵",
            "",
            "| 故事 | 模块 | 验收层级 | 类型 | 测试层 | 前端 | API | 状态 |",
            "|---|---|---|---|---|---|---|---|",
        ]
    )
    for story in stories:
        types = story.get("types", [])
        layers = derive_required_layers(types) if isinstance(types, list) else []
        frontend_pages = "、".join(story.get("frontend_pages", [])) or "-"
        api_paths = "<br>".join(story.get("api_paths", [])) or "-"
        statuses = []
        for dimension in DIMENSIONS:
            status = dimension_status(story, dimension) or "missing"
            statuses.append(f"{dimension}:{status}")
        lines.append(
            "| {id} {title} | {module} | {level} | {types} | {layers} | {frontend} | {api} | {status} |".format(
                id=story.get("id", "-"),
                title=story.get("title", "-"),
                module=story.get("module", "-"),
                level=derive_acceptance_level(types) if isinstance(types, list) else "-",
                types="、".join(types) if isinstance(types, list) else "-",
                layers="、".join(layers) or "-",
                frontend=frontend_pages,
                api=api_paths,
                status="<br>".join(statuses),
            ),
        )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--write-doc", action="store_true", help="按 TOML 刷新 MkDocs 展示页")
    parser.add_argument("--complete-module", help="模块验收：只要仍有延期故事即失败")
    args = parser.parse_args(argv)

    if args.write_doc:
        matrix = load_matrix()
        DOC.parent.mkdir(parents=True, exist_ok=True)
        DOC.write_text(build_markdown(matrix), encoding="utf-8")

    issues = scan(complete_module=args.complete_module)
    payload = {
        "check": "check_quality_matrix",
        "tier": "T1",
        "category": "流程治理",
        "matrix": rel(MATRIX),
        "doc": rel(DOC),
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }
    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_quality_matrix (T1, 流程治理)")
        if issues:
            print(f"  ✘ {len(issues)} 个质量矩阵问题:")
            for issue in issues:
                print(f"    - {issue.story_id} [{issue.dimension}]: {issue.message}")
        else:
            print("  ✓ 全链路质量矩阵通过")
    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
