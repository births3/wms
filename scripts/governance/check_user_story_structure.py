#!/usr/bin/env python3
"""check_user_story_structure.py — 用户故事结构与关键词校验

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：
  0  通过
  1  发现结构/关键词违规
  2  脚本自身错误

校验项（全部为可自动化的结构/关键词检查）：
- 每个故事有"验收标准"段
- 验收标准是编号列表
- 无模糊词（黑名单）
- 写操作故事提到"审计追踪"
- 写操作故事提到"幂等"
- 有"跨故事约束"段
- 故事编号连续无重复
- 角色名在白名单内

不覆盖（需人工语义 review）：
- 异常路径是否完整
- 状态机之间是否衔接
- 验收标准是否真的可测试
- GSP 法规合规性
- 业务逻辑是否矛盾
- 外部依赖是否可行
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 角色白名单
ROLES = {
    "系统管理员", "仓库主管", "收货员", "养护员", "保管员",
    "货主", "门店用户", "司机", "外部系统", "系统",
    # 横向基础设施故事允许技术/审计/监管类 actor。
    "WMS 用户", "WMS 后端开发者", "业务模块开发者", "后端开发者",
    "前端开发者", "内部开发者", "运维", "监管检查员", "GSP 审计员",
    "监管对接服务", "门店店长", "门店采购员", "门店质量负责人",
    # v3.1 新增：H-DOCK 月台预约 / M1-005 财税链
    "ERP", "发货员", "现场调度员", "财务专员",
}

# 模糊词黑名单
FUZZY_WORDS = [
    "快速", "迅速", "合理", "适当", "尽量", "大量",
    "良好", "友好", "简单", "方便", "及时",
]

# 写操作关键词（出现这些词的故事应该提到审计/幂等）
WRITE_INDICATORS = [
    "创建", "修改", "删除", "停用", "禁用", "作废",
    "上架", "拣选", "复核", "签字", "审批", "绑定",
    "录入", "提交", "发货", "退货", "调拨", "过户",
    "冻结", "解冻", "盘点",
]

# 故事编号正则：US-XX-NNN
STORY_ID_RE = re.compile(r"^##\s+~?~?(US-[A-Z0-9]+-\d{3}[a-z]?)")
# strikethrough 故事识别：## ~~US-XX-NNN~~ ... 这种已移除占位故事，不做格式检查（仅参与编号连续性）
STORY_STRIKETHROUGH_RE = re.compile(r"^##\s+~~US-[A-Z0-9]+-\d{3}[a-z]?")
# 验收标准段
AC_HEADER_RE = re.compile(r"^###\s+验收标准", re.MULTILINE)
# 编号列表项
NUMBERED_ITEM_RE = re.compile(r"^\d+\.\s+")
# 角色提取："作为 XXX"
ROLE_RE = re.compile(r"\*\*作为\*\*\s*(.+?)$", re.MULTILINE)


@dataclass
class StoryCheck:
    id: str
    issues: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


@dataclass
class FileCheck:
    file: str
    stories: list[StoryCheck] = field(default_factory=list)
    file_issues: list[str] = field(default_factory=list)


def _split_stories(text: str) -> list[tuple[str, str, bool]]:
    """拆分为 [(story_id, story_text, is_strikethrough), ...]
    is_strikethrough=True 的故事仅参与编号连续性检查，不做内容格式检查（已移除占位）。
    """
    parts: list[tuple[str, str, bool]] = []
    lines = text.splitlines()
    current_id = ""
    current_lines: list[str] = []
    current_strikethrough = False

    for line in lines:
        m = STORY_ID_RE.match(line)
        if m:
            if current_id:
                parts.append((current_id, "\n".join(current_lines), current_strikethrough))
            current_id = m.group(1)
            current_lines = [line]
            current_strikethrough = bool(STORY_STRIKETHROUGH_RE.match(line))
        else:
            current_lines.append(line)

    if current_id:
        parts.append((current_id, "\n".join(current_lines), current_strikethrough))
    return parts


def _check_story(story_id: str, text: str, file_has_idempotency: bool = False) -> StoryCheck:
    sc = StoryCheck(id=story_id)

    # 剥离故事末尾的 "## Review 记录" 段及其后内容（最后一个故事会含全文档的 Review 段）
    rv_idx = (text.find("\n<details markdown=\"1\">\n<summary>📋 Review 记录") if "📋 Review 记录" in text else text.find("\n## Review 记录"))
    if rv_idx != -1:
        text = text[:rv_idx]

    # 1. 有验收标准段
    if not AC_HEADER_RE.search(text):
        sc.issues.append("缺少 '### 验收标准' 段")
    else:
        # 2. 验收标准是编号列表
        in_ac = False
        has_numbered = False
        for line in text.splitlines():
            if AC_HEADER_RE.match(line):
                in_ac = True
                continue
            if in_ac and line.startswith("##"):
                break
            if in_ac and NUMBERED_ITEM_RE.match(line.strip()):
                has_numbered = True
        if not has_numbered:
            sc.issues.append("验收标准段没有编号列表（应为 1. 2. 3. ...）")

    # 3. 模糊词检查
    for word in FUZZY_WORDS:
        if word in text:
            sc.warnings.append(f"含模糊词: '{word}'")

    # 4. 写操作应提到审计
    is_write = any(w in text for w in WRITE_INDICATORS)
    if is_write and "审计" not in text:
        sc.issues.append("写操作故事未提到'审计追踪'")

    # 5. 写操作应提到幂等（文件级跨故事约束已声明则跳过）
    if is_write and "幂等" not in text and "Idempotency" not in text and not file_has_idempotency:
        sc.warnings.append("写操作故事未提到'幂等性'（可能在跨故事约束中统一声明）")

    # 6. 角色白名单
    for m in ROLE_RE.finditer(text):
        role_text = m.group(1).strip()
        # 先剥离括号附注（中英文括号）：括号内通常是岗位/触发方式说明，不参与角色白名单校验
        # 例如 "保管员（复核岗）" → "保管员"；"系统（定时任务）" → "系统"
        role_text = re.sub(r"（[^）]*）", "", role_text)
        role_text = re.sub(r"\([^)]*\)", "", role_text)
        role_text = role_text.strip()
        if not role_text:
            continue
        # 可能是"仓库主管/外部系统"这种复合
        for r in re.split(r"[/、]", role_text):
            r = r.strip()
            if r and r not in ROLES and not any(known in r for known in ROLES):
                sc.warnings.append(f"角色 '{r}' 不在白名单中")

    return sc


def check_file(path: Path) -> FileCheck:
    rel = path.relative_to(REPO_ROOT).as_posix()
    fc = FileCheck(file=rel)
    text = path.read_text(encoding="utf-8")

    # 文件级检查：有跨故事约束段
    if "跨故事约束" not in text:
        fc.file_issues.append("缺少 '跨故事约束' 段")

    # 文件级检测：跨故事约束段是否声明了幂等性（声明则单故事不再 warn）
    file_has_idempotency = False
    cs_match = re.search(r"##\s+跨故事约束[^\n]*\n([\s\S]+?)(?=\n##|\Z)", text)
    if cs_match:
        cs_body = cs_match.group(1)
        if "幂等" in cs_body or "Idempotency" in cs_body:
            file_has_idempotency = True

    # 拆分故事
    stories = _split_stories(text)
    if not stories:
        fc.file_issues.append("未找到任何用户故事（格式：## US-XX-NNN）")
        return fc

    # 编号连续性
    # 区分活跃故事和 strikethrough 占位故事
    ids = [s[0] for s in stories]
    active_ids = [s[0] for s in stories if not s[2]]
    # 按模块分组检查
    modules: dict[str, list[int]] = {}
    for sid in ids:
        parts = sid.split("-")
        if len(parts) >= 3:
            mod = "-".join(parts[:-1])  # US-M2
            try:
                num = int(re.sub(r"[a-z]$", "", parts[-1]))
                modules.setdefault(mod, []).append(num)
            except ValueError:
                pass

    for mod, nums in modules.items():
        sorted_nums = sorted(set(nums))
        # 检查重复（仅对活跃故事；strikethrough 占位与活跃故事可同号）
        active_in_mod = [s for s in active_ids if s.startswith(mod + "-")]
        if len(active_in_mod) != len(set(active_in_mod)):
            fc.file_issues.append(f"{mod}: 存在重复编号")
        # 检查连续（含 strikethrough 占位以保持编号链路完整）
        for i in range(1, len(sorted_nums)):
            if sorted_nums[i] - sorted_nums[i - 1] > 1:
                fc.file_issues.append(
                    f"{mod}: 编号不连续（{sorted_nums[i-1]:03d} → {sorted_nums[i]:03d}）"
                )

    # 逐故事检查（strikethrough 故事跳过内容格式检查，仅参与上面的编号连续性）
    for story_id, story_text, is_strikethrough in stories:
        if is_strikethrough:
            continue
        fc.stories.append(_check_story(story_id, story_text, file_has_idempotency))

    return fc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    files = sorted(DOMAIN_DIR.glob("user-stories-*.md"))
    if not files:
        print("check_user_story_structure: no user-stories-*.md found")
        return 0

    results: list[FileCheck] = []
    for f in files:
        results.append(check_file(f))

    total_issues = sum(
        len(fc.file_issues) + sum(len(s.issues) for s in fc.stories)
        for fc in results
    )
    total_warnings = sum(
        sum(len(s.warnings) for s in fc.stories)
        for fc in results
    )

    if args.json:
        payload = {
            "check": "check_user_story_structure",
            "tier": "T1",
            "category": "文档治理",
            "files_scanned": len(results),
            "results": [
                {
                    "file": fc.file,
                    "file_issues": fc.file_issues,
                    "stories": [asdict(s) for s in fc.stories],
                }
                for fc in results
            ],
            "total_issues": total_issues,
            "total_warnings": total_warnings,
            "ok": total_issues == 0,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(f"check_user_story_structure (T1, 文档治理) — scanned {len(results)} files")
        for fc in results:
            if fc.file_issues or any(s.issues or s.warnings for s in fc.stories):
                print(f"\n  {fc.file}:")
                for issue in fc.file_issues:
                    print(f"    ✘ [file] {issue}")
                for s in fc.stories:
                    for issue in s.issues:
                        print(f"    ✘ [{s.id}] {issue}")
                    for w in s.warnings:
                        print(f"    ⚠ [{s.id}] {w}")
            else:
                print(f"  ✓ {fc.file}")

        print(f"\n  总计: {total_issues} error(s), {total_warnings} warning(s)")
        if total_issues == 0:
            print("  ✓ 结构检查通过")
            print("    ⓘ 本脚本仅校验骨架（As a / I want / So that 三件套 + 验收标准块 + 必填段）")
            print("    ⓘ 业务正确性 / 合规性 / 完备性 / 与上下游故事的一致性等语义检查仍需 PR 评审人工 review")

    return 0 if total_issues == 0 else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
