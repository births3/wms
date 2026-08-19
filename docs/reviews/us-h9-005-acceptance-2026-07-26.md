# US-H9-005 业务模块接入规则验收记录

- 故事：`US-H9-005`
- 验收基线：US-H9-005 当前工作区（后端解析、六类业务入口、真实 E2E 与治理登记）
- 验收层级：`S2`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用
- 验收日期：`2026-07-26`
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 显式、货主默认、全局默认解析顺序 | V1 / V2 | 真实 PostgreSQL 分别创建显式模板、货主默认、全局默认和其他货主模板，断言优先级、停用拒绝和禁止跨货主回退 | `business_integration.rs` | PASS | - |
| AC-2 首批六类业务接入 | V1 / V2 / V3 | 六类模板均走同一 resolve → preview → print 契约；关闭 dev-mock 后从真实业务页面读取并打印同一业务键 | PostgreSQL 六类契约测试；M1/M2/M3/M4 真实 E2E；六张截图 | PASS | - |
| AC-3 后续能力仅接入 H9 契约 | V0 | H5 快递面单、M6 法定台账、后端 PDF/Word 和静默客户端保留为后续接入方，本故事未加入局部模板选择实现 | 用户故事边界；`H9BusinessPrintDialog.tsx` | PASS | 深度实现由对应后续故事验收 |
| AC-4 四类统一错误处理 | V1 / V2 / V3 | 后端分别返回不存在、停用、字段库未发布和字段库绑定不匹配错误；共享前端入口映射为中文修复提示 | PostgreSQL/HTTP 测试；共享业务打印弹窗；self-check | PASS | - |
| AC-5 四个权限码 | V0 / V1 / V2 | 读取、维护、发布、打印权限独立登记；handler 按动作校验，业务页面打印按钮要求打印权限 | 权限迁移；handler；PostgreSQL 权限测试；真实页面 | PASS | - |
| AC-6 新场景补 L2/L3 | V1 / V2 / V3 | 六类场景由一个 L2 HTTP 契约测试逐类执行，并由 M1/M2/M3/M4 Playwright 业务流程测试验证 | `six_business_template_types_resolve_preview_and_record_through_one_contract`；四个 real spec | PASS | - |
| L4 错误路径 | V1 / V2 | 显式停用模板不回退；无默认模板、字段库未发布、字段库不匹配返回受控错误码 | `print_template_postgres` | PASS | - |
| L5 数据一致与审计 | V2 / V3 | 预览、页面字段、打印请求和打印记录复用同一业务键与模板版本；打印动作追加 H2 审计 | PostgreSQL 测试；真实 E2E 请求断言 | PASS | - |
| L8 权限与货主隔离 | V1 / V2 / V3 | 其他货主显式模板不可解析；业务页面先受模块读取权限约束，打印动作再要求 H9 打印权限 | PostgreSQL 测试；权限迁移；真实登录 E2E | PASS | - |
| L11 幂等 | V2 | 打印记录使用 `Idempotency-Key`，重复请求不重复产生业务单号或打印记录 | `browser_print.rs`；真实 E2E | PASS | - |
| UI-SEMANTICS 中文和共用控件 | V3 | 六类业务页面均显示中文打印动作、模板标题、字段名称、纸张方向和结果确认 | 六张人工复核截图 | PASS | 协议缩写 ASN、LPN 保留 |
| BUSINESS-CONTENT 真实业务内容 | V2 / V3 | M1 使用 `P-M1-E2E-001`、`A01-01-02-03`，M3 使用 `LPN-E2E-001`，M4 使用本次创建的出库单号，M2 使用本次创建的 ASN 号；API、预览和截图逐项一致 | 确定性种子/本次业务键；Playwright 字段值断言；六张截图 | PASS | - |

## 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`
- V0：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9 --json`
- V0：`just openapi-check`
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test print_template_postgres -- --test-threads=1`
- V1：`node apps/web-admin/self-checks/h9-print-template-tree-self-check.mjs`
- V1：`pnpm --dir apps/web-admin run build`
- V3：`just web-admin-m1-real-e2e`
- V3：`just web-admin-m2-real-e2e`
- V3：`pnpm --dir apps/web-admin run test:e2e:m3-real`
- V3：`pnpm --dir apps/web-admin run test:e2e:m4-real`
- V3 截图：`artifacts/screenshot-portal/real-web/m2-inbound/receiving-print-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m2-inbound/inspection-print-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m4-outbound/delivery-note-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m1-locations/location-label-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m3-batches/lpn-label-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m1-products/product-label-preview.png`
- V4：不适用；本故事验证浏览器软件链路，不以截图替代真实打印机、Print Agent、PDF/Word 或发布环境证据。

## 验收结论

- 已证明：六条 AC、六类业务入口、统一解析优先级、四类错误、权限、货主隔离、真实 PostgreSQL、关闭 dev-mock 的业务 E2E、同一业务键和六张人工复核截图均已闭环。
- 未完成：无 US-H9-005 软件故事内缺口。
- 范围声明：本记录不代表 H9 模块完成；US-H9-006～015 继续逐故事验收，硬件与发布环境按 V4/S4 分层独立收口。
