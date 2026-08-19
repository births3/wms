# ADR-0044：PostgreSQL 作为 HTTP 幂等权威存储

- 状态：Accepted
- 决策日期：2026-07-31
- 决策人：项目主人
- 局部取代：ADR-0018 §1 的 HTTP 写操作存储描述
- 关联：ADR-0034、ADR-0006 L6/L11、编码规范 §3.3、AR-06

## 背景

多个后端模块已经直接使用共享的 `idempotency_request` 表，但各自复制了锁、回放、过期和结果写入逻辑。
ADR-0018 的 Redis-first 文案与当前运行实现不一致；当前 HTTP 幂等事实源一直是 PostgreSQL。

## 候选方案

1. Redis → PostgreSQL 双存储：低延迟，但需要双写、故障恢复、顺序和一致性协议。
2. PostgreSQL-only：事务内锁定、回放、过期删除和业务写入保持同一提交边界。
3. 继续各模块自实现：改动小，但继续放大语义漂移和审计风险。

## 决策

HTTP 写操作统一采用 PostgreSQL-only：

- 幂等身份保持 `(owner_id, idempotency_key)`，不改 ADR-0034 的唯一约束。
- `request_hash`、HTTP method 和 path 共同参与冲突判断；同键同载荷同路径回放，同键异载荷或异操作返回冲突。
- 共享实现使用 PostgreSQL advisory transaction lock、`FOR UPDATE`、`expires_at` 和 24 小时 TTL；成功响应与业务写入在同一事务保存。
- 客户端继续生成 UUID v4；网络重试和人工重试必须复用原 key。
- 新增模块禁止直接读写 `idempotency_request`；迁移期间由可收缩基线门禁登记遗留访问。

本 ADR 不删除 Redis：鉴权撤销、现有独立外部协议和其他明确用途继续按各自 ADR 处理；本决策只约束 HTTP 幂等结果回放。

## 后果

- 正面：跨实例共享同一事实源，业务副作用、幂等记录和审计可保持事务一致；不新增 Redis 双写协议。
- 负面：高 QPS 幂等热点消耗 PostgreSQL 连接、行锁和磁盘；必须持续观察 p99、锁等待和过期清理。
- 迁移：首切片先迁移任务类型和参数映射；其余直接访问逐项迁移，基线归零后关闭例外。

## 验收约束

- L6 并发请求只能产生一次业务副作用。
- L11 同键回放返回一致结果；同键异 method/path/hash 必须冲突；过期记录允许重新执行。
- `python3 scripts/governance/check_idempotency_storage.py --json` 不得出现新增直接访问。
- 真实 PostgreSQL 测试覆盖锁定、回放、冲突、过期、owner 隔离和结果保存。

## 参考

- [ADR-0018：弹性工程](0018-resilience-engineering.md)
- [ADR-0034：Wave 3 PostgreSQL schema](0034-wave-3-operational-postgres-schema.md)
- [编码规范 §3.3](../coding-standards.md)
