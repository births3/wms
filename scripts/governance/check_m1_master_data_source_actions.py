#!/usr/bin/env python3
"""check_m1_master_data_source_actions.py — M1 档案来源列与新建/导入入口检查

类别：6. 前端治理
Tier：T1（< 10s，纯静态扫描）
输入：apps/web-admin M1 基础档案页面、列模型、查询层、dev mock
输出：人类可读 + --json
退出码：
  0  商品、供应商、客户来源列与新建/导入入口存在
  1  存在缺口
  2  脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path


THIS = Path(__file__).resolve()
REPO_ROOT = THIS.parent.parent.parent
WEB_ADMIN = REPO_ROOT / "apps" / "web-admin"
QUERY_TS = WEB_ADMIN / "src" / "features" / "master-data" / "master-data-queries.ts"
PAGE_TSX = WEB_ADMIN / "src" / "pages" / "master-data" / "M1MasterDataPage.tsx"
MODEL_TS = WEB_ADMIN / "src" / "pages" / "master-data" / "m1-product-page-model.ts"
ACTIONS_TSX = WEB_ADMIN / "src" / "pages" / "master-data" / "MasterDataSourceActions.tsx"
VITE_CONFIG = WEB_ADMIN / "vite.config.ts"
SELF_CHECK = WEB_ADMIN / "self-checks" / "m1-product-source-actions-self-check.mjs"
API_CLIENT_SCHEMA = REPO_ROOT / "packages" / "api-client" / "src" / "schema.ts"
DOMAIN_RS = REPO_ROOT / "backend" / "crates" / "domain" / "src" / "lib.rs"
HANDLERS_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "master_data_handlers.rs"
POSTGRES_RS = REPO_ROOT / "backend" / "crates" / "api" / "src" / "master_data_postgres.rs"
MIGRATION_SQL = REPO_ROOT / "backend" / "migrations" / "202607020002_master_data_source.sql"


@dataclass(frozen=True)
class TokenSpec:
    path: Path
    token: str
    message: str


@dataclass
class Issue:
    file: str
    message: str


TOKEN_SPECS = (
    TokenSpec(MODEL_TS, 'header: "来源"', "基础档案列模型缺少来源列"),
    TokenSpec(MODEL_TS, 'viewId === "m1-products"', "商品档案未接入来源列"),
    TokenSpec(MODEL_TS, 'viewId === "m1-suppliers"', "供应商档案未接入来源列"),
    TokenSpec(MODEL_TS, 'viewId === "m1-customers"', "客户档案未接入来源列"),
    TokenSpec(MODEL_TS, "masterDataActionLabels", "基础档案动作标签未统一建模"),
    TokenSpec(MODEL_TS, "新建供应商", "供应商档案缺少新建入口标签"),
    TokenSpec(MODEL_TS, "新建客户", "客户档案缺少新建入口标签"),
    TokenSpec(QUERY_TS, "export type CreateSupplierRequest", "查询层未导出供应商创建契约"),
    TokenSpec(QUERY_TS, "export type CreateCustomerRequest", "查询层未导出客户创建契约"),
    TokenSpec(QUERY_TS, 'api.POST("/api/v1/master-data/suppliers"', "供应商新建未复用后端接口"),
    TokenSpec(QUERY_TS, 'api.POST("/api/v1/master-data/customers"', "客户新建未复用后端接口"),
    TokenSpec(QUERY_TS, "sourceValue: supplierSource(item)", "供应商行缺少来源值映射"),
    TokenSpec(QUERY_TS, "sourceValue: customerSource(item)", "客户行缺少来源值映射"),
    TokenSpec(PAGE_TSX, "MasterDataSourceActions", "M1 页面缺少供应商/客户来源动作组件"),
    TokenSpec(PAGE_TSX, "createSupplierFromDialog", "供应商新建动作未接线"),
    TokenSpec(PAGE_TSX, "importSuppliersFromDialog", "供应商批量导入动作未接线"),
    TokenSpec(PAGE_TSX, "createCustomerFromDialog", "客户新建动作未接线"),
    TokenSpec(PAGE_TSX, "importCustomersFromDialog", "客户批量导入动作未接线"),
    TokenSpec(ACTIONS_TSX, "parseSupplierImportText", "供应商批量导入解析缺失"),
    TokenSpec(ACTIONS_TSX, "parseCustomerImportText", "客户批量导入解析缺失"),
    TokenSpec(ACTIONS_TSX, 'source: "manual"', "新建供应商/客户请求缺少手工新建来源"),
    TokenSpec(ACTIONS_TSX, 'source: "batch_import"', "批量导入供应商/客户请求缺少批量导入来源"),
    TokenSpec(ACTIONS_TSX, "supplier_code,supplier_name,license_no,contact_name", "供应商导入模板缺失"),
    TokenSpec(ACTIONS_TSX, "customer_code,customer_name,license_no", "客户导入模板缺失"),
    TokenSpec(VITE_CONFIG, 'pathname === "/api/v1/master-data/suppliers"', "dev mock 缺少供应商 API"),
    TokenSpec(VITE_CONFIG, 'pathname === "/api/v1/master-data/customers"', "dev mock 缺少客户 API"),
    TokenSpec(VITE_CONFIG, "devSupplierFromCreateRequest", "dev mock 缺少供应商创建"),
    TokenSpec(VITE_CONFIG, "devCustomerFromCreateRequest", "dev mock 缺少客户创建"),
    TokenSpec(DOMAIN_RS, "pub source: String", "后端供应商/客户响应契约缺少来源字段"),
    TokenSpec(DOMAIN_RS, "pub source: Option<String>", "后端供应商/客户创建契约缺少来源输入"),
    TokenSpec(HANDLERS_RS, "state.create_product", "商品新建未接入真实后端写路径"),
    TokenSpec(HANDLERS_RS, "state.create_supplier", "供应商新建未接入真实后端写路径"),
    TokenSpec(HANDLERS_RS, "state.create_customer", "客户新建未接入真实后端写路径"),
    TokenSpec(POSTGRES_RS, "pub async fn create_product", "PostgreSQL 商品新建写入缺失"),
    TokenSpec(POSTGRES_RS, "pub async fn create_supplier", "PostgreSQL 供应商新建写入缺失"),
    TokenSpec(POSTGRES_RS, "pub async fn create_customer", "PostgreSQL 客户新建写入缺失"),
    TokenSpec(POSTGRES_RS, "append_master_data_audit", "商品/供应商/客户新建缺少审计写入"),
    TokenSpec(MIGRATION_SQL, "ALTER TABLE products", "商品表缺少来源字段迁移"),
    TokenSpec(MIGRATION_SQL, "ALTER TABLE suppliers", "供应商表缺少来源字段迁移"),
    TokenSpec(MIGRATION_SQL, "ALTER TABLE customers", "客户表缺少来源字段迁移"),
    TokenSpec(API_CLIENT_SCHEMA, "source: string;", "api-client 供应商/客户 schema 缺少来源字段"),
    TokenSpec(API_CLIENT_SCHEMA, "source?: string | null;", "api-client 创建契约缺少可选来源字段"),
    TokenSpec(SELF_CHECK, "masterDataActionLabels", "self-check 未覆盖统一动作标签"),
    TokenSpec(SELF_CHECK, '"m1-suppliers"', "self-check 未覆盖供应商来源入口"),
    TokenSpec(SELF_CHECK, '"m1-customers"', "self-check 未覆盖客户来源入口"),
)


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def scan() -> list[Issue]:
    issues: list[Issue] = []
    cache: dict[Path, str] = {}
    for spec in TOKEN_SPECS:
        if not spec.path.exists():
            issues.append(Issue(rel(spec.path), "必需文件不存在"))
            continue
        text = cache.setdefault(spec.path, spec.path.read_text(encoding="utf-8"))
        if spec.token not in text:
            issues.append(Issue(rel(spec.path), spec.message))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    issues = scan()
    payload = {
        "check": "check_m1_master_data_source_actions",
        "tier": "T1",
        "category": "前端治理",
        "files": sorted({rel(spec.path) for spec in TOKEN_SPECS}),
        "issues": [asdict(issue) for issue in issues],
        "ok": not issues,
    }

    if args.json:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print("check_m1_master_data_source_actions (T1, 前端治理)")
        for file_name in payload["files"]:
            print(f"  · 检查文件: {file_name}")
        if issues:
            print(f"  ✘ {len(issues)} 处 M1 来源入口缺口:")
            for issue in issues:
                print(f"    - {issue.file}: {issue.message}")
        else:
            print("  ✓ M1 商品、供应商、客户来源列与新建/导入入口已登记")

    return 0 if not issues else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
