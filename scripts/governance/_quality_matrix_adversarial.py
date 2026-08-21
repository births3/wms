"""质量矩阵对抗测试检查。由 check_quality_matrix 调用，避免主脚本触达 800 行门禁。"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Any

try:
    import tomllib as toml
except ModuleNotFoundError:
    import tomli as toml

_ADVERSARIAL_CATALOG_CACHE: dict[str, Any] | None = None


def _catalog_path() -> Path:
    from check_quality_matrix import ADVERSARIAL_CATALOG

    return ADVERSARIAL_CATALOG


def load_adversarial_catalog() -> dict[str, Any]:
    global _ADVERSARIAL_CATALOG_CACHE
    if _ADVERSARIAL_CATALOG_CACHE is None:
        _ADVERSARIAL_CATALOG_CACHE = toml.loads(_catalog_path().read_text(encoding="utf-8"))
    return _ADVERSARIAL_CATALOG_CACHE


def derive_required_attack_classes(types: list[str]) -> list[str]:
    """由故事类型并集推导对抗攻击类；不新增 L12。"""
    required_by_type = load_adversarial_catalog().get("required_by_type", {})
    classes: set[str] = set()
    for story_type in types:
        entry = required_by_type.get(story_type, {})
        if not isinstance(entry, dict):
            continue
        values = entry.get("classes", [])
        if isinstance(values, list):
            classes.update(str(item) for item in values if item)
    return sorted(classes)


def _story_owns_adversarial_test(story: dict[str, Any], spec: str) -> bool:
    """对抗测试必须落在本故事 evidence_refs / test_checks，禁止跨故事借用。"""
    path, _, fn_name = spec.partition("::")
    path = path.replace("\\", "/")
    refs = [
        str(value).replace("\\", "/")
        for value in (story.get("evidence_refs") or [])
        if isinstance(value, str)
    ]
    checks = [str(value) for value in (story.get("test_checks") or []) if isinstance(value, str)]
    if path in refs:
        return True
    for ref in refs:
        crate_dir = ref.removesuffix(".rs")
        if path.startswith(f"{crate_dir}/"):
            return True
    for check in checks:
        match = re.search(r"--test\s+([A-Za-z0-9_]+)", check)
        if match:
            crate = match.group(1)
            if path.endswith(f"/tests/{crate}.rs") or f"/tests/{crate}/" in f"/{path}":
                return True
        stem = Path(path).stem
        if re.search(rf"--lib\s+{re.escape(stem)}\b", check):
            return True
        if fn_name and re.search(rf"\b{re.escape(fn_name)}\b", check):
            return True
        if path in check.replace("\\", "/"):
            return True
    return False


def _adversarial_test_error(spec: str) -> str | None:
    from check_quality_matrix import REPO_ROOT

    match = re.fullmatch(r"(?P<path>.+\.rs)::(?P<name>[A-Za-z0-9_]+)", spec)
    if not match:
        return f"adversarial_checks.test 格式必须是 'path.rs::fn_name': {spec}"
    rel_path = match.group("path")
    path = REPO_ROOT / rel_path
    if not path.is_file():
        return f"对抗测试文件不存在: {rel_path}"
    name = match.group("name")
    text = path.read_text(encoding="utf-8")
    if not re.search(
        rf"#\[(?:sqlx::test|tokio::test|test)[^\]]*\]\s*(?:async\s+)?fn\s+{re.escape(name)}\s*\(",
        text,
        flags=re.S,
    ):
        return f"对抗测试必须指向带 #[test]/#[tokio::test]/#[sqlx::test] 的函数: {spec}"
    return None


def check_adversarial_checks(story: dict[str, Any], *, require_coverage: bool) -> list[Any]:
    """T1 只校验已填写的条目；模块验收才要求 types 推导的攻击类齐全。"""
    from check_quality_matrix import Issue

    story_id = str(story.get("id", "<missing>"))
    types = story.get("types", [])
    required = derive_required_attack_classes(
        [item for item in types if isinstance(item, str)] if isinstance(types, list) else []
    )
    checks = story.get("adversarial_checks")
    issues: list[Any] = []
    if checks is None:
        if require_coverage and required:
            issues.append(
                Issue(story_id, "adversarial", f"模块验收缺少攻击类: {', '.join(required)}")
            )
        return issues
    if not isinstance(checks, list) or not checks:
        return [Issue(story_id, "adversarial", "adversarial_checks 必须是非空对象数组")]

    known_classes = {
        key
        for key in (load_adversarial_catalog().get("classes") or {})
        if isinstance(key, str)
    }
    declared: set[str] = set()
    for item in checks:
        if not isinstance(item, dict):
            issues.append(Issue(story_id, "adversarial", "adversarial_checks 每项必须是对象"))
            continue
        attack_id = item.get("id")
        test = item.get("test")
        if not isinstance(attack_id, str) or attack_id not in known_classes:
            issues.append(Issue(story_id, "adversarial", f"未知攻击类: {attack_id}"))
            continue
        if attack_id in declared:
            issues.append(Issue(story_id, "adversarial", f"攻击类重复登记: {attack_id}"))
            continue
        declared.add(attack_id)
        if not isinstance(test, str) or not test.strip():
            issues.append(Issue(story_id, "adversarial", f"{attack_id} 缺少 test"))
            continue
        spec = test.strip()
        error = _adversarial_test_error(spec)
        if error:
            issues.append(Issue(story_id, "adversarial", error))
            continue
        if not _story_owns_adversarial_test(story, spec):
            issues.append(
                Issue(
                    story_id,
                    "adversarial",
                    f"{attack_id} 测试未进入本故事 evidence_refs 或 test_checks: {spec}",
                )
            )
    if require_coverage:
        missing = [item for item in required if item not in declared]
        if missing:
            issues.append(
                Issue(story_id, "adversarial", f"模块验收缺少攻击类: {', '.join(missing)}")
            )
    return issues
