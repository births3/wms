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

`test` 必须是仓库内 `path.rs::fn_name`，函数名要能在该文件搜到。T1 只在字段已填写时校验路径；缺字段不失败。

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
| M2 / M4 / M-SA | 测试文件里已有跨货主与拒绝路径 | 故事仍延期，未进矩阵验收 |
| M3 | 库存查询跨货主、养护/盘点幂等、移库数量 | 多数故事缺 A2/A3/A6/A7 |
| H1 | 会话货主隔离与登出幂等、API Key 幂等、角色创建货主隔离 | 缺权限写、提权 |
| H2 | 未强行借用他模块测试 | 跨货主导出、审计篡改需按故事补登记 |
| H6 | 出库状态切片幂等 | 缺 A1/A3 |
| H8 / H9 / AL / TE / DI / RC / DOCK | 仅保留本故事证据中的 A 类 | 其余攻击类未覆盖 |

## 验证

```bash
python3 scripts/governance/tests/test_adversarial_catalog.py
python3 scripts/governance/check_quality_matrix.py --complete-module H6
just gov-t1
```
