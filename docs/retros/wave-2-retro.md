# Wave 2 Retro：业务底座 + Schema 先行

日期：2026-06-03

## 结论

Wave 2 开发完成。静态完成项由 `just wave-2-complete-check` 验证，真实 dev/staging runtime evidence 因当前环境尚不稳定，作为预发布 gate 后移，不伪造证据。

2026-06-09 更新：W6.C 已补齐配置中心版 Feature Flag runtime evidence，写入 `docs/retros/wave-2-runtime-evidence.json`，staging 切源 / 对账 / smoke 通过 `just wave-2-runtime-evidence-validate`。

## 已完成

- W2.A：M1.a 商品 / 供应商 / 客户 / 仓库 / 库位 / 特殊药品分类 schema + 基础 CRUD
- W2.B：M2 收货单 schema + 基础 CRUD 骨架
- W2.C：M6 报表查询接口骨架
- W2.E：M-PM 参数对照服务，覆盖字典 / 规则 / 待映射队列 / 执行 API / 反向追溯
- W2.G static：M1-008 配置中心版 Feature Flag，覆盖迁移 / 导出 / 批量导入 / 对账 / 切换读取源 / 旧文件归档
- W2.G runtime：staging 配置中心切源 / 对账 / smoke evidence 已写入 `docs/retros/wave-2-runtime-evidence.json`
- OpenAPI：`shared/openapi/openapi.json` 与 `packages/api-client/src/schema.ts` 已同步 Wave 2 schema
- 治理：`check_openapi_contract.py` 覆盖 Wave 2 path/schema；`report_wave2_completion.py` 落地完成度检查

## 验证

- `cargo test -p wms-api --lib`：通过，26 tests
- `just openapi-sync`：通过
- `python3 scripts/governance/check_openapi_contract.py --json`：通过
- `python3 scripts/governance/report_wave2_completion.py --strict`：静态完成项通过；runtime evidence 标记为预发布 gate
- `just wave-2-runtime-evidence-validate`：通过

## 未完成但不阻断开发完成

- Wave 3 真 PDA / L7 evidence 仍由 W6.D 承接。
- Wave 4-6 的外部依赖、硬件、TMS 和灰度发布 evidence 仍由 W6.E-H 承接。

## 风险

- 当前 CRUD 服务为内存实现，证明 schema、owner scope、契约和行为骨架；正式持久化仍需后续 Wave 接入 PostgreSQL repository。
- M2 当前只落 schema 与基础 CRUD，不包含验收、质检、上架等业务规则；业务规则按 Wave 3 推进。
- 配置中心版 Feature Flag 的真实灰度链路已经在 staging 验证；后续若更换环境或重建配置中心数据，需要按 `docs/runbooks/wave-2-runtime-evidence.md` 重新采集。
