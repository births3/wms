# ADR-0018：弹性工程（幂等 / 重试 / 限流 / 熔断 / 降级 / 死信）

- 状态：Accepted
- 决策日期：2026-05-18
- 决策人：项目主人
- 关联：ADR-0006 L11 / ADR-0010 / ADR-0011 / ADR-0012 / coding-standards §3.3 / technical-specs §H8 / usability-baseline §2.5 / clarifications #36 / pattern-extraction §5.2 缺口 #1

---

## 背景

模式提炼报告（2026-05-18）识别"弹性工程"为 P0 缺口：

- 故事中词频：幂等 84 + 重试 43 + 降级 24 + 限流 10 + 熔断 8 + 死信 4 = **173 次**
- 当前覆盖：**零散分布在 5+ 文档中**，无统一方案
- 影响：Wave 1 H1/H2/H8 实施时就需要幂等/重试/限流，缺统一方案会导致各模块各做各的

本 ADR 把已有约束**聚合为单一事实之源**，并补齐缺失的熔断/降级/限流实现方案 + 死信处理流程 + 重试策略分级。

---

## 已有约束（本 ADR 继承，不重复定义）

| 来源 | 约束 | 本 ADR 角色 |
|------|------|------------|
| coding-standards §3.3 | 幂等键规范（Idempotency-Key + Redis/PG + TTL 24h + 前端自动附加） | 继承，不修改 |
| ADR-0006 L11 | 写操作必须有幂等性测试（testcontainers + 重放断言） | 继承 |
| technical-specs §H8 | ERP 防腐层重试（3 次指数退避 / 档案补录 5 次 / 死信 / 降级） | 继承，本 ADR 泛化为全局策略 |
| usability-baseline §2.5 | 限流违规告警 ≤ 1 分钟 | 继承 |
| clarifications #36 | 全局 1000 QPS / 单用户 100 QPS | 继承 |
| ADR-0012 H2 BC | DLQ 实体 | 继承 |
| ADR-0010 | error severity 含 `retry_hint` | 继承 |

---

## 决策

### 1. 幂等（Idempotency）

**已由 coding-standards §3.3 完整定义**，本 ADR 仅补充分级：

| 场景 | 幂等键来源 | 存储 | TTL |
|------|-----------|------|-----|
| 前端写操作 | 客户端 UUID v4（TanStack Query 自动附加） | Redis → PG fallback | 24h |
| 模块间事件消费 | `event_id`（H2-005 事件总线自带） | PG `processed_events` 表 | 7d |
| ERP 回调 | 外部 `request_id` 或 `Idempotency-Key` | Redis | 24h |
| 定时任务 | `task_type + scheduled_at` 组合键 | PG | 任务周期 × 2 |

### 2. 重试（Retry）

#### 2.1 重试策略分级

| 级别 | 场景 | 最大次数 | 退避策略 | 总超时 | 示例 |
|------|------|---------|---------|--------|------|
| **L0 不重试** | 业务校验失败（4xx） | 0 | — | — | 库存不足、权限拒绝 |
| **L1 快速重试** | 瞬时网络抖动 | 3 | 指数退避（100ms / 200ms / 400ms） | 2s | Redis 连接超时 |
| **L2 标准重试** | 外部服务暂不可用 | 5 | 指数退避 + 抖动（1s / 2s / 4s / 8s / 16s ± 20%） | 60s | ERP 接口 / 码上放心 |
| **L3 持久重试** | 关键业务不可丢 | 无限（进死信前） | 固定间隔 5min | 24h | 档案补录（technical-specs §H8 已定义） |

#### 2.2 重试判定规则

```
收到响应 →
  HTTP 4xx（非 429）→ L0 不重试，返回错误
  HTTP 429 → 按 Retry-After 头等待，然后 L1
  HTTP 5xx / 超时 / 连接失败 →
    error.retry_hint == "no_retry" → L0
    error.retry_hint == "fast" → L1
    error.retry_hint == "standard" → L2（默认）
    error.retry_hint == "persistent" → L3
```

#### 2.3 实现约束

- 重试必须携带相同 `Idempotency-Key`（保证幂等）
- 重试日志必须含 `attempt_number` + `next_retry_at`（ADR-0011 可观测性）
- 重试 metric：`wms_retry_total{service, level, outcome}` counter

### 3. 限流（Rate Limiting）

#### 3.1 配额（继承 clarifications #36）

| 维度 | 配额 | 算法 | 存储 |
|------|------|------|------|
| 全局 | 1000 QPS | 令牌桶（Token Bucket） | Redis |
| 单用户 | 100 QPS | 滑动窗口（Sliding Window Log） | Redis |
| 单 IP（未认证） | 20 QPS | 固定窗口 | Redis |

#### 3.2 响应

- 超限 → HTTP 429 + `Retry-After` 头（秒）+ `X-RateLimit-Remaining: 0`
- 正常 → `X-RateLimit-Limit` + `X-RateLimit-Remaining` + `X-RateLimit-Reset`

#### 3.3 豁免

- 健康检查 `/healthz` `/readyz` 不计入限流
- 内部服务间调用（通过 `X-Internal-Service` 头 + mTLS 验证）不计入用户限流，仅计入全局

#### 3.4 监控

- 限流触发 → H-AL 告警（≤ 1 分钟，继承 usability-baseline §2.5）
- metric：`wms_rate_limit_rejected_total{dimension, user_id}`

### 4. 熔断（Circuit Breaker）

#### 4.1 适用范围

仅对**外部依赖**启用熔断（不对内部模块间调用熔断）：

| 外部依赖 | 熔断阈值 | 半开探测 | 恢复条件 |
|---------|---------|---------|---------|
| ERP 接口（H8） | 连续 5 次失败 或 60s 内失败率 > 50% | 每 30s 放 1 请求探测 | 连续 3 次探测成功 |
| 码上放心（M-TC） | 连续 5 次失败 或 60s 内失败率 > 50% | 每 30s 放 1 请求探测 | 连续 3 次探测成功 |
| 企业微信（H4） | 连续 10 次失败 | 每 60s 放 1 请求探测 | 连续 2 次探测成功 |
| 快递 API（H5） | 连续 10 次失败 | 每 60s 放 1 请求探测 | 连续 2 次探测成功 |

#### 4.2 状态机

```
Closed（正常）→ [阈值触发] → Open（熔断）→ [半开定时器] → HalfOpen → [探测成功] → Closed
                                                                    → [探测失败] → Open
```

#### 4.3 实现

- Rust 侧推荐 `tower` middleware 或自研轻量 `CircuitBreaker<S>` wrapper
- 状态存储：进程内（单实例）或 Redis（多实例共享）
- metric：`wms_circuit_breaker_state{service}` gauge（0=closed / 1=half_open / 2=open）

### 5. 降级（Degradation）

#### 5.1 降级策略分级

| 级别 | 触发条件 | 行为 | 恢复 |
|------|---------|------|------|
| **D0 无降级** | 正常 | 全功能 | — |
| **D1 功能降级** | 非关键外部依赖熔断 | 关闭非核心功能（如通知推送），核心业务不受影响 | 熔断恢复后自动 |
| **D2 数据降级** | 缓存/查询服务不可用 | 返回缓存数据 + `X-Degraded: true` 头 | 服务恢复后自动 |
| **D3 业务降级** | 关键外部依赖熔断 | 暂存消息 + 人工介入提示 | 人工确认后恢复 |

#### 5.2 降级矩阵（外部依赖 × 降级级别）

| 外部依赖 | 熔断时降级级别 | 具体行为 |
|---------|-------------|---------|
| ERP（H8） | D3 | 消息暂存队列；WMS 核心业务不阻塞（除档案补录阻塞当前 ASN） |
| 码上放心（M-TC） | D3 | 上报暂存；出库不阻塞（追溯码本地已核销） |
| 企业微信（H4） | D1 | 通知静默丢弃 + 告警；业务不受影响 |
| 快递 API（H5） | D1 | 运单号获取失败 → 人工填写兜底 |
| Redis 缓存 | D2 | 直接查 PG；性能下降但功能完整 |

#### 5.3 降级可观测

- 降级触发/恢复 → H2 审计事件（`system.resilience.degraded` / `system.resilience.recovered`）
- 降级状态 → Grafana 面板 + H-AL 告警
- metric：`wms_degradation_active{service, level}` gauge

### 6. 死信（Dead Letter Queue）

#### 6.1 进入死信条件

- L2 重试耗尽（5 次 / 60s）
- L3 重试超过 24h 总超时
- 消息格式无法解析（poison message）

#### 6.2 死信处理流程

```
消息重试耗尽
    ↓
写入 DLQ 表（PG `dead_letter_queue`）
    ↓
触发 H-AL 告警（severity = error）
    ↓
运维/业务方在管理后台查看
    ↓
├─ 修复后重新投递（retry from DLQ）
├─ 标记为已处理（acknowledge）
└─ 转人工工单（M-QL 质量联系单）
```

#### 6.3 DLQ 表结构（方向性）

```sql
CREATE TABLE dead_letter_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    source_module TEXT NOT NULL,        -- 来源模块（H8/M-TC/H4...）
    event_type TEXT NOT NULL,           -- 事件类型
    payload JSONB NOT NULL,             -- 原始消息体
    error_message TEXT NOT NULL,        -- 最后一次失败原因
    retry_count INT NOT NULL,           -- 已重试次数
    first_failed_at TIMESTAMPTZ NOT NULL,
    last_failed_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending/retrying/acknowledged/escalated
    acknowledged_by UUID,               -- 处理人
    acknowledged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

#### 6.4 DLQ 治理

- 每日 09:00 自动告警：pending 条目 > 0 → 推送运维群
- pending > 7 天 → 自动升级为 `escalated` + 推送仓库主管
- DLQ 数据保留 90 天，之后归档到 `dead_letter_archive`

---

## 后果

### 正面

- **统一方案**：6 个弹性能力有单一事实之源，不再各模块各做各的
- **分级清晰**：重试 4 级 / 降级 4 级 / 限流 3 维度，实施时直接查表
- **可观测**：每个能力都有 metric + 告警 + 审计事件
- **GSP 合规**：幂等保证不重复数据 + 死信保证不丢数据 + 审计追踪降级事件

### 负面

- **Redis 强依赖**：限流 + 幂等 + 熔断状态都依赖 Redis
- **应对**：Redis 不可用时降级到 PG（D2 级别）；Wave 1 末补 infra/cache-strategy.md

### 风险

- **熔断阈值不适配**：初版阈值（5 次 / 50%）可能对 wms 流量不适配
- **应对**：Wave 1 末用真实流量校准（与 ADR-0016 灰度策略同理）
- **DLQ 堆积**：运维不及时处理 → 堆积 → 磁盘满
- **应对**：7 天自动升级 + 90 天归档 + 磁盘告警

---

## 实施约束

1. 所有外部调用必须包裹 `resilience` middleware（重试 + 熔断 + 降级）
2. 所有写操作必须携带 `Idempotency-Key`（继承 coding-standards §3.3 红线）
3. 限流配置必须可运行时调整（通过 M1-008 配置中心 / Wave 1 用环境变量）
4. 死信处理必须有管理后台 UI（Wave 2 M6 报表模块承载）
5. 弹性 metric 必须接入 ADR-0011 可观测性体系（Prometheus + Grafana）
6. 降级事件必须写入 H2 审计追踪

---

## 与其他 ADR 的关系

| ADR | 关系 |
|-----|------|
| ADR-0006 L11 | 幂等性测试验证本 ADR §1 的实现正确性 |
| ADR-0010 | `retry_hint` 字段驱动本 ADR §2.2 重试判定 |
| ADR-0011 | 弹性 metric 纳入可观测性体系 |
| ADR-0012 | DLQ 实体属于 H2 BC |
| ADR-0013 | 限流配额 / 熔断阈值存于配置中心（Wave 2 起）|
| ADR-0016 | 灰度发布时弹性阈值可能需要按阶段调整 |
| coding-standards §3.3 | 幂等键规范的单一事实之源 |
| technical-specs §H8 | ERP 防腐层的具体重试/降级参数 |
| usability-baseline §2.5 | 限流告警 SLA |

---

## 变更记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-05-18 | v1 | 初版：6 大弹性能力统一方案（幂等分级 / 重试 4 级 / 限流 3 维度 / 熔断状态机 / 降级 4 级 / 死信处理流程 + DLQ 表）|
