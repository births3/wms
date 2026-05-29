#!/usr/bin/env python3
"""check_prototype_fidelity.py — 全量矩阵原型业务保真度治理

类别：6. 原型治理
Tier：T1（< 10s）
输入：prototypes/src/prototype-kit/full-matrix-specs.ts + prototype-model.ts
输出：人类可读 + --json
退出码：0 通过 / 1 违规 / 2 脚本错误

校验项：
- 全量矩阵中出现的每个 moduleCode 都必须在 MODULE_BLUEPRINTS 中有专属蓝图
- 每个模块蓝图必须至少包含 6 个 fields、4 个 steps、3 个 actions、2 个 exceptions
- 禁止回退到固定通用字段模板：故事 / 对象 / 单据号 / 批号 / 数量 / 审计
- UniversalPrototypePage 不得重新承载 makeFields/makeRows 等大模板函数
- prototype-matrix-r3.md 中标注“建议与 X 合并”的故事，Tabs.tsx 必须声明复用映射
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
KIT_DIR = REPO_ROOT / "prototypes" / "src" / "prototype-kit"
SPECS_FILE = KIT_DIR / "full-matrix-specs.ts"
MODEL_FILE = KIT_DIR / "prototype-model.ts"
UNIVERSAL_FILE = KIT_DIR / "UniversalPrototypePage.tsx"
TABS_FILE = REPO_ROOT / "prototypes" / "src" / "Tabs.tsx"
MATRIX_FILE = REPO_ROOT / "docs" / "prototypes" / "prototype-matrix-r3.md"

GENERIC_FIELDS = {"故事", "对象", "单据号", "批号", "数量", "审计"}
MODULE_RE = re.compile(r'^\s*([A-Z0-9]+):\s*\{(?P<body>.*)\},?\s*$', re.MULTILINE)
ARRAY_RE_TEMPLATE = r'{name}:\s*\[(.*?)\]'
STRING_RE = re.compile(r'"([^"]+)"')
SPEC_BLOCK_RE = re.compile(r'\{\s*storyId:\s*"[^"]+".*?\n\s*\},?', re.DOTALL)
SPEC_PROP_RE = re.compile(r'(storyId|title|end|reason|group|moduleCode|slug):\s*"([^"]*)"')
PROFILE_RE = re.compile(r'if\s*\(/(.+?)/([a-z]*)\.test\(text\)\)\s*\{.*?return\s*\{(.*?)\};', re.DOTALL)
MERGE_HINT_RE = re.compile(r"\|\s*\d+\s*\|\s*(US-[A-Z0-9-]+)\s*\|.*?\|\s*(PC|PDA|PAD|H5)(?:\+[^|]+)?\s*\|.*?建议与\s+([A-Z0-9-]+)\s+合并", re.IGNORECASE)

END_TO_SLUG_PREFIX = {
    "PC": "pc",
    "PDA": "pda",
    "PAD": "pad",
    "H5": "h5",
}


@dataclass
class Issue:
    kind: str
    target: str
    detail: str


@dataclass(frozen=True)
class SemanticTerm:
    name: str
    pattern: re.Pattern[str]
    model_terms: tuple[str, ...]


SEMANTIC_TERMS: tuple[SemanticTerm, ...] = (
    SemanticTerm("LPN", re.compile(r"\bLPN\b", re.IGNORECASE), ("LPN", "容器", "货箱", "周转箱")),
    SemanticTerm("容器", re.compile("容器"), ("容器", "LPN", "货箱", "周转箱")),
    SemanticTerm("绑定", re.compile("绑定"), ("绑定",)),
    SemanticTerm("解绑", re.compile("解绑"), ("解绑",)),
    SemanticTerm("回收", re.compile("回收"), ("回收", "归还")),
    SemanticTerm("追踪", re.compile("追踪|跟踪"), ("追踪", "跟踪", "轨迹", "时间线")),
    SemanticTerm("客户", re.compile("客户"), ("客户",)),
    SemanticTerm("门店", re.compile("门店"), ("门店",)),
    SemanticTerm("层级", re.compile("层级"), ("层级",)),
    SemanticTerm("多货主", re.compile("多货主"), ("多货主", "货主")),
    SemanticTerm("数据隔离", re.compile("数据隔离"), ("数据隔离", "租户隔离", "隔离范围")),
    SemanticTerm("特殊药品", re.compile("特殊药品|麻精|放射|血液制品"), ("特殊药品", "麻精", "放射", "血液制品")),
    SemanticTerm("养护", re.compile("养护"), ("养护",)),
    SemanticTerm("ABC", re.compile(r"\bABC\b", re.IGNORECASE), ("ABC", "分类")),
    SemanticTerm("合并", re.compile("合并"), ("合并",)),
    SemanticTerm("拆单", re.compile("拆单"), ("拆单", "拆分")),
    SemanticTerm("越库", re.compile("越库|Cross-Docking", re.IGNORECASE), ("越库", "Cross-Docking", "交叉月台")),
    SemanticTerm("退货", re.compile("退货"), ("退货",)),
    SemanticTerm("PIX", re.compile(r"\bPIX\b|三码"), ("PIX", "三码", "交易类型")),
    SemanticTerm("码库", re.compile("码库|大中小码"), ("码库", "大码", "中码", "小码")),
    SemanticTerm("Put-to-Light", re.compile("Put-to-Light", re.IGNORECASE), ("Put-to-Light", "格口", "点亮")),
    SemanticTerm("保温箱", re.compile("保温箱"), ("保温箱",)),
)


def _string_values_from_array(body: str, name: str) -> list[str]:
    m = re.search(ARRAY_RE_TEMPLATE.format(name=re.escape(name)), body)
    if not m:
        return []
    return STRING_RE.findall(m.group(1))


def _module_codes_from_specs() -> set[str]:
    if not SPECS_FILE.exists():
        return set()
    text = SPECS_FILE.read_text(encoding="utf-8")
    return set(re.findall(r'moduleCode:\s*"([A-Z0-9]+)"', text))


def _specs_from_specs() -> list[dict[str, str]]:
    if not SPECS_FILE.exists():
        return []
    text = SPECS_FILE.read_text(encoding="utf-8")
    specs: list[dict[str, str]] = []
    for block in SPEC_BLOCK_RE.findall(text):
        item = dict(SPEC_PROP_RE.findall(block))
        if {"storyId", "title", "end", "reason", "group", "moduleCode", "slug"}.issubset(item):
            specs.append(item)
    return specs


def _merge_hints_from_matrix() -> list[tuple[str, str, str, str]]:
    if not MATRIX_FILE.exists():
        return []
    text = MATRIX_FILE.read_text(encoding="utf-8")
    hints: list[tuple[str, str, str, str]] = []
    for m in MERGE_HINT_RE.finditer(text):
        story_id = m.group(1)
        end = m.group(2).upper()
        target = m.group(3)
        prefix = END_TO_SLUG_PREFIX[end]
        legacy_key = f"{prefix}-{story_id.lower()}"
        target_key = f"{prefix}-us-{target.lower()}"
        hints.append((legacy_key, story_id, target, target_key))
    return hints


def _tabs_legacy_mappings(text: str) -> dict[str, str]:
    body_m = re.search(r"const LEGACY_SLUGS:\s*Record<string,\s*string>\s*=\s*\{(.*?)\};", text, re.DOTALL)
    if not body_m:
        return {}
    return dict(re.findall(r'"([^"]+)"\s*:\s*"([^"]+)"', body_m.group(1)))


def _blueprints() -> dict[str, dict[str, list[str]]]:
    if not MODEL_FILE.exists():
        return {}
    text = MODEL_FILE.read_text(encoding="utf-8")
    out: dict[str, dict[str, list[str]]] = {}
    for m in MODULE_RE.finditer(text):
        code = m.group(1)
        body = m.group("body")
        out[code] = {
            "fields": _string_values_from_array(body, "fields"),
            "steps": _string_values_from_array(body, "steps"),
            "actions": _string_values_from_array(body, "actions"),
            "exceptions": _string_values_from_array(body, "exceptions"),
            "columns": _string_values_from_array(body, "columns"),
            "all": STRING_RE.findall(body),
        }
    return out


def _keyword_profiles() -> list[tuple[re.Pattern[str], list[str]]]:
    if not MODEL_FILE.exists():
        return []
    text = MODEL_FILE.read_text(encoding="utf-8")
    profiles: list[tuple[re.Pattern[str], list[str]]] = []
    for pattern_text, flags_text, body in PROFILE_RE.findall(text):
        flags = re.IGNORECASE if "i" in flags_text else 0
        try:
            pattern = re.compile(pattern_text, flags)
        except re.error:
            continue
        profiles.append((pattern, STRING_RE.findall(body)))
    return profiles


def _model_text_for_spec(
    spec: dict[str, str],
    blueprints: dict[str, dict[str, list[str]]],
    profiles: list[tuple[re.Pattern[str], list[str]]],
) -> str:
    values: list[str] = []
    values.extend(blueprints.get(spec["moduleCode"], {}).get("all", []))
    source_text = f'{spec["title"]}{spec["reason"]}{spec["group"]}'
    for pattern, profile_values in profiles:
        if pattern.search(source_text):
            values.extend(profile_values)
            break
    return "\n".join(values)


def _semantic_issues(
    specs: list[dict[str, str]],
    blueprints: dict[str, dict[str, list[str]]],
    profiles: list[tuple[re.Pattern[str], list[str]]],
    legacy_mappings: dict[str, str],
) -> list[Issue]:
    issues: list[Issue] = []
    for spec in specs:
        legacy_key = f'{spec["end"]}-{spec["storyId"].lower()}'
        if legacy_key in legacy_mappings:
            continue
        source_text = f'{spec["title"]}{spec["reason"]}{spec["group"]}'
        model_text = _model_text_for_spec(spec, blueprints, profiles)
        missing: list[str] = []
        for term in SEMANTIC_TERMS:
            if term.pattern.search(source_text) and not any(token in model_text for token in term.model_terms):
                missing.append(term.name)
        if missing:
            issues.append(Issue(
                "story_semantic_gap",
                spec["slug"],
                f'{spec["storyId"]}「{spec["title"]}」生成模型未覆盖主题词：{", ".join(missing)}',
            ))
    return issues


def run() -> list[Issue]:
    issues: list[Issue] = []

    for path in (SPECS_FILE, MODEL_FILE, UNIVERSAL_FILE, TABS_FILE, MATRIX_FILE):
      if not path.exists():
          issues.append(Issue("missing_file", path.relative_to(REPO_ROOT).as_posix(), "文件不存在"))
    if issues:
        return issues

    specs = _specs_from_specs()
    spec_modules = {spec["moduleCode"] for spec in specs} or _module_codes_from_specs()
    blueprints = _blueprints()
    profiles = _keyword_profiles()
    if not spec_modules:
        issues.append(Issue("parse_specs", SPECS_FILE.relative_to(REPO_ROOT).as_posix(), "未解析到 moduleCode"))
    if not specs:
        issues.append(Issue("parse_specs", SPECS_FILE.relative_to(REPO_ROOT).as_posix(), "未解析到全量原型 spec"))
    if not blueprints:
        issues.append(Issue("parse_blueprints", MODEL_FILE.relative_to(REPO_ROOT).as_posix(), "未解析到 MODULE_BLUEPRINTS"))
    if not profiles:
        issues.append(Issue("parse_keyword_profiles", MODEL_FILE.relative_to(REPO_ROOT).as_posix(), "未解析到 keywordProfile 分支"))

    for code in sorted(spec_modules - set(blueprints)):
        issues.append(Issue("missing_blueprint", code, "full-matrix-specs.ts 使用该 moduleCode，但 MODULE_BLUEPRINTS 未定义"))

    for code in sorted(spec_modules & set(blueprints)):
        bp = blueprints[code]
        thresholds = {"fields": 6, "steps": 4, "actions": 3, "exceptions": 2, "columns": 5}
        for key, min_count in thresholds.items():
            actual = len(bp.get(key, []))
            if actual < min_count:
                issues.append(Issue("thin_blueprint", code, f"{key} 仅 {actual} 项，少于 {min_count} 项"))
        fields = set(bp.get("fields", []))
        if GENERIC_FIELDS.issubset(fields):
            issues.append(Issue("generic_fields", code, "字段集合退回通用模板：故事/对象/单据号/批号/数量/审计"))
        if len(fields - GENERIC_FIELDS) < 4:
            issues.append(Issue("low_specificity", code, "模块专属字段少于 4 个"))

    universal_text = UNIVERSAL_FILE.read_text(encoding="utf-8")
    forbidden = ["function makeFields", "function makeRows", "function makeColumns", "const moduleName"]
    for token in forbidden:
        if token in universal_text:
            issues.append(Issue("universal_template_regression", "UniversalPrototypePage.tsx", f"禁止重新出现 {token}"))

    model_text = MODEL_FILE.read_text(encoding="utf-8")
    if "GENERIC_FIELDS" in model_text:
        issues.append(Issue("self_reference", "prototype-model.ts", "业务模型文件不应引用治理脚本内部常量名"))

    tabs_text = TABS_FILE.read_text(encoding="utf-8")
    legacy_mappings = _tabs_legacy_mappings(tabs_text)
    if not legacy_mappings:
        issues.append(Issue("missing_legacy_mapping_table", "Tabs.tsx", "未解析到 LEGACY_SLUGS 复用映射表"))
    if "LEGACY_SLUGS[legacyKeyFor(spec)]" not in tabs_text:
        issues.append(Issue("unused_legacy_mapping_table", "Tabs.tsx", "生成 tab 时未使用 LEGACY_SLUGS 复用映射表"))

    for legacy_key, story_id, target_story, target_key in _merge_hints_from_matrix():
        if legacy_key not in legacy_mappings:
            issues.append(Issue(
                "missing_merge_mapping",
                story_id,
                f"prototype-matrix-r3.md 标注建议与 {target_story} 合并，但 Tabs.tsx 未声明 \"{legacy_key}\" 复用映射",
            ))
            continue
        target_slug = legacy_mappings.get(target_key)
        if not target_slug:
            issues.append(Issue(
                "missing_merge_target_mapping",
                story_id,
                f"矩阵要求复用 {target_story}，但 Tabs.tsx 未声明 \"{target_key}\" 到手工页的映射",
            ))
            continue
        actual = legacy_mappings[legacy_key]
        if actual != target_slug:
            issues.append(Issue(
                "wrong_merge_mapping",
                story_id,
                f"矩阵要求复用 {target_story} 对应的 {target_slug}，Tabs.tsx 实际映射到 {actual}",
            ))

    issues.extend(_semantic_issues(specs, blueprints, profiles, legacy_mappings))

    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        issues = run()
    except Exception as e:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({
            "status": "fail" if issues else "pass",
            "issues": [asdict(i) for i in issues],
        }, ensure_ascii=False, indent=2))
    else:
        if issues:
            print(f"✗ check_prototype_fidelity: {len(issues)} 项违规")
            for issue in issues:
                print(f"  - [{issue.kind}] {issue.target}: {issue.detail}")
        else:
            print("✓ check_prototype_fidelity: 全量矩阵模块蓝图与故事专属字段通过")

    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
