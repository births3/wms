# REDIS-01：Redis 必要性评估任务与验收标准（2026-07-31）

> 状态：事实盘点完成；选型结论待性能/安全证据确认。本任务只做事实盘点、替代方案评估和基础设施决策，不先引入、删除或替换 Redis。
> 结论必须由项目主人结合性能、可用性、安全和运维证据确认。

## 1. 目标与边界

回答一个问题：当前 WMS 是否需要 Redis，还是 PostgreSQL、进程内能力和现有 outbox 已足够。
评估结果必须按用途给出 `必须保留 / PostgreSQL 可替代 / 可选优化 / 暂缓`，不能用“项目用了缓存”笼统结论。

本任务覆盖鉴权撤销、双人策略缓存、系统字典缓存、幂等、限流、熔断、通知、部署和依赖；不直接修改
Accepted ADR、生产配置、数据库表或运行时依赖。

## 2. 已知 Redis 使用面

首轮盘点必须核对源码、Cargo 依赖、Compose、ADR、runbook 和实际运行配置，至少覆盖：

| 使用面 | 当前证据 | PostgreSQL 候选 | 需要确认的事实 |
|---|---|---|---|
| H1 鉴权撤销 | `auth.rs`、`wms_api.rs`、ADR-0024 | `auth_users.permissions_changed_at`、`auth_sessions.revoked_at` | 故障时延迟窗口与请求 p99 |
| 双人策略缓存 | `dual_person_policy.rs`、`system_dictionary_handlers.rs` | 表查询 + 本地短缓存；必要时 `LISTEN/NOTIFY` | 变更传播时限、命中率、并发量 |
| 幂等与结果回放 | ADR-0018、ADR-0034、各 repository | `idempotency_request` + 索引/锁/清理任务 | QPS、TTL、并发冲突和回放一致性 |
| 限流/熔断/短期状态 | ADR-0018、`docs/infra/cache-strategy.md` | PostgreSQL 计数/锁或进程内单实例能力 | 是否需要多实例共享和低于现有 p99 目标 |
| 部署依赖 | `deploy/docker-compose.staging.yml`、`WMS_REDIS_URL` | 删除硬依赖或改为可选组件 | staging、备份、监控和故障演练成本 |

未在源码或运行配置中确认的 Redis 用途只能登记为“待核实”，不能作为保留理由。

## 2.1 当前可证实结论（非最终决策）

- `backend/crates/api/src/bin/wms_api.rs` 当前强制读取 `WMS_REDIS_URL`，并在启动时建立
  `RedisAuthRevocationStore`；因此“暂不做 Redis”不能只改 Compose，必须先替换鉴权撤销实现和启动装配。
- 双人策略和系统字典已有 PostgreSQL-only 构造路径；这两条链不能作为 Redis 必须保留的证据。
- PostgreSQL 已有 `auth_users.permissions_changed_at`、`auth_sessions.revoked_at` 等候选事实源，但当前运行时没有
  PostgreSQL 版 `AuthRevocationStore`；从 Redis 切换仍是独立实现、性能和故障语义变更。
- ADR-0018、ADR-0024、缓存策略和 staging Compose 都把 Redis 写成现行依赖；最终结论前不得直接删除依赖或改写
  Accepted ADR。

## 2.2 事实盘点（2026-07-31）

`governance/redis-usage-inventory.toml` 是当前入口清单，
`scripts/governance/check_redis_usage_inventory.py` 会扫描生产代码、依赖、部署和生成契约；新增 Redis 引用但未登记
会直接失败。当前清单共 15 个文件，分为以下 6 类：

| 类别 | owner | 真实入口 / 证据 | 当前语义 | PostgreSQL / 进程内候选 | 当前判断 |
|---|---|---|---|---|---|
| 鉴权撤销 | H1 鉴权 / 安全负责人 | `backend/crates/api/src/auth.rs`、`src/bin/wms_api.rs` | JWT `jti` 黑名单和 `permissions_changed_at` 用 Redis SET/GET + TTL；启动强制读取 `WMS_REDIS_URL`，连接失败时 API 不启动；运行时检查失败按 ADR-0024 fail-open | `auth_users.permissions_changed_at` 已存在；jti 需要新增带 `expires_at` 和索引的 PG 表，或改变 session/jti 语义 | 当前硬依赖；迁移需独立任务、故障矩阵和安全确认 |
| M-VR 双人策略缓存 | M-VR / 平台配置负责人 | `dual_person_policy.rs`、`dual_person_policy_handlers.rs`、`system_dictionary_handlers.rs` | Redis hash，owner 级 key，TTL 600 秒；读/写/失效失败均回退或保持 PostgreSQL 事实源 | 已有 `with_postgres` / `PgDualPersonPolicyRepository::new` 路径；可用 PG 直查 + 进程内短缓存 | 可选优化，不是功能硬依赖 |
| 幂等与 API Key 限流 | L11 弹性 / API 平台负责人 | `idempotency_request` SQL、`auth_api_keys.rate_limit_*` 相关 repository | 当前业务权威是 PostgreSQL 行锁、唯一约束、`expires_at` 和窗口计数；未发现运行时 Redis-first 幂等实现 | 现有 PG 结构已承载当前语义；跨实例高 QPS 仍需基准 | 当前不需要新增 Redis；ADR-0018 的 Redis-first 文案与实现不一致 |
| 熔断/限流运行时 | H3 弹性 / API 平台负责人 | `backend/crates/api/src/resilience.rs` 及 `bin/wms_api` 测试 | 当前 `ResilienceState` 为进程内状态；Redis 只出现在 ADR/缓存策略的目标描述 | 单实例继续进程内；多实例共享需单独选择 PG/Redis 并做容量验证 | 不是当前 Redis 使用面 |
| 依赖与部署 | 发布 / 运维负责人 | `backend/Cargo.toml`、`backend/crates/api/Cargo.toml`、`Cargo.lock`、`deploy/docker-compose.staging.yml` | Redis crate、staging 服务和 API 环境变量均已声明；staging API 依赖 Redis healthy | 删除前必须先替换鉴权装配、拆分 Compose 依赖并补回滚/演练 | 当前拓扑硬依赖，不能只删 Compose |
| 说明/生成证据 | 架构治理负责人 | `docs/adr/0018-resilience-engineering.md`、`docs/adr/0024-auth-model.md`、`docs/infra/cache-strategy.md`、前端文案和生成 schema | 部分是目标架构或降级说明，不等于运行时已实现 | 需在 successor ADR/迁移完成后同步，避免把目标写成现状 | 作为决策依据，不能单独证明“必须 Redis” |

### 事实边界

- 当前真正会阻止 API 启动的只有鉴权撤销装配；M-VR 缓存是可选连接，PostgreSQL 是事实源。
- `auth_sessions.revoked_at` 是会话级撤销记录，不能直接等价替代 JWT `jti` 黑名单；若要 PostgreSQL-only，必须明确
  token/session 绑定、过期清理、并发撤销和 fail-open/fail-closed 语义。
- `LISTEN/NOTIFY` 只能做失效通知，advisory lock 只能做锁，二者都不能直接替代 TTL 缓存或可靠消息队列。
- 当前没有证据证明幂等、outbox 或 API Key 限流必须经过 Redis；不能因 ADR/缓存策略中的目标文字反推运行时事实。

## 3. 评估方法

1. 用 `rg -n -i 'redis|WMS_REDIS_URL' backend apps packages deploy docs governance scripts` 生成清单，逐项补 owner、调用路径、数据语义和故障行为。
2. 对每项建立 PostgreSQL 替代说明：表/索引、事务边界、锁、过期清理、通知、恢复和多实例行为。
3. 复用 PostgreSQL 的真实能力，但不把它们包装成完整 Redis：advisory lock 只解决锁，`LISTEN/NOTIFY` 只解决通知，TTL 需要 `expires_at` 和清理任务。
4. 对鉴权、幂等和双人策略各做一个最小代表性基准；基准必须使用一次性 PostgreSQL、多实例并发和现有目标，不使用 dev mock 代替性能证据。
5. 给出“保留 Redis / PostgreSQL-only / 混合”的决策矩阵，并记录性能、可用性、故障、运维、成本和迁移风险。

## 3.1 初步决策矩阵

本轮项目主人已明确“不做 Redis”。这里将其记录为“不新增 Redis 用途、目标转向 PostgreSQL/进程内”，不等同于
立即删除当前 Redis：鉴权撤销的替代实现、迁移、回滚和安全验收必须另建任务；Accepted ADR 在 successor ADR
生效前保持不变。

| 用途 | 目标选型 | 仍需补的证据 / 后续任务 | REDIS-01 结论 |
|---|---|---|---|
| 鉴权撤销 | PostgreSQL-only 候选 | jti 表或 session 绑定设计；p99、并发、故障恢复；AR-01 高风险写策略 | 暂不删除；单独迁移任务 |
| M-VR 策略缓存 | PostgreSQL-only + 可选本地短缓存 | 变更传播时限、命中率和多实例容量基准 | Redis 非必要 |
| 幂等结果回放 | PostgreSQL-only | AR-06 统一语义、24h 清理和并发基准 | Redis 非必要 |
| 限流/熔断 | 先保留现有 PG/进程内实现 | 多实例 QPS 目标若变化，再建独立技术决策 | Redis 暂不引入 |
| staging/部署 | 后续移除 Redis 硬依赖 | 先完成鉴权替代、Compose/runbook/监控/演练 | 本任务不改拓扑 |

因此，本任务的可执行结论是：**不新增 Redis 用途；Redis 当前仍因鉴权撤销而暂时保留，直到独立迁移任务和
successor ADR 完成。** 缺少真实 PostgreSQL/多实例性能基准时，不把“PostgreSQL 有对应能力”写成已经满足同一
延迟、并发和故障语义。

参考：[PostgreSQL advisory locks](https://www.postgresql.org/docs/current/view-pg-locks.html)、[PostgreSQL LISTEN/NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html)。

## 3.2 2026-08-01 本地 PostgreSQL 行为证据

以下命令均在一次性测试数据库上通过：鉴权会话 `4/4`、双人策略 `5/5`、共享幂等 `2/2`。
其中共享幂等包含同键并发回放；双人策略覆盖 PostgreSQL-only 构造、规则解析、双人确认、审计和幂等。
鉴权测试证明会话撤销和租户隔离行为，但当前仍使用 `AuthRevocationStore` 测试替身，不能替代
PostgreSQL jti/session 替代实现、p99/多实例容量和 Redis 故障恢复基准，因此验收中的代表链证据继续保持 blocked。

```bash
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test auth_session_postgres -- --test-threads=1
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test dual_person_policy_postgres -- --test-threads=1
cargo test --manifest-path backend/Cargo.toml -p wms-api \
  --test shared_idempotency_postgres -- --test-threads=1
```

## 4. 验收标准

- [x] 每个 Redis 引用都能追溯到源码/依赖/配置/ADR，并有用途、owner、读写语义和故障行为。
- [x] 每个用途都有 PostgreSQL-only、保留 Redis 或混合方案的明确理由，不能只比较组件名称。
- [ ] 鉴权、幂等、双人策略三条代表链均有真实 PostgreSQL 行为或基准证据；缺失证据保持 blocked。
- [x] 明确哪些 Redis 能力是硬要求：鉴权撤销的低延迟、多实例共享 TTL；跨实例限流和广播目前没有运行时证据。
- [x] 明确 PostgreSQL 不能替代的部分，不把 `LISTEN/NOTIFY` 当可靠消息队列，不把 advisory lock 当缓存。
- [x] 记录项目主人“不做 Redis”的目标方向；如果改变 ADR-0018/0024 或部署拓扑，仍须先建 successor ADR，再建实现/迁移任务。
- [ ] 任何删除 Redis 的后续任务都有数据迁移、回滚、监控、容量和故障演练标准；本任务不执行删除。
- [ ] 将结论回写主整改计划、`docs/infra/cache-strategy.md`、相关 ADR、Compose/runbook 和 TODO 父状态。

## 5. 最小验证

```bash
rg -n -i 'redis|WMS_REDIS_URL' backend apps packages deploy docs governance scripts
python3 scripts/governance/check_doc_links.py --json
python3 scripts/governance/validate_doc_layers.py --json
just gov-t1
# 本任务新增后必须可运行：
python3 scripts/governance/check_redis_usage_inventory.py --json
python3 -m pytest scripts/governance/tests/test_check_redis_usage_inventory.py -q
```

## 6. Review Loop 与停止条件

每轮只处理一个用途：盘点事实 → 写失败 fixture/基准 → 评估 PostgreSQL 替代 → review → 修复证据缺口
→ 再 review → 记录结论。当前事实盘点和初步选型已完成；鉴权替代、真实性能/多实例基准和安全故障决策不足，
因此 REDIS-01 不关闭，后续实现任务仍标记 blocked。没有项目主人确认不得删除 Redis、修改部署硬依赖或改写
Accepted ADR。
