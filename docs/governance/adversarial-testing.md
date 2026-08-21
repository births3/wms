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

只读故事至少 A1+A2。`write` 加 A3+A4。库存变化加 A5+A6。`concurrent_resource` 加 A7。外部 / PDA / 硬件加 A8。`api_change` 与纯前端交互不推导攻击类。

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

当前盘点结论（按测试函数名，不是门禁）：

| 模块 | 已有对抗切片 | 主要缺口 |
|---|---|---|
| M1 | 跨货主字典/档案、LPN 并发、设备 PTL 互斥、WCS 回执幂等 | 部分写故事未登记 `adversarial_checks` |
| M2 | 跨货主上架、非法数量、过期资质 ASN | 多数故事仍延期 |
| M3 | 跨货主养护/盘点、过期不合格批、移库数量 | 盘点/移库并发与越权需对齐矩阵 |
| M4 | 非法状态再校验、销退幂等 | 超卖、过期出库、跨货主读单 |
| M-SA | 跨货主读报损、写权限+幂等、双人 | 超库存报损 |
| M-RC | 原子调账、幂等 | 跨货主差异处理 |
| H1 | AuthContext 跨货主、路由未认证 | 角色提权路径 |
| H2 | 跨货主导出、哈希篡改 | 审计 UPDATE/DELETE 拒绝需持续保留 |
| H8 | 连接器回执拒绝他方 Key、并发同一出库单 | 接口表伪造 |
| H9 | 跨货主模板/设备、截单幂等与并发 | 打印任务伪造 |
| AL | GSP 强制告警不可删、跨货主定义 | 升级规则越权 |
| TE | 资格状态机+幂等、过期资格 | 跨货主领任务 |
| DI | 不合格验收阻断 | 跨货主药检副本 |
| 其余延期模块 | 零星 `rejects_*` | 按故事迁入 `stories` 时补登记与测试 |

## 验证

```bash
python3 scripts/governance/tests/test_adversarial_catalog.py
python3 scripts/governance/check_quality_matrix.py --complete-module H6
just gov-t1
```
