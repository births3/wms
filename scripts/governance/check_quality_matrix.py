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
DOC = REPO_ROOT / "docs" / "governance" / "quality-matrix.md"
OPENAPI_JSON = REPO_ROOT / "shared" / "openapi" / "openapi.json"
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
    "node apps/web-admin/self-checks/m2-inbound-page-helpers-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m2-inbound-page-helpers-self-check.mjs",
    "node apps/web-admin/self-checks/m2-putaway-strategy-self-check.mjs": REPO_ROOT
    / "apps/web-admin/self-checks/m2-putaway-strategy-self-check.mjs",
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
    "runtime_guard": {"L1", "L2", "L4", "L5", "L7", "L8", "L9", "L10", "L11"},
    "pda_runtime": {f"L{index}" for index in range(1, 12)},
    "hardware_runtime": {f"L{index}" for index in range(1, 12)},
    "external_runtime": {f"L{index}" for index in range(1, 12)},
    "release_runtime": {f"L{index}" for index in range(1, 12)},
}
S4_TYPES = {"pda_runtime", "hardware_runtime", "external_runtime", "release_runtime"}
S3_TYPES = {"inventory_change", "concurrent_resource", "critical_path", "audit_compliance"}


@dataclass(frozen=True)
class Issue:
    story_id: str
    dimension: str
    message: str


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def load_matrix() -> dict[str, Any]:
    return toml.loads(MATRIX.read_text(encoding="utf-8"))


def story_files() -> set[str]:
    return {rel(path) for path in REPO_ROOT.glob(STORY_GLOB)}


def openapi_paths() -> set[str]:
    payload = json.loads(OPENAPI_JSON.read_text(encoding="utf-8"))
    paths = payload.get("paths")
    if not isinstance(paths, dict):
        return set()
    return {
        f"{method.upper()} {path}"
        for path, operations in paths.items()
        if isinstance(operations, dict)
        for method in operations
    }


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
    """模块完成要求该模块不存在延期故事。"""
    return [
        Issue(
            str(story.get("id", "<missing>")),
            "module_completion",
            f"模块 {module} 仍有延期故事: {story.get('title', '-')}",
        )
        for story in matrix.get("deferred_stories", [])
        if isinstance(story, dict) and story.get("module") == module
    ]


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
                if isinstance(value, str) and not (repo_root / value).exists():
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
        "- 验收深度由故事类型自动推导：S1 查询/展示，S2 普通写操作，S3 库存/并发/关键路径/GSP，S4 PDA/硬件/外部系统/发布。",
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
                "| 故事 | 模块 | 当前原因 |",
                "|---|---|---|",
            ]
        )
        for story in deferred:
            lines.append(
                "| {id} {title} | {module} | {reason} |".format(
                    id=story.get("id", "-"),
                    title=story.get("title", "-"),
                    module=story.get("module", "-"),
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
