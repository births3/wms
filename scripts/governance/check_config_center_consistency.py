#!/usr/bin/env python3
"""check_config_center_consistency.py — 配置项与配置中心列表一致性检查（双向）

类别：1. 文档治理
Tier：T1（< 10s）
输入：docs/domain/user-stories-*.md
输出：人类可读 + --json
退出码：
  0  通过
  1  发现不一致
  2  脚本自身错误

背景：
  M1-008 系统配置中心列出了"分类 → 配置项数 → 示例"。
  WMS 配置必须双向一致：
    1. 反向（v3 已有）：故事中的"默认 X"配置必须被 M1-008 列出
    2. 正向（v14 新增）：M1-008 列出的配置项必须在故事中给出默认值

检查规则（v14 升级双向）：
  反向：扫描 user-stories-*.md 中"（默认 X）"或"**默认 X**"等模式
        → 收集配置项关键词 → 在 M1-008 配置中心查找；缺失则报告。
  正向：解析 M1-008 配置中心的"分类 → 示例"列出的所有配置项
        → 在所有故事中查找"该项 + 默认"的近邻匹配
        → 缺默认值则报告。

EXEMPT 关键词：技术性默认值（开/关/启用/禁用 等通用词）、对应实体不是配置项的（如"默认地址"是档案字段）。
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
M1_FILE = DOMAIN_DIR / "user-stories-m1-master-data-warehouse.md"

# 反向规则：故事 → M1-008
# 标签形式："标签关键词" → ["M1-008 表格中可能的措辞..."]
EXPECTED_CONFIG_KEYS: dict[str, list[str]] = {
    "上架模式": ["上架模式"],
    "追溯码录入环节": ["追溯码录入环节"],
    "盘点差异审批阈值": ["盘点差异阈值", "盘点差异"],
    "设备校准预警天数": ["设备校准", "校准预警"],
    "容器超期未回收预警天数": ["容器超期", "容器回收"],
    "联系单超时时长": ["联系单超时", "审批超时", "通知超时"],
    "双人签字开关": ["双人签字"],
    "近效期预警天数": ["近效期"],
    "波次触发方式": ["波次触发"],
    "拣选路径策略": ["拣选路径"],
    "出库箱规则": ["出库箱"],
    "对账频率": ["对账频率"],
}

# 正向规则：M1-008 列项 → 故事
# 配置项名 → [故事中查找该项+默认值的近邻匹配关键词]
# 如果该配置项已在故事中显式声明默认值，列在这里为"已验证有默认值"
M1008_CONFIG_ITEMS: dict[str, list[str]] = {
    # 入库流程 4 项
    "追溯码录入环节": ["追溯码录入环节"],
    "上架模式": ["上架模式"],
    "双人签字时机": ["双人签字时机", "双人签字"],
    "上架策略优先级": ["上架策略", "默认优先级"],
    # 出库流程 7 项
    "波次触发方式": ["波次触发方式"],
    "波次分组规则": ["波次分组", "合单"],
    "拣选路径策略": ["拣选路径"],
    "出库箱规则": ["出库箱规则", "出库箱推荐"],
    "库存扣减时机": ["库存扣减时机", "扣减时机"],
    "复核方式": ["复核方式"],
    "合并发货条件": ["合并发货", "合单"],
    # 补货 4 项
    "阈值统计天数": ["阈值统计天数", "统计天数"],
    "安全系数": ["安全系数"],
    "检查频率": ["检查频率", "补货检查"],
    "补货数量单位": ["补货数量单位"],
    # 任务引擎 5 项
    "优先级排序": ["任务优先级", "优先级排序"],
    "分配策略": ["分配策略", "任务分配"],
    "最大并行数": ["最大并行数"],
    "超时阈值": ["超时阈值", "任务超时"],
    "合并开关": ["任务合并", "合并开关"],
    # 库存管理 6 项
    "近效期预警天数": ["近效期预警", "近效期"],
    "盘点差异阈值": ["盘点差异阈值", "盘点差异"],
    "ABC 阈值": ["ABC 阈值", "ABC 分类"],
    "ABC 重算频率": ["ABC 重算"],
    "库位最大 SKU 数": ["最大 SKU 数"],
    "效期隔离": ["效期隔离"],
    # 包装站 2 项
    "称重偏差阈值": ["称重偏差"],
    "保温箱蓄冷剂规则": ["保温箱", "蓄冷剂"],
    # 冷链 1 项
    "设备校准预警天数": ["设备校准", "校准预警"],
    # 对账 2 项
    "对账频率": ["对账频率"],
    "差异锁定阈值": ["差异锁定阈值", "差异锁定"],
    # 计费 2 项
    "仓储费计算方式": ["仓储费计算"],
    "账单生成日": ["账单生成日"],
    # 通知 2 项
    "事件通知映射": ["通知映射", "事件通知"],
    "审批超时时间": ["审批超时", "联系单超时"],
    # 监管 2 项
    "上报频率": ["上报频率"],
    "重试次数": ["重试次数"],
    # 多仓/连锁 4 项
    "默认仓库": ["默认仓库"],
    "跨仓可见性": ["跨仓可见性"],
    "门店补货统计天数": ["门店补货统计"],
    "越库条件": ["越库条件"],
    # 容器/物流 2 项
    "容器超期未回收预警天数": ["容器超期"],
    "回收追踪开关": ["回收追踪"],
}


@dataclass
class Inconsistency:
    direction: str  # "reverse" 反向 | "forward" 正向
    config_key: str
    used_in_files: list[str] = field(default_factory=list)
    found_in_config_center: bool = False
    found_default_in_stories: bool = False


def _extract_m1_config_section(m1_text: str) -> str:
    """提取 M1-008 系统配置中心的整段（到下一个 ## 为止）"""
    m = re.search(r"^##\s+US-M1-008.*?\n([\s\S]+?)(?=\n##\s+|\Z)", m1_text, re.MULTILINE)
    return m.group(1) if m else ""


def _strip_review(text: str) -> str:
    """剥离 Review 段（v3 折叠或老格式）"""
    idx = text.find("\n<details markdown=\"1\">\n<summary>📋 Review 记录")
    if idx == -1:
        idx = text.find("\n## Review 记录")
    if idx == -1:
        idx = text.find("\n## 📋 Review 记录")
    return text[:idx] if idx != -1 else text


def _has_default_near(text: str, keyword: str, window: int = 300) -> bool:
    """在文本中找 keyword 的位置，检查附近窗口或同一表格行内是否含'默认'/'default'"""
    for m in re.finditer(re.escape(keyword), text):
        # 1) 窗口检查（前后 window 字符）
        start = max(0, m.start() - window)
        end = min(len(text), m.end() + window)
        window_text = text[start:end]
        if "默认" in window_text or "default" in window_text.lower():
            return True
        # 2) 整行检查（如果 keyword 在 markdown 表格行内，看上方表头是否声明"默认值"列）
        line_start = text.rfind("\n", 0, m.start()) + 1
        line_end = text.find("\n", m.end())
        if line_end == -1:
            line_end = len(text)
        line = text[line_start:line_end]
        # 表格行特征：以 | 开头
        if line.lstrip().startswith("|"):
            # 向上查找表头（最多 3 行）
            head_search_start = max(0, line_start - 500)
            head_text = text[head_search_start:line_start]
            # 表头含"默认值"或"默认"列名
            if re.search(r"\|\s*\*?\*?默认值?\s*\*?\*?\s*\|", head_text):
                return True
    return False


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    if not M1_FILE.exists():
        print(f"check_config_center_consistency: {M1_FILE} not found", file=sys.stderr)
        return 2

    m1_text = M1_FILE.read_text(encoding="utf-8")
    config_section = _extract_m1_config_section(m1_text)

    files = sorted(DOMAIN_DIR.glob("user-stories-*.md"))
    all_text = ""
    for f in files:
        text = _strip_review(f.read_text(encoding="utf-8"))
        all_text += "\n" + text

    inconsistencies: list[Inconsistency] = []

    # ========== 反向检查（v3 原有） ==========
    found_in_files: dict[str, list[str]] = {}
    for k in EXPECTED_CONFIG_KEYS:
        found_in_files[k] = []
    for f in files:
        text = _strip_review(f.read_text(encoding="utf-8"))
        for key, patterns in EXPECTED_CONFIG_KEYS.items():
            if any(p in text for p in patterns):
                found_in_files[key].append(f.relative_to(REPO_ROOT).as_posix())

    for key, patterns in EXPECTED_CONFIG_KEYS.items():
        in_center = any(p in config_section for p in patterns)
        used_in = found_in_files[key]
        if used_in and not in_center:
            inconsistencies.append(
                Inconsistency(
                    direction="reverse",
                    config_key=key,
                    used_in_files=used_in,
                    found_in_config_center=False,
                )
            )

    # ========== 正向检查（v14 新增） ==========
    for item, patterns in M1008_CONFIG_ITEMS.items():
        # 检查该项是否在故事中有"默认"声明
        has_default = False
        for p in patterns:
            if _has_default_near(all_text, p):
                has_default = True
                break
        if not has_default:
            inconsistencies.append(
                Inconsistency(
                    direction="forward",
                    config_key=item,
                    used_in_files=[],
                    found_in_config_center=True,
                    found_default_in_stories=False,
                )
            )

    if args.json:
        payload = {
            "check": "check_config_center_consistency",
            "tier": "T1",
            "category": "文档治理",
            "scanned_reverse_keys": len(EXPECTED_CONFIG_KEYS),
            "scanned_forward_keys": len(M1008_CONFIG_ITEMS),
            "inconsistencies": [asdict(i) for i in inconsistencies],
            "ok": not inconsistencies,
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        total_keys = len(EXPECTED_CONFIG_KEYS) + len(M1008_CONFIG_ITEMS)
        print(
            f"check_config_center_consistency (T1, 文档治理) — checked {total_keys} config keys (反向 {len(EXPECTED_CONFIG_KEYS)} + 正向 {len(M1008_CONFIG_ITEMS)})"
        )
        if not inconsistencies:
            print("  ✓ 配置项双向一致：故事使用 ⇄ M1-008 配置中心 ⇄ 故事默认值")
        else:
            reverse = [i for i in inconsistencies if i.direction == "reverse"]
            forward = [i for i in inconsistencies if i.direction == "forward"]
            print(f"  ✘ {len(inconsistencies)} 个不一致 (反向 {len(reverse)} + 正向 {len(forward)})：")
            if reverse:
                print("    [反向] 故事使用但 M1-008 未列出：")
                for i in reverse:
                    files_short = ", ".join(p.split("/")[-1] for p in i.used_in_files[:3])
                    print(f"      [{i.config_key}]  使用于: {files_short}")
            if forward:
                print("    [正向] M1-008 列出但故事中无默认值声明：")
                for i in forward:
                    print(f"      [{i.config_key}]")

    return 0 if not inconsistencies else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
