# ADR-0025：审计存储模型（PostgreSQL append-only + 月分区 + JSONB diff + 哈希链）

- 状态：Accepted
- 决策日期：2026-05-24
- 修订日期：2026-05-24（v0.2，修 review 标注风险 3）
- 决策人：项目主人
- 来源：SPIKE-002 验证结果（accept，7/7 测试通过）
- 关联：ADR-0001（PG）/ ADR-0014（数据迁移）/ ADR-0024（鉴权与 actor 字段）/ ADR-0026（跨端契约）

---

## 1. 背景

H2 横向能力（audit-trail）覆盖：append-only 审计记录、操作前后值快照、操作人/时间/IP 全溯源。
GSP 第 9 条要求审计追踪不可篡改、保留期 5 年（部分模块 30 年）；本 ADR 决定如何在 PostgreSQL 内**强制**这些约束（不只是应用层"约定不写 UPDATE"）。

ADR-0001 选定 PG，但具体表结构、分区策略、不可变机制、完整性自检、与鉴权/审计查询/归档的衔接都没说清。
SPIKE-002 验证 5 个核心假设（H1-H5 全 accept），本 ADR 把验证结果固化为 Wave 1 W1.B 的硬约束。

---

## 2. 决策

### 2.1 表结构（锁定）

```sql
CREATE TABLE audit_event (
    id            BIGSERIAL,
    occurred_at   TIMESTAMPTZ NOT NULL,
    actor_id      UUID NOT NULL,             -- 来自 ADR-0024 AuthContext.user_id
    actor_name    TEXT NOT NULL,             -- 冗余（避免后续 user 改名时审计脱离上下文）
    owner_id      UUID NOT NULL,             -- 多租户隔离
    action        TEXT NOT NULL,             -- create/update/delete/login/logout/...
    module        TEXT NOT NULL,             -- M1/M2/H1/...
    resource_type TEXT,                      -- 实体类型（item/po/...）
    resource_id   TEXT,                      -- 实体业务 ID（PO-2026-0001）
    diff          JSONB,                     -- {before, after, changed_keys}
    request_id    UUID,                      -- 链路追踪（与 ADR-0011 衔接）
    ip            INET,
    user_agent    TEXT,
    prev_hash     TEXT,                      -- 上一条 self_hash（封档周期内）
    self_hash     TEXT NOT NULL,             -- sha256(prev_hash || canonical(row))
    PRIMARY KEY (id, occurred_at)            -- partition key 必须含主键
) PARTITION BY RANGE (occurred_at);
```

**字段选型**：
- `actor_name` 冗余非规范化：user 改名后历史审计仍显示旧名（合规要求）
- `diff JSONB`：单字段存 before/after/changed_keys，比拆三字段查询性能更好（spike-002 H3 验证）
- `request_id`：与 ADR-0011 可观测体系的 trace_id 关联
- `prev_hash + self_hash`：哈希链，篡改检测（详见 §2.4）

### 2.2 分区策略

**按月 RANGE 分区**（spike-002 H2 验证 partition pruning 完美生效）：

```sql
CREATE TABLE audit_event_2026_01 PARTITION OF audit_event
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
-- ... 每月一个
```

**滚动维护**：
- 起步 spike：手工建 12 个月（2026 全年）
- Wave 1 W1.B 上线：cron 每月 1 日跑 `create_next_partition.sql`，提前建下个月分区
- 单分区数据量预估：每月 5-10M 条事件（按业务规模估），3 年累计 60M+；EXPLAIN partition pruning 让查询仅扫单月分区

**分区上的 trigger**：必须给主表 + 所有子分区都挂 `audit_event_immutable` trigger（PG 不会自动继承）。spike-002 用 `DO $$` 块循环 `pg_inherits` 给所有现有分区挂同样 trigger；新建分区也必须同步挂。

### 2.3 不可篡改：trigger + 角色权限双重保护

**第一层 — trigger**（spike-002 H1 验证）：

```sql
CREATE OR REPLACE FUNCTION audit_event_immutable() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_event is append-only: % attempted by %', TG_OP, current_user
        USING ERRCODE = 'insufficient_privilege';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_event_no_update
    BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event
    FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable();
```

**第二层 — 角色权限**：

```sql
CREATE ROLE wms_app NOLOGIN;
GRANT INSERT, SELECT ON audit_event TO wms_app;
-- 不授予 UPDATE / DELETE / TRUNCATE
```

业务连接（应用 / API）必须以 `wms_app` 角色访问；DBA 紧急维护用专用账号且必须经审批。即使 DBA 临时 `DISABLE TRIGGER`，权限层仍拒。

**第三层 — 哈希链**（兜底）：即使前两层都被绕过（DBA 用 superuser 改数据），哈希链每日校验会立即发现（详见 §2.4）。

### 2.4 哈希链：方案 C 每日封档

SPIKE-002 §7.3 决定 hash chain 用**方案 C**：

- 每日 0:00 一个新 chain；同日内事件链接到前一条
- 跨天 chain 不连接（避免昨日修补后今日全链失效）
- 当日内并发 INSERT 用 `SELECT FOR UPDATE` 锁住"当日链头"行（开销可控，因为只锁单行）

每日 cron（02:00 跑）：
1. 校验昨日 chain 完整性（spike-002 `verify_hash_chain` 函数）
2. 把昨日 chain 的最后一条 self_hash 加签时间戳后存入只读表 `audit_chain_seal`
3. 异常告警走 ADR-0011 可观测告警

`audit_chain_seal` 表（spike 未实现，Wave 1 W1.B 实施）：

```sql
CREATE TABLE audit_chain_seal (
    seal_date     DATE PRIMARY KEY,
    last_id       BIGINT NOT NULL,
    last_self_hash TEXT NOT NULL,
    sealed_at     TIMESTAMPTZ NOT NULL,
    seal_signature TEXT                       -- 可选：用 KMS / HSM 加签
);
```

#### 2.4.1 并发场景未在 spike 验证（v0.2 修订加入）

**风险**：spike-002 H4 仅做单线程 timing（2.17ms/条），并发场景下「同日内多 INSERT 抢同一行 SELECT FOR UPDATE 锁住当日链头」未实测。1k QPS 真并发可能：
- chain head 行成为热点，序列化所有 INSERT
- P99 延迟从单线程 2.17ms 升到 10-100ms 量级（仍远低于 200ms 假设阈值，但需实测）

**fallback 决策**：
- **W1.B 退出条件之一已含**「wrk 1k QPS × 1 小时压测 P99 < 200ms」（§4 实施 checklist）— 这是并发场景的真验证关口
- **若 P99 不达标**：启动 spike-002b 评估替代方案：
  - 候选 a：专用 sequence + 顺序号（`audit_chain_seq` 序列每条 +1，prev_hash 改为查"当日内 seq - 1"，避免 SELECT FOR UPDATE）
  - 候选 b：弱一致 — 接受 prev_hash 短期错链，每日 02:00 cron 修补
  - 候选 c：每小时封档（24 chain → 1440 chain，单 chain 锁竞争更小）
- **spike-002b 启动条件**：W1.B 实测 P99 > 200ms，或同日内并发 INSERT 出现死锁/超时

**为什么不在 spike-002 阶段做**：
- spike 时间盒已超满（4/5 spike 实工 10.5h vs 8.5d 时间盒 → 10x 加速比是非常态）
- 并发问题本质是 W1.B 实施细节，不是"是否能用 hash chain"的核心假设证伪
- W1.B 退出条件已留兜底口子；若问题暴露，spike-002b 是合理后备

### 2.5 索引

```sql
-- 按变化字段过滤（jsonb_path_ops 比 jsonb_ops 索引小约 1/3）
CREATE INDEX audit_event_diff_changed_keys_idx
    ON audit_event USING gin (diff jsonb_path_ops);

-- 常规过滤
CREATE INDEX audit_event_actor_idx ON audit_event (actor_id, occurred_at DESC);
CREATE INDEX audit_event_module_idx ON audit_event (module, occurred_at DESC);
CREATE INDEX audit_event_owner_idx ON audit_event (owner_id, occurred_at DESC);
```

**不索引**：
- `action`（基数低，按 module 过滤后 seq scan 已够快）
- `request_id`（仅审计排查用，按 occurred_at 范围过滤后扫即可）

### 2.6 写入路径

```rust
pub async fn append_event(pool: &PgPool, req: &AuditWriteRequest) -> AuditResult<i64> {
    // 1. 取昨夜以来的 chain head（同日 SELECT FOR UPDATE 锁单行）
    // 2. 计算 self_hash = sha256(prev_hash || canonical_row)
    // 3. INSERT 带 occurred_at（必含，partition key）
}
```

**性能基线**（spike-002 H4 验证，单线程）：
- 100 条插入 0.22s，平均 **2.17ms/条**
- 远低于假设阈值 P99 < 200ms（约 100x 安全边际）

**Wave 1 W1.B 必做的并发压测**（W1.B 退出条件）：
- wrk 1k QPS × 1 小时 + 60M 行 baseline
- 验证 P99 在并发场景仍 < 200ms

### 2.7 归档与保留

依据 ADR-0014 §数据迁移：

| 数据 | 保留期 | 归档触发 | 存储介质 |
|------|--------|---------|---------|
| `audit_event`（普通 GSP） | 5 年 | 365 天后归档 | S3 冷存储 (cn-northwest-1) |
| `audit_event`（追溯码原始事件 M-TC）| 5 年 | 180 天后归档 | S3 冷存储 |
| `audit_event`（特殊药品分类，对应 H10 矩阵）| 30 年 | 5 年后归档 | S3 冷存储 |

归档机制：`pg_partman` + `aws s3 sync`，detach 老分区 → dump → 上传 → drop（保留 metadata 索引）。
具体实施在 Wave 4+（见 ROADMAP §v25 backlog 特殊药品落地）。

### 2.8 与 ADR-0024（鉴权）/ ADR-0026（跨端契约）衔接

| ADR | 衔接点 |
|-----|--------|
| ADR-0024 | 每条 audit 的 `actor_id` `actor_name` `owner_id` 来自 AuthContext；`request_id` 与登录态/请求级 trace 关联 |
| ADR-0026 | `AuditEvent` struct 加 `#[derive(ToSchema)]`；前端 H2 审计查询页用 `@wms/api-client` 拿到类型；diff 字段在 OpenAPI 是 `additionalProperties: true`（前端 unknown，运行时按 changed_keys 决定渲染） |
| ADR-0011 | `request_id` ↔ tracing span_id 一一对应；告警链路对接 |

---

## 3. 候选方案

### A. 本决策（PG trigger + 月分区 + JSONB diff + 每日封档 hash chain）— 接受

理由：spike-002 全 5 假设 accept；PG 单库满足需求；不引入新中间件；与 SQLx (ADR-0001 §SQLx 附录) 自然衔接。

### B. EventStore / Apache Kafka audit log — 否决

理由：
- 引入新中间件 = 新可观测 / 备份 / HA 链路
- 跨服务 EventBus 模式当前不需要（单后端集群）
- 写新 ADR 评估的成本远高于本方案

### C. 应用层"约定不写 UPDATE" — 否决

理由：违反 GSP 不可篡改要求；DBA 误操作 / SQL 注入 / 未来 ORM 切换都会绕过应用层约束。本 ADR 强制 DB 层（trigger + 权限）+ 业务层（哈希链）三重保护。

### D. 哈希链 per-row（每行单独算 hash）— 否决

理由：性能差（每条 INSERT 必须 SELECT FOR UPDATE 锁前一条）；并发场景不可接受。改用**方案 C 每日封档**（spike-002 §7.3 决定）。

### E. 哈希链跨天连续（永远不断）— 否决

理由：跨天连续意味着任何一天的修补都会破坏后续所有 chain；运维成本高。每日封档 + audit_chain_seal 加签足够 GSP 合规。

### F. PostgreSQL Row-Level Security 强制 owner_id — 推迟

理由：当前用 ADR-0024 §2.7 "middleware 注入 + handler 过滤 + audit 兜底"模式。RLS 学习曲线陡 + SQLx 协作模式未定型；Wave 2+ 评估写新 ADR。

---

## 4. 实施 checklist（Wave 1 W1.B 启动时）

- [ ] `backend/migrations/<ts>_audit_event.sql`：迁 spike-002 schema 到主项目
- [ ] `backend/migrations/<ts>_wms_app_role.sql`：建角色 + 授权
- [ ] `backend/crates/infra/src/audit.rs`：实现 `append_event`（含 SELECT FOR UPDATE 锁同日 chain head）
- [ ] `backend/crates/infra/src/audit/seal.rs`：每日 cron 任务（封档 + 校验）
- [ ] `audit_chain_seal` 表 schema + migration
- [ ] H1 衔接：业务 handler 写 audit 时从 `AuthContext` 取 actor/owner/jti
- [ ] H3 衔接：`AuditEvent` 加 `#[derive(ToSchema)]`，前端 packages/api-client 自动生成类型
- [ ] 治理脚本 `check_audit_required.py`（T2，diff 触发）：业务 handler 写操作必须含 `audit::append_event` 调用
- [ ] 文档：`docs/governance.md` §x 加"审计追踪不可绕过"红线
- [ ] **W1.B 退出条件之一**：wrk 1k QPS × 1 小时 + 60M 行 baseline 压测，P99 < 200ms
- [ ] **W1.B 退出条件之二**：每日封档 cron 在 dev 环境跑 7 天无异常

---

## 5. 后果

### 正面

- **GSP 合规硬性要求达成**：trigger + 权限 + hash chain 三层保护，"应用层不写 UPDATE"不再是孤证
- **审计查询性能可控**：partition pruning + 4 个针对性索引，单查询命中单分区
- **不引入新中间件**：单 PG 集群满足；运维边界清晰
- **与 ADR-0011 / 0024 / 0026 协同**：request_id / actor_id / OpenAPI 类型一处定义全链复用

### 负面

- **每月新分区维护**：cron job 必须可靠；遗忘建分区会导致写入失败（业务全停）；缓解：建当月 + 提前建未来 3 个月，多 fail-safe
- **trigger 在子分区也需要挂**：PG 限制；新建分区脚本必须同步挂；治理脚本 `check_audit_partitions.py` 强制
- **每日 hash chain 封档需要监控**：cron 失败必须告警 P1；ADR-0011 可观测落地

### 风险

- **DBA 越权（DISABLE TRIGGER + UPDATE）**：spike-002 t5 模拟过；缓解：hash chain 每日校验 + DBA 操作走 PG 自身的 pg_audit 扩展（Wave 2+ 评估）
- **并发 INSERT 在同日 chain head 上排队**：单行 SELECT FOR UPDATE 开销小但仍有锁；W1.B 实测 1k QPS 看是否成瓶颈，必要时改方案 A（专用 sequence）
- **JSON diff 大对象（如批量更新 100 字段）**：JSONB 存储 toast 化即可；jsonb_path_ops 索引仅按 changed_keys 顶层数组工作，不影响 diff 体积
- **5 年保留 + 每月 5-10M 行 = 300-600M 行**：单 PG 实例可应付；W4+ 数据量增长需评估水平分库（ADR-0014 §数据迁移已留接口）

---

## 6. 关联文档

- [SPIKE-002 验证记录](../spikes/spike-002-h2-append-only.md)
- [Spike 代码](../../spikes/spike-002-h2-append-only/)
- [ADR-0001 技术栈 + §SQLx 附录](0001-tech-stack.md)
- [ADR-0014 数据迁移](0014-data-migration.md)
- [ADR-0024 鉴权模型](0024-auth-model.md)（actor 字段来源）
- [ADR-0026 跨端契约](0026-cross-end-contract-pipeline.md)（前端类型生成）
- [ADR-0011 可观测](0011-observability.md)（request_id 关联 + cron 告警）


---

## 7. 修订记录

### v0.2 — 2026-05-24（review 后修风险 3）

针对 Wave 0.5 退出前的集中 review 标注的"hash chain 并发场景未在 spike 验证"风险：

- 修：§2.4 末尾新增 §2.4.1 "并发场景未在 spike 验证"段
  - 明示 spike-002 H4 仅做单线程 timing（2.17ms/条）
  - W1.B 退出条件已含「wrk 1k QPS × 1 小时压测 P99 < 200ms」作硬关口
  - 若 P99 不达标 → 启动 spike-002b 评估 3 个候选方案：
    a. 专用 sequence + 顺序号
    b. 弱一致 prev_hash + 每日 cron 修补
    c. 每小时封档（chain 数从 365/年 → 8760/年，单 chain 锁竞争更小）
  - 说明 spike 阶段不做并发验证的理由（时间盒已超满 + W1.B 退出兜底）

### v0.1 — 2026-05-24（初版，SPIKE-002 验证后产出）

详见 §1-§6。
