# API 列表分页统一规范

> 状态：草案（2026-08-10）· 适用范围：WMS 后端所有对外列表接口
> 触发背景：M1 商品档案等主数据列表接口无分页全量返回，数据量大时查询慢。

## 0. 现状问题面（2026-08-10 全量扫描）

后端 220 处 `fetch_all` 中，**22 处为缺分页的对外列表接口**（A 类）；其余为内部用途（job 批处理/详情小集/字典小表/ID 集合/聚合统计/测试），全量合理（B 类）。已有分页的参考实现（C 类）：audit `list_events`（keyset 游标）、h8 消息 list（keyset）、wave4 出库订单（LIMIT 参数化）、wave3 各固定上限型（LIMIT 100-500）。

**A 类清单与优先级**：

| 优先级 | 接口 | 路由 | 规模特征 |
|---|---|---|---|
| P0 | `list_inventory_batches_with_query` | GET /api/v1/inventory/batches | 最大表（批次×库位），15 过滤条件无分页 |
| P0 | `list_receiving_orders` | GET /api/v1/inbound/receiving-orders | 流水 + 每单 N+1 拉明细 |
| P0 | `list_inbound_documents` | GET /api/v1/drug-inspection/inbound-documents | ASN×SKU 行放大 |
| P0 | `list_role_users` / `list_worker_candidates` | /api/v1/auth/roles/users、/task-engine/workers | 员工规模全量聚合 |
| P0 | `list_print_suite_instances` | /api/v1/print-orchestration/suite-instances | 每次截单生成，长期流水 |
| P1 | `list_products` 等 M1 主数据 9 项 | /api/v1/master-data/* | 主数据（M1 已落地 offset 分页） |
| P1 | M3 养护任务/记录、M2 药检列表、H1 api-keys、H9 租约/线路绑定、M4 温控队列、dock 预约 | 见各处 | 中低规模 |

改造顺序建议：P0 五组先行（游标模式），P1 按模块滚动（M1 已开始）。

## 1. 适用范围与强制要求

以下接口**必须**实现分页：

- 列表类 GET 接口（`list_*` / `query_*` / `search_*` handler），返回数据随业务增长的表：
  - 流水/日志/消息类（入库/出库/盘点/告警/审计/ERP 消息/打印任务等）——**必须**，数据无限增长
  - 主数据类（商品/供应商/客户/仓库/库区/库位/用户/角色/字典）——**必须**，数据可达数千行
- 例外（**允许**全量返回，需在 handler 注释说明理由）：
  - 选项下拉（`options`，通常 <200 行）
  - ID 集合/关联小集（JOIN 取少量）
  - 常量/配置表（行数有界）
- 禁止：`fetch_all` 直接返回给前端的列表 handler（评审 gate）。

## 2. 分页模式选择

| 模式 | 适用 | 既有参考实现 |
|---|---|---|
| **offset 分页**（`page`/`page_size` + `total`） | **默认统一模式**：主数据与流水列表接口一律采用；数据量 <10 万行、页面跳转是刚需 | M1 列表（`/api/v1/master-data/products` 等，2026-08 落地中） |
| **游标分页**（`limit` + `cursor`，keyset） | 可选优化：>10 万行且持续增长的流水（深翻页性能），需前端"加载更多"适配 | `h8_erp_messages`、audit `list_events`（既有实现） |

选择原则（2026-08-10 决议）：**2026-08 批次改造全部采用 offset 统一**（前端 DataGrid 一个 serverPagination 模式，改造成本可控）；游标作为大表后续优化项，前端另行适配。

## 3. offset 分页契约（主数据类）

请求参数：

```
GET /api/v1/master-data/products?page=1&page_size=20
page      u32，从 1 起，默认 1
page_size u32，默认 20，上限 200（越界返回 400 或钳制到上限，按接口约定）
```

响应（复用 `PageMeta`，可选 `total` 字段向后兼容）：

```json
{
  "data": [ ... 本页数据 ... ],
  "page": { "next_cursor": null, "count": 20, "total": 1320 }
}
```

约束：

- SQL：`LIMIT $page_size OFFSET ($page - 1) * $page_size` + `SELECT count(*)`（同事务或紧随其后）。
- 排序必须稳定：`ORDER BY <键> DESC, <唯一列>`；`(owner_id, 排序键)` 组合索引（迁移文件加 `idx_<表>_owner_<排序键>`）。
- `total` 为 `Option<u32>`（`#[serde(default)]`），旧客户端兼容。

## 4. 游标分页契约（流水/日志类）

请求参数：

```
GET /api/v1/integration/erp-messages?limit=50&cursor=<opaque>
limit  u32，默认 50，上限 200（校验 1..=200）
cursor 不透明字符串（base64 编码的 keyset 值），首屏不传
```

响应（`PageMeta.next_cursor`）：

```json
{
  "data": [ ... ],
  "page": { "next_cursor": "eyJjcmVhdGVkX2F0Ijoi...", "count": 50, "total": null }
}
```

约束：

- 游标编码 keyset 列值（`(created_at, id)` 等），排序键唯一。
- 无 `total`（游标模式不保证总数一致性；需要总量走独立 count 接口）。
- 游标解码失败返回 400；游标过期（数据被清理）返回空列表而非错误。

## 5. 公共约束（两种模式通用）

1. **参数校验**：越界返回 400（`BadRequest`），错误信息明确（如 `limit must be 1..=200`）。
2. **默认值**：offset 默认 `page=1&page_size=20`；游标默认 `limit=50`。
3. **响应结构**：统一 `{ data, page: PageMeta }`；`PageMeta`（`docs/domain`）为唯一分页元数据载体。
4. **索引**：列表查询的过滤+排序路径必须有组合索引（迁移文件命名 `2026xxxxxx_<模块>_list_query_indexes.sql`）。
5. **前端适配**：DataGrid 提供 `serverPagination` 受控模式（offset 用）；流水类页面用"加载更多/游标"（H8 消息页既有实现）。
6. **评审 gate**：新增/修改列表接口时检查：是否有分页、模式选择是否合理、索引是否存在、`fetch_all` 是否有豁免注释。

## 6. 验收清单（新列表接口）

- [ ] 确定数据特征（主数据/流水）并选择对应模式
- [ ] 参数校验（默认值/上限/错误语义）
- [ ] SQL 含 LIMIT（或游标）+ 稳定排序
- [ ] 组合索引迁移
- [ ] openapi 契约（查询参数 + PageMeta.total 若用 offset）
- [ ] 前端适配（DataGrid serverPagination 或游标加载）
- [ ] e2e 覆盖（分页边界：第 1 页/最后一页/越界参数）
