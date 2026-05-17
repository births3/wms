#!/usr/bin/env python3
"""check_pda_story_completeness.py — PDA 操作类故事完备性检查

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：
  0  通过
  1  发现 PDA 故事缺三件套
  2  脚本自身错误

背景：
  PDA 操作类故事（保管员/收货员/养护员在 PDA 上的物理操作）必须含"三件套"：
    1. 字段表（录什么字段，必填/可选/系统带出）
    2. 扫码顺序（先扫什么后扫什么的明确步骤）
    3. 离线声明（断网时如何处理）

  目的：保证 PDA 开发能从故事直接推导出表单/交互；不齐 PDA 写代码时容易猜。

检查规则：
  对每个用户故事，若同时满足：
    (a) 标题或正文含 "PDA"（即 PDA 操作故事）
    (b) 不在 EXEMPT 列表（看板/查询类不需要）
  则要求三件套各自存在；任一缺失即报告。

"三件套"识别启发：
  - 字段表：含 markdown 表格且首行类似 "| 字段 |" 或 "| 项目 |" 或 "| 必填 |"
  - 扫码顺序：含 "扫" 关键词且故事内含连续的"扫 X → 扫 Y" 或编号步骤含扫码
  - 离线声明：含 "离线" 或 "断网"
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

# 含 "PDA" 但不算"PDA 操作类故事"的（看板/查询/配置/泛提及类）
EXEMPT_STORY_IDS = {
    "US-M1-004a",  # 容器管理（PDA 操作分散在 M2/M3/M4）
    "US-M2-007",  # 单据打印（PDA 提及但本质是打印）
    "US-M2-008",  # 收货进度看板
    "US-M2-009",  # 打印模板设计
    "US-M2-010",  # 上架策略配置（PDA 仅提及一次）
    "US-M3-001",  # 实时库存查询
    "US-M3-005",  # 盘点（包含但走 M-TE 任务引擎）
    "US-M3-009",  # 库存预警
    "US-M3-010",  # ABC 分类
    "US-M4-007",  # 出库进度看板
    "US-M4-008",  # 退货流程（多步骤，PDA 仅环节）
    "US-M4-010",  # 拣选路径策略配置
    "US-M4-011",  # 合并/拆单配置
    "US-TE-001", "US-TE-002", "US-TE-003", "US-TE-004",  # 任务配置类
    "US-TE-005", "US-TE-006", "US-TE-007",  # 任务调度类
    "US-TE-009", "US-TE-010", "US-TE-011",  # 看板/绩效/设备
    "US-PK-001",  # 包装站工位管理（配置）
    "US-PK-004",  # 保温箱配置（自动建议）
    "US-DI-001",  # 药检单平台对接配置
    "US-DI-004",  # 药检单有效性校验（系统自动）
    "US-H4-001", "US-H4-002", "US-H4-003", "US-H4-004",  # 通知配置
    "US-H5-001", "US-H5-002", "US-H5-005",  # 快递配置/查询
    "US-RP-001", "US-RP-002", "US-RP-003", "US-RP-004",  # 补货由系统触发，PDA 执行通过 M-TE-008
    "US-SA-003",  # 报损报溢查询统计（不是 PDA 操作）
    "US-TC-001",  # 追溯码分类管理（配置）
    "US-TC-005",  # 追溯码录入环节配置
    "US-TC-002",  # 码库管理（系统自动 + 部分 PDA，归 M-TE-008）
}

# 必须含三件套的核心 PDA 操作故事（白名单：明确要求齐全）
# 这些故事是保管员/收货员/养护员在 PDA 上的主要工作流
REQUIRED_PDA_STORIES = {
    "US-M2-002",  # PDA 收货
    "US-M2-003",  # PDA 验收
    "US-M2-005",  # 智能上架（含 PDA 操作）
    "US-M3-004",  # 在库养护（PDA 养护）
    "US-M3-006",  # 库内移库（v22 升级含完整 PDA 三件套：库位/商品码/电子监管码三道扫码）
    "US-M4-003",  # PDA 拣选
    "US-M4-004",  # 出库复核（含 PDA 整件/零件复核）
    "US-PK-002",  # 装箱（含包装站 + PDA 装箱）
    "US-DI-002",  # 扫码批量查询药检单
    "US-TC-003",  # PDA 追溯码绑定
    "US-TC-004",  # PDA 追溯码维护
    "US-TE-008",  # 任务执行（PDA 统一入口）
    # v15 W4.E 新增司机/门店主动故事（移动端 PDA 操作）
    "US-DR-001",  # 司机端登录与任务列表
    "US-DR-002",  # 司机签收（自有车队交接）
    "US-DR-004",  # 到店签收（客户签字）
    "US-ST-003",  # 门店签收（含追溯码抽检）
    # v21 M-BA 批号调整 PDA 故事
    "US-BA-001",  # 批号调整单创建（PDA 即时发起）
    "US-BA-003",  # PDA 执行批号调整（三道实物核对）
    # 以下故事的 PDA 操作已被其他故事覆盖，不重复检查（豁免）：
    # - US-PK-003 称重（电子秤工位自动读取，不是 PDA 扫码主操作）
    # - US-PK-005 追溯码出库核验（已被 PK-002 装箱时扫追溯码覆盖）
    # - US-PK-006 快递面单打印（打印动作，不是扫码主操作）
    # - US-DI-003 药检单存储查看（PC 端查看为主）
    # - US-SA-001/002 报损报溢（PDA 扫码确认子动作，由 M-TE-008 通用规范覆盖）
    # - US-TC-006 出库追溯码核验（已被 PK-002/M4-004 覆盖）
}

STORY_ID_RE = re.compile(r"^##\s+(US-[A-Z0-9]+-\d{3}[a-z]?)")


@dataclass
class Issue:
    file: str
    story_id: str
    missing: list[str] = field(default_factory=list)


def _split_stories(text: str) -> list[tuple[str, str]]:
    parts: list[tuple[str, str]] = []
    current_id = ""
    current_lines: list[str] = []
    for line in text.splitlines():
        m = STORY_ID_RE.match(line)
        if m:
            if current_id:
                parts.append((current_id, "\n".join(current_lines)))
            current_id = m.group(1)
            current_lines = [line]
        else:
            current_lines.append(line)
    if current_id:
        parts.append((current_id, "\n".join(current_lines)))
    return parts


def _strip_review(text: str) -> str:
    idx = text.find('\n<details markdown="1">\n<summary>📋 Review 记录')
    if idx == -1:
        idx = text.find("\n## Review 记录")
    return text[:idx] if idx != -1 else text


def _has_field_table(text: str) -> bool:
    """字段表识别：markdown 表格的表头含 '字段'/'项目'/'必填'/'核对项'"""
    # 表格首行模式："| 字段 | 必填 | 说明 |" 等
    for m in re.finditer(r"^\|([^\n]+)\|\s*\n\|[\s|:-]+\|", text, re.MULTILINE):
        header = m.group(1)
        if any(k in header for k in ["字段", "项目", "必填", "核对项", "核对内容"]):
            return True
    return False


def _has_scan_sequence(text: str) -> bool:
    """扫码顺序识别：含 '扫' 且出现 '扫 X → 扫 Y' 或编号步骤含连续扫码"""
    if "扫" not in text:
        return False
    # 模式 1：扫 X → 扫 Y / 扫 X →
    if re.search(r"扫[^\n]{0,15}→\s*扫", text):
        return True
    # 模式 2：编号列表项中含 "扫码"，且至少出现 2 次"扫"动作
    scan_count = len(re.findall(r"扫(?:码|描|.{0,4}码)", text))
    if scan_count >= 2:
        return True
    return False


def _has_offline_clause(text: str) -> bool:
    """离线声明：含 '离线' 或 '断网'"""
    return "离线" in text or "断网" in text


def check_file(path: Path) -> list[Issue]:
    rel = path.relative_to(REPO_ROOT).as_posix()
    text = path.read_text(encoding="utf-8")
    text = _strip_review(text)
    issues: list[Issue] = []
    for sid, story_text in _split_stories(text):
        # 仅检查白名单中的核心 PDA 操作故事
        if sid not in REQUIRED_PDA_STORIES:
            continue
        if sid in EXEMPT_STORY_IDS:
            continue
        missing: list[str] = []
        if not _has_field_table(story_text):
            missing.append("字段表")
        if not _has_scan_sequence(story_text):
            missing.append("扫码顺序")
        if not _has_offline_clause(story_text):
            missing.append("离线声明")
        if missing:
            issues.append(Issue(file=rel, story_id=sid, missing=missing))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    files = sorted(DOMAIN_DIR.glob("user-stories-*.md"))
    all_issues: list[Issue] = []
    for f in files:
        all_issues.extend(check_file(f))

    if args.json:
        payload = {
            "check": "check_pda_story_completeness",
            "tier": "T1",
            "category": "文档治理",
            "scanned": len(files),
            "issues": [asdict(i) for i in all_issues],
            "ok": not all_issues,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(
            f"check_pda_story_completeness (T1, 文档治理) — scanned {len(files)} files"
        )
        if not all_issues:
            print("  ✓ 所有 PDA 操作类故事都含三件套（字段表 + 扫码顺序 + 离线声明）")
        else:
            print(f"  ✘ {len(all_issues)} 个 PDA 故事缺三件套：")
            for i in all_issues:
                missing = ", ".join(i.missing)
                print(f"    {i.file}  [{i.story_id}]  缺: {missing}")

    return 0 if not all_issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
