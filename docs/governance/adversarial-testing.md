# 功能模块对抗测试

> 事实源：`governance/adversarial-catalog.toml`。质量矩阵字段：`adversarial_checks`。检查器：`scripts/governance/check_quality_matrix.py --complete-module <模块>`。
> 不新增测试层。A1–A8 映射 ADR-0006 的 L4 / L6 / L8 / L11。

## 目标

每个功能模块的写路径都要能回答：合法身份用错误或恶意用法时，系统是否拒绝，以及库存、状态、审计是否保持不变。

T1 不把对抗覆盖当硬门禁。模块验收（`--complete-module`）按故事 `types` 推导攻击类，并要求 `adversarial_checks` 指向真实测试函数。

## 攻击类

| ID | 名称 | 层 | 通过标准 |
|---|---|---|---|
| A1 | 跨货主 IDOR | L8 | 403/404，被访问货主零变更、无审计泄漏 |
| A2 | 缺权限 / 未认证 | L8 | 401/403，不落业务行 |
| A3 | 非法状态跳转 | L4 | 业务错误，状态机原地 |
| A4 | 幂等重放与键冲突 | L11 | 同键同载荷不双写；同键不同载荷冲突 |
| A5 | 数量或标识篡改 | L4+L5 | 负数量、超计划、改锁定批被拒绝 |
| A6 | GSP / 受控配置绕过 | L4 | 过期批、温区错配、特药无双人、关闭强制规则被拒绝 |
| A7 | 并发双花 | L6 | 同库位/同批次并发最多一笔成功 |
| A8 | 伪造回执 / 离线重放 | L4 | 无有效派发的回执或过期包失败并审计 |

只读故事至少 A1+A2。`write` 加 A3+A4。库存变化加 A5+A6。`concurrent_resource` 加 A7。`pda_runtime` / `hardware_runtime` / `external_runtime` / `offline_sync` 加 A8。`integration` 只加跨货主与幂等，不加伪造回执。`api_change` 与纯前端交互不推导攻击类。

`adversarial_checks` 必须指向**本故事** `test_checks` / `evidence_refs` 里的测试函数，禁止把其他故事或模块的 `rejects_*` 拿来凑覆盖。

## 矩阵字段

写在用户故事上，格式与 `e2e_checks` 同级：

```toml
adversarial_checks = [
  { id = "A1", test = "backend/crates/api/tests/stock_adjustment_postgres.rs::cross_owner_cannot_read_stock_loss_order" },
]
```

`test` 必须是仓库内 `path.rs::fn_name`，且是带 `#[test]` / `#[tokio::test]` / `#[sqlx::test]` 的函数。T1 在字段已填写时还要求该文件属于本故事的 `evidence_refs` 或 `test_checks`（含 `--test <crate>` 目录）；缺字段不失败。

## 测试夹具

跨货主 / 权限夹具放在既有目录 `backend/crates/api/tests/support/adversarial.rs`（不另建 `tests/common`）。集成测试用：

```rust
#[path = "support/adversarial.rs"]
mod adversarial_support;
```

## 模块覆盖策略

全部活跃模块共用同一目录，不按波次另起套件：

1. 已进入 `stories` 的写故事：模块验收前必须登记所需 A 类。
2. 仍在 `deferred_stories` 的模块：目录同样适用，但延期本身已阻断 `--complete-module`。
3. 优先复用现有 `rejects_*` / `cross_owner_*` / `idempotenc*` / `forbidden` 测试，再按缺口补 postgres 测试。
4. Playwright 截图不是对抗证据。

当前盘点结论（只登记本故事证据文件中的测试；缺类仍等模块验收补）：

| 模块 | 已诚实登记 | 主要缺口 |
|---|---|---|
| M1 | 字典跨货主/权限/幂等，LPN 状态与并发，设备注册幂等与 PTL 互斥 | 仓库库位写路径、设备越权 |
| M2 | ASN 跨货主/资质/幂等、收货缺权限/跨货主/草稿收货、验收超量与双签绕过、上架跨货主/数量/状态/重放 | 故事仍延期；A8 PDA/外部回执、看板页不做 |
| M4 / M-SA | 测试文件里已有跨货主与拒绝路径 | 故事仍延期，未进矩阵验收 |
| M3 | 库存查询/移库/ABC/预警/库位历史权限与货主隔离 | 移库并发与 GSP、ABC 跨货主 |
| H1 | 登录跨货主/错密/锁定、会话隔离、角色权限 | 提权 |
| H2 | 审计 append-only 触发器与货主隔离、归档幂等 | 导出越权 HTTP |
| H3 | 限流身份隔离、重放 429、熔断 | 缺权限类 A2 |
| H6 | 目录无货主私有机、权限、非法跳转、出库切片 | 无 |
| H8 / H9 / AL / TE / DI / RC / DOCK | 仅保留本故事证据中的 A 类 | 其余攻击类未覆盖 |

## 验证

```bash
python3 scripts/governance/tests/test_adversarial_catalog.py
python3 scripts/governance/check_quality_matrix.py --complete-module H6
just gov-t1
```
