# US-H9-004 预览与浏览器打印验收记录

- 故事：`US-H9-004`
- 验收基线：US-H9-004 本地分组提交（后端、管理端、真实 E2E 与治理提交）
- 验收层级：`S2`
- 质量矩阵状态：`stories`
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 不适用
- 验收日期：`2026-07-26`
- 整体结论：`PASS`

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 PC 浏览器预览和打印 | V0 / V1 / V3 | M2 业务页读取 H9 预览，hiprint 调起浏览器打印；打印后由用户明确登记完成、取消或失败 | `H9TemplatePreviewDialog.tsx`；真实 E2E；三张截图 | PASS | 静默客户端不在第一阶段范围 |
| AC-2 业务数据、字段库与必填校验 | V1 / V2 / V3 | 由 M2 `/print-data` 读取真实 ASN/收货数据；预览绑定已发布字段库；缺失嵌套必填字段返回 `H9_TEMPLATE_FIELD_MISSING` | PostgreSQL 测试；M2 真实 E2E | PASS | - |
| AC-3 打印记录完整 | V1 / V2 / V3 | 回读模板版本、业务单据、货主、操作人、状态、失败原因和重试次数；失败无原因时受控拒绝 | `browser_print.rs`；真实 E2E 响应断言 | PASS | - |
| AC-4 补打复用原业务数据 | V2 / V3 | 同一真实 ASN 依次登记 `printed/cancelled/failed`，始终复用同一业务单据 ID 和 ASN 号，重试次数为 0/1/2 | `web-admin-m2-real.spec.ts` | PASS | - |
| AC-5 设备失败边界 | V0 | 第一阶段只记录浏览器打印结果；打印机离线、蓝牙和设备队列继续由 H5/后续 Print Agent 故事负责 | H9 用户故事；ADR-0036 | PASS | 本故事不以浏览器截图冒充真实硬件证据 |
| L4 错误与取消路径 | V1 / V2 / V3 | 嵌套必填字段缺失受控拒绝；失败必须填写原因；浏览器取消和失败分别落库 | PostgreSQL 测试；真实 E2E | PASS | - |
| L5 数据一致性与审计 | V2 | 每条记录引用已发布模板版本、同一业务单据和当前操作人；打印动作在同一事务追加 H2 审计 | PostgreSQL 测试；仓储实现 | PASS | - |
| L8 权限与货主隔离 | V2 | 既有 HTTP/PostgreSQL 测试验证无打印权限和跨货主访问受控拒绝 | `print_template_postgres` | PASS | - |
| L11 幂等 | V2 | 同一幂等键重放返回原打印记录；不同打印尝试产生递增重试次数 | `print_template_postgres` | PASS | - |
| UI-SEMANTICS 打印结果确认 | V3 | 中文展示“确认打印结果”，提供“已取消 / 打印失败 / 已完成打印”；关闭确认弹窗按取消记录 | 真实 E2E；确认截图 | PASS | - |
| BUSINESS-CONTENT 真实业务内容 | V2 / V3 | V3 复用 V2 创建的 ASN 业务键，预览显示同一 ASN 号，打印请求继续携带同一业务数据 | 真实 E2E；ASN/验收预览截图 | PASS | - |
| SECURITY 敏感字段脱敏 | V1 / V2 | 预览前按字段库元数据脱敏普通字段和 `lines[]` 嵌套字段 | `browser_print.rs` | PASS | - |

## 聚合验证

- V0：`python3 scripts/governance/check_quality_matrix.py --json`
- V0：`python3 scripts/governance/check_scope_gap_discovery.py --strict --module H9 --json`
- V0：`just openapi-check`
- V1 / V2：`cargo test --manifest-path backend/Cargo.toml -p wms-api --test print_field_library_postgres --test print_template_postgres -- --test-threads=1`
- V1：`node apps/web-admin/self-checks/h9-print-template-tree-self-check.mjs`
- V1：`node apps/web-admin/self-checks/m2-inbound-page-helpers-self-check.mjs`
- V1：`pnpm --dir apps/web-admin run build`
- V3：`just web-admin-m2-real-e2e`（创建并回收一次性 PostgreSQL，关闭 dev-mock）
- V3 截图：`artifacts/screenshot-portal/real-web/m2-inbound/receiving-print-preview.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m2-inbound/browser-print-result-confirmation.png`
- V3 截图：`artifacts/screenshot-portal/real-web/m2-inbound/inspection-print-preview.png`
- V4：不适用；真实打印机、蓝牙、Print Agent 和静默打印由 H5、US-H9-012～015 及 S4 门禁单独验收。

## 验收结论

- 已证明：五条 AC、L4/L5/L8/L11、真实 PostgreSQL、真实 ASN 数据、浏览器结果确认、三种结果记录、重试计数、审计、脱敏和三张人工复核截图均已闭环。
- 未完成：无 US-H9-004 软件故事内缺口。
- 范围声明：本记录不代表 H9 模块完成；US-H9-005～015 继续逐故事验收，浏览器截图不能抵扣真实打印硬件证据。
