#!/usr/bin/env python3
"""check_bounded_contexts.py — 限界上下文与 Context Map 一致性校验

类别：1. 文档治理
Tier：T1（< 5s）
输入：
  docs/domain/<bc-slug>/module-manifest.toml（24 个 BC 的 manifest）
  docs/adr/0012-bounded-contexts.md（参考决策）
输出：人类可读 + --json
退出码：
  0 通过
  1 发现违规
  2 脚本自身错误

校验项：
  1. 24 个 BC 的 manifest 是否都已就位（普通模式报告，--strict 阻断）
  2. 每个 manifest 必有 [bounded_context] + [integrations] 段
  3. 集成模式在 8 种白名单内
  4. Shared Kernel 类型在 9 个白名单内
  5. 跨 BC 依赖图双向一致（A 声明依赖 B 时，B 也应承认 A）

适用范围：所有 docs/domain/<bc-slug>/module-manifest.toml
普通模式用于日常盘点；T4 使用 --strict，缺失 manifest 会阻断。
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:
    import tomli as tomllib

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 24 个 BC 清单（来自 ADR-0012）
BOUNDED_CONTEXTS = {
    # 12 横向能力
    "H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10",
    "H-DOCK", "H-AL",
    # 12 横向业务
    "M-TE", "M-RP", "M-PK", "M-VR", "M-QL", "M-CG", "M-SA",
    "M-RC", "M-DI", "M-BA", "M-PM", "M-TC",
}

# BC slug 到模块编号的映射（manifest 文件路径用 slug）
SLUG_TO_BC = {
    "h1": "H1", "h2": "H2", "h3": "H3", "h4": "H4", "h5": "H5",
    "h6": "H6", "h7": "H7", "h8": "H8", "h9": "H9", "h10": "H10",
    "h-dock": "H-DOCK", "h-al": "H-AL",
    "m-te": "M-TE", "m-rp": "M-RP", "m-pk": "M-PK", "m-vr": "M-VR",
    "m-ql": "M-QL", "m-cg": "M-CG", "m-sa": "M-SA", "m-rc": "M-RC",
    "m-di": "M-DI", "m-ba": "M-BA", "m-pm": "M-PM", "m-tc": "M-TC",
}

# 8 种集成模式
VALID_INTEGRATION_PATTERNS = {
    "Customer-Supplier",
    "Conformist",
    "Anti-Corruption Layer",
    "ACL",  # 简写
    "Open Host Service",
    "OHS",
    "Published Language",
    "Shared Kernel",
    "Partnership",
    "Separate Ways",
}

# 9 个 Shared Kernel 类型（来自 ADR-0012 §Shared Kernel 清单）
VALID_SHARED_KERNEL = {
    "OwnerId", "TenantId", "WarehouseId", "ProductCode", "BatchNo",
    "ApprovalSource", "OperatorId", "TraceId", "ErrorCode",
}


@dataclass
class BCManifest:
    bc_code: str
    file: str
    has_bc_section: bool = False
    has_integrations_section: bool = False
    integrations: dict = field(default_factory=dict)
    shared_kernel_provides: list = field(default_factory=list)
    shared_kernel_consumes: list = field(default_factory=list)


@dataclass
class Issue:
    bc: str
    severity: str  # error / warning / info
    rule: str
    message: str


def load_manifests() -> list[BCManifest]:
    """扫描 docs/domain/<slug>/module-manifest.toml。"""
    manifests: list[BCManifest] = []
    for d in sorted(DOMAIN_DIR.iterdir()):
        if not d.is_dir():
            continue
        slug = d.name.lower()
        bc_code = SLUG_TO_BC.get(slug)
        if not bc_code:
            continue
        m_path = d / "module-manifest.toml"
        if not m_path.exists():
            manifests.append(BCManifest(bc_code=bc_code, file=str(m_path)))
            continue
        try:
            with open(m_path, "rb") as f:
                data = tomllib.load(f)
            m = BCManifest(
                bc_code=bc_code,
                file=str(m_path.relative_to(REPO_ROOT)),
                has_bc_section="bounded_context" in data,
                has_integrations_section="integrations" in data,
                integrations=data.get("integrations", {}),
                shared_kernel_provides=data.get("shared_kernel", {}).get("provides", []),
                shared_kernel_consumes=data.get("shared_kernel", {}).get("consumes", []),
            )
            manifests.append(m)
        except Exception as e:
            manifests.append(BCManifest(bc_code=bc_code, file=str(m_path)))

    return manifests


def check(manifests: list[BCManifest]) -> list[Issue]:
    issues: list[Issue] = []
    found_bcs = {m.bc_code for m in manifests if Path(REPO_ROOT / m.file).exists()}

    # 1. 缺失的 BC manifest（普通模式盘点，strict 阻断）
    missing = BOUNDED_CONTEXTS - found_bcs
    for bc in sorted(missing):
        issues.append(Issue(bc, "info", "manifest_missing",
                            f"BC '{bc}' 的 module-manifest.toml 未创建（T4 strict 出口前必须补）"))

    # 2-5. 已存在的 manifest 校验
    for m in manifests:
        if not Path(REPO_ROOT / m.file).exists():
            continue

        if not m.has_bc_section:
            issues.append(Issue(m.bc_code, "warning", "no_bc_section",
                                f"manifest 缺 [bounded_context] 段"))

        if not m.has_integrations_section:
            issues.append(Issue(m.bc_code, "warning", "no_integrations_section",
                                f"manifest 缺 [integrations] 段"))
            continue

        # 3. 集成模式合法性
        for target_bc, pattern in m.integrations.items():
            if pattern not in VALID_INTEGRATION_PATTERNS:
                issues.append(Issue(m.bc_code, "error", "invalid_integration_pattern",
                                    f"集成模式 '{pattern}' (与 {target_bc}) 不在 8 种白名单内"))

        # 4. Shared Kernel 类型合法性
        for kernel_type in m.shared_kernel_provides + m.shared_kernel_consumes:
            if kernel_type not in VALID_SHARED_KERNEL:
                issues.append(Issue(m.bc_code, "error", "invalid_shared_kernel",
                                    f"Shared Kernel '{kernel_type}' 不在 9 个白名单内"))

    # 5. 双向一致性（A 声明依赖 B 时，B 应承认 A）
    integrations_map = {m.bc_code: m.integrations for m in manifests
                        if m.has_integrations_section}
    for bc_a, deps in integrations_map.items():
        for bc_b in deps:
            # 跳过外部系统（如 "码上放心"，不在 BC 清单内）
            if bc_b not in BOUNDED_CONTEXTS:
                continue
            if bc_b not in integrations_map:
                # B 没 manifest，前面已 info
                continue
            # B 应承认 A（不一定相同模式）
            if bc_a not in integrations_map[bc_b]:
                issues.append(Issue(bc_a, "info", "asymmetric_dependency",
                                    f"声明依赖 {bc_b} 但 {bc_b} 未承认（可能合理：单向 OHS）"))

    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="缺失、警告或错误均阻断")
    args = parser.parse_args(argv)

    manifests = load_manifests()
    issues = check(manifests)
    errors = [i for i in issues if i.severity == "error"]
    warnings = [i for i in issues if i.severity == "warning"]
    infos = [i for i in issues if i.severity == "info"]

    found_count = sum(1 for m in manifests if Path(REPO_ROOT / m.file).exists())

    if args.json:
        print(json.dumps({
            "check": "check_bounded_contexts",
            "tier": "T1",
            "category": "文档治理",
            "expected_bcs": len(BOUNDED_CONTEXTS),
            "found_manifests": found_count,
            "errors": [asdict(i) for i in errors],
            "warnings": [asdict(i) for i in warnings],
            "infos": [asdict(i) for i in infos],
            "strict": args.strict,
            "ok": not errors and not (args.strict and (warnings or infos)),
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_bounded_contexts (T1, 文档治理) — "
              f"{len(BOUNDED_CONTEXTS)} 个 BC / "
              f"{found_count} 个已有 manifest")

        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                print(f"    ✘ [{i.bc}] {i.rule}: {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项）：")
            for i in warnings:
                print(f"    ⚠ [{i.bc}] {i.rule}: {i.message}")
        if infos:
            print(f"\n  信息（{len(infos)} 项）：")
            for i in infos[:5]:
                print(f"    ℹ [{i.bc}] {i.rule}: {i.message}")
            if len(infos) > 5:
                print(f"    ...还有 {len(infos)-5} 项")
        if not (errors or warnings or infos):
            print("  ✓ 所有 BC manifest 合规")

    return 1 if errors or (args.strict and (warnings or infos)) else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
