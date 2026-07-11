#!/usr/bin/env python3
"""check_observability.py — 故事中 KPI 声明的合规校验

类别：1. 文档治理
Tier：T1（< 5s）
输入：
  docs/domain/user-stories-*.md（扫描 §跨故事约束 §10 KPI 声明）
  docs/adr/0011-observability.md（参考决策）
输出：人类可读 + --json
退出码：
  0 通过
  1 发现违规
  2 脚本自身错误

校验项：
  1. 核心写操作模块必须声明 KPI（每模块至少 5 个）
  2. KPI 命名符合 wms_<module>_<entity>_<action>_<metric_type>
  3. metric_type 后缀合法（_total/_seconds/_bytes/无后缀=Gauge）
  4. strict 模式检查运行时 metrics、trace context 和 OTel/tracing 接入

适用范围：核心模块（M1-M4 / M-VR / M-QL / M-TC / M-SA / H1 / H2）
不强制：辅助模块（已废弃/占位/Wave 5+ 模块）
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOMAIN_DIR = REPO_ROOT / "docs" / "domain"

# 核心写操作模块（必须声明 KPI）
CORE_MODULES = {
    "user-stories-h1-auth-tenant",
    "user-stories-h2-audit-trail",
    "user-stories-m1-master-data-product",
    "user-stories-m1-master-data-warehouse",
    "user-stories-m2-inbound-asn",
    "user-stories-m2-inbound-verify",
    "user-stories-m3-inventory-query",
    "user-stories-m3-inventory-operation",
    "user-stories-m4-outbound-order",
    "user-stories-m4-outbound-pick",
    "user-stories-m4-outbound-return",
    "user-stories-mvr-validation-rules",
    "user-stories-mql-quality-liaison",
    "user-stories-mtc-traceability-code",
    "user-stories-msa-stock-adjustment",
}

# KPI 命名规则
KPI_RE = re.compile(r"^wms_[a-z][a-z0-9_]+_[a-z][a-z0-9_]+_[a-z][a-z0-9_]+(_total|_seconds|_bytes|)$")
VALID_SUFFIXES = ("_total", "_seconds", "_bytes", "")


@dataclass
class ModuleKPI:
    file: str
    kpis: list[str] = field(default_factory=list)
    has_section: bool = False


@dataclass
class Issue:
    file: str
    severity: str  # error / warning
    rule: str
    message: str


def scan_kpis() -> list[ModuleKPI]:
    """扫描故事文件中的 §10 KPI 声明。

    识别模式：
    - "## 跨故事约束" 段中含 "KPI" 或 "metric" 关键字的子项
    - 或独立的 "### KPI" 段
    """
    results: list[ModuleKPI] = []
    for f in sorted(DOMAIN_DIR.glob("user-stories-*.md")):
        text = f.read_text(encoding="utf-8")
        rel = f.stem
        m = ModuleKPI(file=rel)

        # 找跨故事约束段
        block_match = re.search(
            r"^## 跨故事约束[^\n]*\n([\s\S]*?)(?=^## |\Z)",
            text,
            re.M,
        )
        if not block_match:
            results.append(m)
            continue
        block = block_match.group(1)

        # 找 KPI 子段（关键词：KPI / metric / 可观测性指标）
        if re.search(r"KPI|metric|可观测性指标", block, re.IGNORECASE):
            m.has_section = True
            # 提取所有形如 wms_xxx_yyy_zzz 的标识符
            for kpi in re.findall(r"`wms_[a-z][a-z0-9_]+`", block):
                m.kpis.append(kpi.strip("`"))

        # 也支持 ### KPI 独立段
        kpi_section = re.search(
            r"^### .*?(KPI|可观测性指标).*?\n([\s\S]*?)(?=^##|^### |\Z)",
            text,
            re.M,
        )
        if kpi_section:
            m.has_section = True
            for kpi in re.findall(r"`wms_[a-z][a-z0-9_]+`", kpi_section.group(2)):
                if kpi.strip("`") not in m.kpis:
                    m.kpis.append(kpi.strip("`"))

        results.append(m)
    return results


def check_kpis(modules: list[ModuleKPI]) -> list[Issue]:
    issues: list[Issue] = []

    for m in modules:
        is_core = m.file in CORE_MODULES

        # 1. 核心模块必须声明 KPI
        if is_core and not m.has_section:
            issues.append(Issue(m.file, "warning", "no_kpi_section",
                                "核心写操作模块未声明 KPI（参见 ADR-0011 §KPI 清单）"))
            continue

        if is_core and len(m.kpis) < 5:
            issues.append(Issue(m.file, "warning", "insufficient_kpi",
                                f"核心模块 KPI 数 {len(m.kpis)} < 5（建议每模块至少 5 个）"))

        # 2. KPI 命名合规
        for kpi in m.kpis:
            if not KPI_RE.match(kpi):
                issues.append(Issue(m.file, "warning", "kpi_naming",
                                    f"KPI 命名不符 wms_<module>_<entity>_<action>_<metric_type>: {kpi}"))
            else:
                # 检查后缀
                suffix_ok = any(kpi.endswith(s) for s in VALID_SUFFIXES if s)
                # 无后缀也合法（Gauge）
                if not suffix_ok and not (kpi.count("_") >= 3):
                    issues.append(Issue(m.file, "info", "kpi_suffix",
                                        f"KPI '{kpi}' 无 _total/_seconds/_bytes 后缀（视为 Gauge）"))

    return issues


def check_runtime_signals(repo_root: Path = REPO_ROOT) -> list[Issue]:
    issues: list[Issue] = []
    cargo = repo_root / "backend" / "Cargo.toml"
    runtime = repo_root / "backend" / "crates" / "api" / "src" / "bin" / "wms_api.rs"
    resilience = repo_root / "backend" / "crates" / "api" / "src" / "resilience.rs"
    backend_files = list((repo_root / "backend").rglob("*.rs")) if (repo_root / "backend").exists() else []
    cargo_text = cargo.read_text(encoding="utf-8") if cargo.exists() else ""
    runtime_text = runtime.read_text(encoding="utf-8") if runtime.exists() else ""
    resilience_text = resilience.read_text(encoding="utf-8") if resilience.exists() else ""
    backend_text = "\n".join(path.read_text(encoding="utf-8") for path in backend_files)

    if 'route("/metrics"' not in runtime_text or "# TYPE wms_" not in resilience_text:
        issues.append(Issue(
            "backend/crates/api",
            "warning",
            "metrics_endpoint_missing",
            "运行时未同时提供 /metrics 路由和 Prometheus 指标",
        ))
    if "tracing" not in cargo_text or "opentelemetry" not in cargo_text:
        issues.append(Issue(
            "backend/Cargo.toml",
            "warning",
            "otel_dependency_missing",
            "ADR-0011 要求的 tracing + OpenTelemetry 依赖未接入",
        ))
    if "tracing::instrument" not in backend_text:
        issues.append(Issue(
            "backend",
            "warning",
            "instrumentation_missing",
            "后端写操作尚无 tracing::instrument 运行时埋点",
        ))
    if "traceparent" not in backend_text:
        issues.append(Issue(
            "backend",
            "warning",
            "trace_context_missing",
            "后端尚未接入 W3C traceparent 上下文传播",
        ))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="核心模块 KPI 警告阻断")
    args = parser.parse_args(argv)

    modules = scan_kpis()
    issues = check_kpis(modules)
    runtime_issues = check_runtime_signals() if args.strict else []
    issues.extend(runtime_issues)

    errors = [i for i in issues if i.severity == "error"]
    warnings = [i for i in issues if i.severity == "warning"]
    infos = [i for i in issues if i.severity == "info"]

    total_kpis = sum(len(m.kpis) for m in modules)
    core_with_kpi = sum(1 for m in modules if m.file in CORE_MODULES and m.has_section)

    if args.json:
        print(json.dumps({
            "check": "check_observability",
            "tier": "T1",
            "category": "文档治理",
            "modules_total": len(modules),
            "core_modules_total": len(CORE_MODULES),
            "core_modules_with_kpi": core_with_kpi,
            "total_kpis_declared": total_kpis,
            "runtime_issue_count": len(runtime_issues),
            "errors": [asdict(i) for i in errors],
            "warnings": [asdict(i) for i in warnings],
            "infos": [asdict(i) for i in infos],
            "strict": args.strict,
            "ok": not errors and not (args.strict and warnings),
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_observability (T1, 文档治理) — {len(CORE_MODULES)} 核心模块 / "
              f"{core_with_kpi} 已声明 KPI / {total_kpis} 个 KPI")

        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                print(f"    ✘ [{i.file}] {i.rule}: {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项，T4 strict 出口前补全）：")
            for i in warnings:
                print(f"    ⚠ [{i.file}] {i.rule}: {i.message}")
        if infos:
            print(f"\n  信息（{len(infos)} 项）：")
            for i in infos:
                print(f"    ℹ [{i.file}] {i.rule}: {i.message}")
        if not (errors or warnings or infos):
            print("  ✓ 所有核心模块 KPI 声明合规")

    return 1 if errors or (args.strict and warnings) else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
