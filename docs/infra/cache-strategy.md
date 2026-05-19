# 缓存策略

> 定位：基础设施层文档
> 关联：模式提炼报告 §5.2 缺口 #5（29 次提及）；ADR-0001（Redis）；ADR-0013（配置）；ADR-0018（幂等键/限流/熔断状态存储）
> 文档层级：L2 规范

---

## 1. 选型

**Redis 7+**（ADR-0001 已选定）。单实例（小型）/ Sentinel（中型）/ Cluster（大型）按 ADR-0016 部署矩阵选。

---

## 2. 用途清单

| # | 用途 | Key 模式 | TTL | 失效策略 | Wave |
|---|------|---------|-----|---------|------|
| 1 | 幂等键存储 | `idem:{idempotency_key}` | 24h | 自然过期 | 1 |
| 2 | 限流计数器 | `rl:{dimension}:{id}:{window}` | 窗口期 | 自然过期 | 1 |
| 3 | H1 JWT token 黑名单 | `token:bl:{jti}` | token 剩余有效期 | 自然过期 | 1 |
| 4 | H1 权限码缓存 | `perm:{user_id}` | 5 min | 权限变更时主动删除 | 1 |
| 5 | M-VR 双人策略矩阵 | `vr:matrix:{owner_id}` | 10 min | 矩阵变更时主动删除 | 3 |
| 6 | M-PM 映射规则缓存 | `pm:rules:{owner_id}` | 10 min | 规则变更时主动删除 | 2 |
| 7 | 熔断器状态 | `cb:{service}` | 无（手动管理） | 状态变更时覆写 | 1 |
| 8 | Feature Flag | `ff:{flag_key}` | 60s | 配置变更时主动删除 | 1 |
| 9 | 热点商品信息 | `product:{id}` | 5 min | 商品变更时主动删除 | 2 |

---

## 3. 缓存失效策略

### 3.1 主动失效（Cache Aside + Event-Driven Invalidation）

```
写操作 → 先写 DB → 成功后删 Redis key → 下次读 miss 时回填
```

- **不用** Cache-Through / Write-Behind（复杂度高 + 一致性风险）
- 删除优于更新（避免并发写导致脏缓存）
- 事件驱动：H2-005 事件总线发布变更事件 → 订阅方删对应 key

### 3.2 被动失效（TTL 兜底）

所有缓存**必须设 TTL**（禁止永不过期）。TTL 是最后防线，防止主动失效遗漏。

---

## 4. 防护策略

| 问题 | 防护 |
|------|------|
| **缓存穿透**（查不存在的 key） | 布隆过滤器（商品 ID / 用户 ID）+ 空值缓存 TTL 30s |
| **缓存雪崩**（大量 key 同时过期） | TTL 加随机抖动（±20%）；热点 key 永不同时过期 |
| **缓存击穿**（热点 key 过期瞬间高并发） | 互斥锁（Redis `SET NX EX 5`）回填；仅 1 个请求回源 |
| **Redis 不可用** | 降级到 PG 直查（ADR-0018 §5 D2 级降级）；性能下降但功能完整 |

---

## 5. 约束

| 规则 | 说明 |
|------|------|
| Key 命名 | `{namespace}:{dimension}:{id}`，全小写，冒号分隔 |
| 序列化 | JSON（可读 + 调试友好）；性能敏感场景可用 MessagePack |
| 最大 value | ≤ 1 MB；超过拆分或不缓存 |
| 多租户隔离 | key 含 `tenant_id` 或 `owner_id`（按场景） |
| 监控 | `wms_cache_hit_total` / `wms_cache_miss_total` / `wms_cache_eviction_total` |
| 禁止 | 缓存敏感数据明文（密码/密钥）；缓存审计日志（必须走 PG append-only） |

---

## 6. 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：Redis 用途清单 9 项 + Cache Aside 失效策略 + 4 项防护 + 约束 |
