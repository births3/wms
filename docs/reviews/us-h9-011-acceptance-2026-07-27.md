# US-H9-011 验收记录

- 故事：`US-H9-011 打印机、纸盒与设备租约`
- 验收基线：2026-07-27 当前工作区（物理打印站点、货主仓映射、打印机/纸盒维护、测试打印记录、设备租约与人工释放）
- 验收层级：`S4`（本记录只覆盖软件层 V0-V3；真实物理打印机/USB/Print Agent 证据待 S4 硬件验收）
- 质量矩阵状态：`deferred_stories`（软件切片已有证据；S4 硬件与跨故事运行时未完成）
- 证据层覆盖：`V0 / V1 / V2 / V3`；`V4` 与真实硬件证据本机不可达，如实登记缺口
- 验收日期：2026-07-27
- 整体结论：`NEEDS_WORK`；软件 V0-V3 已通过，S4 硬件与 AC-4/AC-5 运行时闭环待后续故事

| AC | 证据层 | 验证命令或方式 | 证据 | 结果 | 缺口 / 恢复条件 |
|---|---|---|---|---|---|
| AC-1 打印机归属唯一站点、货主仓显式映射、禁止跨站点引用 | V1 / V2 / V3 | `h9_printers.site_id` NOT NULL + `(site_id, id)` 复合唯一锚点；纸盒/租约用复合外键指向 `h9_printers(site_id, id)`，跨站点直插被 FK 拒绝；`h9_print_site_owner_mappings` 部分唯一索引限定活动映射，停用为软删可重建 | `202607270001_h9_print_devices.sql`；`site_scoped_devices_enforce_boundaries_and_capabilities`（跨站点纸盒/租约 SQL 直插均 FK 报错）；页面站点页签映射子表 | PASS | - |
| AC-2 打印机多纸盒；纸盒维护纸张能力、启用状态、设备标识 | V1 / V2 / V3 | `h9_printer_trays`（tray_code/paper_size/paper_type/enabled）；同打印机 tray_code 唯一；PATCH 维护能力与启停 | 同上测试（TRAY 冲突、能力更新、启停）；e2e 纸盒页签建 TRAY-A（A5/不干胶标签纸）截图 `trays.png` | PASS | - |
| AC-3 维护页对指定打印机+纸盒测试打印，结果落表 + H2 审计 | V1 / V2 / V3 | POST `/printers/{id}/test-print`：校验打印机启用、纸盒启用；落 `h9_printer_test_prints`（result=dispatched + 回执字段 result/result_note/result_at）；写 H2 `test_print_printer` | 测试打印落表断言；停用纸盒/打印机 409；e2e `test-print.png` | PASS（受控） | 本机无真实物理打印机：只登记"已下发测试指令"，真实回执由 Print Agent（US-H9-012）或 S4 硬件验收登记，不伪造成功结果 |
| AC-4 模板/打印项纸张要求 → 输出槽映射兼容主/备设备 | V1（部分） | 本故事只落纸盒纸张能力字段（paper_size/paper_type），供 010/012 的输出槽匹配消费 | 表结构与域类型 | 部分 | 输出槽映射、备用设备与"未配置备用默认暂停告警"属 US-H9-010/012 范围，此处仅提供纸张能力数据地基 |
| AC-5 网络打印机同一时点仅一个 Agent 持租约；USB 单机 | V1 / V2 | `h9_device_leases_one_active_uidx`（printer_id, WHERE status='active'）部分唯一索引；`connection_type='usb'` 字段化标注单机语义（迁移注释 + 页面/弹窗文案）；`holder_agent_id` 012 前允许 NULL 占位 | `lease_uniqueness_and_release_mode_snapshot`（第二条活动租约被唯一索引拒绝）；页面 USB（单机）标识 | PASS（软件） | Agent 真实持有/续租在 US-H9-012；真实 USB 单机行为待 S4 |
| AC-6 释放模式全局默认（参数）+ 打印机覆盖 + 租约快照 | V1 / V2 / V3 | 全局默认复用受控系统字典 `h9_device_lease_release/default`（manual_only）；`h9_printers.release_mode_override` 单机覆盖（inherit 清除）；`h9_device_leases.release_mode` 创建时快照，覆盖变更不回写 | `resolve_lease_release_mode` 覆盖优先断言；快照不可改写断言；e2e 释放模式覆盖弹窗（全局默认→单机覆盖 safe_auto）`printers.png` | PASS | - |
| AC-7 人工释放：专用权限+原因+二次确认；printing/result_unknown/未决对账硬拒绝 | V1 / V2 / V3 | 专用权限 `h9.device_lease.release`（handler + service 双层校验，不含在 write 权限内）；`confirm=true` 必填、原因必填≤500；busy_state ∈ printing/result_unknown/reconciling 时任何人 409 拒绝（SQL 播种状态验证）；人工权限只覆盖 manual_only 模式本身 | `manual_release_enforces_permission_reason_confirm_and_hard_safety`（缺权 403 / 未确认 422 / 空原因 422 / 三种硬安全状态 409 / 空闲释放成功 / 幂等重放 / 重复释放 409）；e2e 释放弹窗原因+勾选二次确认 `leases-released.png` | PASS | busy_state 的真实来源在 US-H9-010/012；本故事以测试播种状态验证拒绝路径 |
| L4 错误路径 | V1 / V2 | 站点编码冲突 409、映射冲突/已停用 409、打印机重名 409、纸盒冲突 409、纸盒/打印机停用 409、NotFound 404、缺幂等键 400、幂等键复用冲突 409 | `h9_print_device_postgres.rs`（5/5）；handler 错误码表 | PASS | - |
| L8 权限 | V1 / V3 | 端点要求 `h9.print_device.read/write`；释放要求专用 `h9.device_lease.release`；共享站点读写按活动映射 owner 并集逐一校验，审计按 owner 分别写入；物理站点级测试打印事实不伪造单一 `owner_id`；迁移播种权限并挂角色（release 仅 system_admin）；菜单按钮权限 create_site/map_owner/create_printer/test_print/release_lease | `site_resources_reject_cross_owner_reads_and_mutations`；`shared_site_requires_owner_union_and_audits_each_owner`（打印机、纸盒、测试打印均逐 owner 审计，测试打印表无任意 owner 归属）；页面 `canWrite`/`canRelease` 门控 | PASS | - |
| 幂等 + H2 审计 | V1 / V2 | 全部写端点 Idempotency-Key 必填、同键重放返回原结果、变载荷 409；create/map/disable/update/test/release 全动作写 H2（module=H9） | 幂等重放断言；审计计数断言 | PASS | - |
| UI-SEMANTICS | V3 | 站点/打印机/纸盒/租约四页签、中文状态徽章、测试打印与释放租约弹窗（原因+确认勾选）、硬安全与 USB 单机文案 | `web-admin-h9-print-device-real.spec.ts`；`h9-print-device-slice-self-check.mjs` | PASS | - |
| BUSINESS-CONTENT | V2 / V3 | 种子真实业务键 `SITE-H9-E2E`、`E2E 东区网络打印机`、`TRAY-1`、`LEASE-H9-E2E-001`（manual_only/idle/active），API、页面与截图同键 | e2e 断言与五张截图 | PASS | - |

## 聚合验证

- V0：`node apps/web-admin/self-checks/h9-print-device-slice-self-check.mjs`（通过，已加入 `test:self-checks` 链）
- V0：`cargo test -p wms-domain print_device`（3/3 纯域校验：站点/打印机/释放确认）
- V1 / V2：`DATABASE_URL=... cargo test --manifest-path backend/Cargo.toml -p wms-api --test h9_print_device_postgres`（5/5：站点边界与能力、跨 owner 拒绝、owner 并集鉴权与逐 owner 审计、租约唯一与快照、人工释放权限/确认/硬安全/幂等/审计）
- OpenAPI：10 个 print-devices 端点登记 `openapi_paths/print_device.rs` + `openapi_doc.rs` + `openapi_tests.rs` 必含清单（`cargo test -p wms-api --lib openapi` 6/6）；`just openapi-sync` 已再生成 `shared/openapi/openapi.json` 与 `@wms/api-client`
- 前端：`pnpm --dir apps/web-admin run test:self-checks` 全链通过；`pnpm --dir apps/web-admin run build` 通过
- V3：一次性 PostgreSQL + `WMS_WEB_ADMIN_DEV_MOCK=0`，
  `pnpm --dir prototypes exec playwright test --config=playwright-web-admin-m1-real-config.ts e2e/web-admin-h9-print-device-real.spec.ts`
  （2026-07-28 定向复验 1/1，通过并重新生成五张截图）；随后 `just web-admin-m1-real-e2e`
  全套 m1/h9 回归 12/12 通过
- 截图（artifacts/screenshot-portal/real-web/h9-print-devices/）：
  - `sites-and-mappings.png`（站点 + 货主仓映射）
  - `printers.png`（打印机与释放模式单机覆盖）
  - `trays.png`（纸盒能力）
  - `test-print.png`（测试打印下发）
  - `leases-released.png`(租约人工释放：原因 + 二次确认)

## 缺口清单（如实登记）

1. **真实物理打印机 / 真实打印产物**：本机无任何物理打印机，测试打印仅登记"已下发测试指令"
   （`result=dispatched`）；成功/失败回执字段已预留，待 US-H9-012 Print Agent 或 S4 硬件验收
   登记，浏览器截图不抵扣 S4 证据。
2. **USB 单机租约真实行为**：USB 语义以 `connection_type='usb'` 字段化标注并在页面/弹窗
   明示"单机"，真实"仅本机 Agent 可持有"的运行时强制在 US-H9-012 实现，硬件证据待 S4。
3. **租约真实签发与 busy_state 真实来源**：`holder_agent_id` 允许 NULL 占位、busy_state
   （printing/result_unknown/reconciling）由测试/种子 SQL 播种驱动校验；真实来源在
   US-H9-010（打印执行）与 US-H9-012（Agent/对账）。
4. **AC-4 输出槽映射**：本故事只交付纸盒纸张能力数据；输出槽 → 主/备设备映射与
   "未配置备用默认暂停并告警"在 US-H9-010/012 收口。
5. **Print Agent 管理动作的多货主授权**：本故事已完成站点、打印机、纸盒、测试打印与租约
   释放的活动映射 owner 并集逐一鉴权和逐 owner H2 审计；Agent 激活、轮换、pilot 与全局
   版本动作仍属 US-H9-012。

## 验收结论

- 已证明：七条 AC 的软件面（站点边界、映射软删、纸盒能力、受控测试打印落表、租约唯一、
  释放模式全局默认/单机覆盖/快照、人工释放专用权限+原因+二次确认+硬安全拒绝）、幂等重放、
  H2 审计、OpenAPI 契约、真实菜单入口与中文业务内容截图闭环。
- 未完成：上述缺口清单 1-4，以及缺口 5 所列 US-H9-012 Agent 管理动作；因此本故事在质量
  矩阵继续延期，不能以软件切片通过替代完整故事验收。
- 范围声明：本记录不代表 H9 套打中心全部完成；US-H9-009～010、012～015
  继续逐故事验收，真实 Windows Agent、打印机和纸盒硬件证据不得由本故事截图抵扣。
