# ADR-0047：容器整托不变量、类型策略与强制取号

- 状态：Accepted
- 日期：2026-08-15
- 决策者：项目主人
- 关联：US-M1-004a、US-M2-005、ADR-0038、M-CG

## 背景

v1 要做整托作业，PDA 只负责扫码。第一刀已有 `lpn_containers` 主档，但创建仍手填码、同托混品无策略、上架未回写库存 `container_lpn`。需要把作业不变量和取号权收口，避免再出现「字符串当容器」。

## 决策

1. **对象是容器，码是 LPN。** 与 glossary #53/#54/#72 一致。整托 = 以整个容器为一次作业对象，同一时刻只在一个库位。
2. **上架可以不填 LPN**（散件）。填了则必须已在主档；`idle`/`in_use` 可用；`in_use` 仅允许同一库位加货；跨库位或在途/回收/已出库走 `M2_LPN_BINDNG_FAILED`，找不到才 `M2_LPN_NOT_FOUND`。
3. **同库位再上同一 LPN 永远允许加量**；并发用行锁串行后都成功。
4. **混批、混品按容器类型配置，默认都关。** 策略表按货主+类型存两布尔。上架用该 LPN 已有 `inventory_batches.container_lpn` 判断。容器物理上可装多品，默认策略关掉。
5. **创建只在 PC。** 客户端不传 `lpn_code`；服务端按类型走 M-CG（五种类型五条规则）。规则或字典缺失则失败，禁止手填回退。
6. **上架成功必须回写批次 `container_lpn`。** 否则类型策略无法看见托上已有商品/批号。
7. **本切片不做 PDA、不做容器内容物表、不做嵌套/回收状态机。**

## 后果

- 未配编号规则的货主无法建容器，上架扫 LPN 会失败，必须先配 M-CG。
- 默认一托一品一批，运营要按类型打开混装。
- 库存 `container_lpn` 成为整托识别来源，与主档必须同事务更新。

## 测试证据

- L11：`create_replays_same_idempotency_key_without_second_row`（同一 `Idempotency-Key` 重放，只 1 行）。
- L6：`concurrent_same_sku_batch_putaway_adds_qty`（同 LPN/库位/SKU/批号并发加数量，在手合计正确）。
- 散件/整托互斥：`m1_lpn_container_identity_postgres`（散件后再挂 LPN、LPN 后再上散件均拒绝且行数回滚）；`putaway_rejects_second_lpn_on_same_sku_batch_location`（第二托不得覆盖第一托，第二托保持 `idle`）。
- 质量矩阵类型含 `concurrent_resource`，由脚本推导覆盖 L6。

## 稀疏仓库接线

下列父文件整份未跟踪，禁止整文件提交。LPN 只抽到专用文件；工作区父文件改完后保持本地：

| 父文件（勿整份提交） | LPN 接线 |
|---|---|
| `apps/web-admin/src/App.tsx` | 必须保留字面量 `{ id: "m1-lpn-containers", title: "M1 容器管理", subtitle: "LPN / 类型策略", icon: PackageCheck }`。导航扫描只认 `id`/`title` 字符串，禁止改成展开 `lpnContainerMenuItem`。 |
| `apps/web-admin/src/app-shell/admin-view.ts` | 联合类型字面量 `"m1-lpn-containers"`，不要写成 `typeof LPN_CONTAINER_VIEW_ID`。 |
| `apps/web-admin/src/app-shell/AdminViewRenderer.tsx` | `if (view === "m1-lpn-containers") return <M1LpnContainerPage />` |
| `apps/web-admin/dev-mocks/admin-menu-dev-mock.ts` | `m1-lpn-containers` |
| `backend/crates/domain/src/lib.rs` | `mod lpn_container; pub use lpn_container::*;` |
| `backend/crates/api/src/lib.rs` | `pub mod lpn_container_handlers;` / `lpn_container_repository` |
| `backend/crates/api/src/openapi_doc.rs` | 五个 LPN path 操作 |
| `backend/crates/api/examples/wms_api_e2e.rs` | `lpn_container_router` + `wms_api_e2e_seed_lpn` |
| `backend/crates/api/examples/support/wms_api_e2e_seed_data.rs` | `seed_lpn_putaway_order` |
| `backend/crates/api/src/wave3_repository_part1b.rs` | 上架事务在 `bind_lpn_for_putaway` 之前调用 `enforce_inventory_identity`；整文件勿提交 |
| `docs/error-codes.md` | `M1_LPN_*` / `M2_LPN_*` |
| `docs/glossary.md` | #53 容器、#54 LPN、#72 整托 |
| `governance/quality-matrix.toml` | `[[stories]] US-M1-004a`，含 L6 |

已入库的专用文件：`lpn-container-nav.ts`、`wms_api_e2e_seed_lpn.rs`、本 ADR、handler/repository/openapi path、页面、迁移、postgres 测试、Playwright spec。

## 参考

- `docs/glossary.md` #53 容器、#54 LPN、#72 整托
- `docs/domain/user-stories-m1-master-data-warehouse.md` US-M1-004a
