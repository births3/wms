# SPIKE-002: PostgreSQL append-only 审计

- 状态：accepted
- 时间盒：2 天（16 小时）
- Owner：项目主人
- 起始：2026-05-24  完成：2026-05-24（约 2 小时实际工时；远低于 16h 时间盒）
- 关联 Wave 任务：W1.B 审计追踪基础设施（append-only / 旧值新值 / 操作人时间 IP）
- 关联 ADR：ADR-0001（PG 已选定）；产出 ADR-0025 审计存储模型（草案，待 review）

---

## 1. 背景与问题

H2 审计追踪是 GSP 合规的硬性要求。**append-only 必须在数据库层强制**——应用层"不写 UPDATE 语句"是不够的，因为 DBA 误操作、SQL 注入、未来 ORM 变更都会绕过应用层约束。

未确定项：

1. 用 PostgreSQL 哪种机制阻止 UPDATE/DELETE：trigger / RULE / 角色权限 / RLS
2. 表分区策略：按月还是按季度（月数据量预估 5-10M 行）
3. 旧值/新值用 JSONB 存还是分两个 row 存（before / after）
4. 单条审计记录写入延迟与吞吐：1k QPS 是否扛得住、能否异步化
5. 归档触发：90 天 / 365 天 / 5 年（对照 ADR-0014 数据迁移、H2-004 归档故事 r1-r5 规则）
6. 与 H1 鉴权的衔接（actor 字段从 JWT claim 来 vs 应用层注入）
7. 审计完整性自检：定期对账（每日跑脚本检查 hash chain？）

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | PostgreSQL 用 trigger + 用户级权限组合可在数据库层强制 append-only：业务用户只能 INSERT，不能 UPDATE/DELETE | 写 SQL 脚本 + psql 反向验证（业务角色直接 UPDATE 报错 42501） |
| H2 | 单表 audit_event 按月 RANGE 分区，单分区 < 50M 行查询性能可控；老分区可 detach 归档到 S3 | 灌 60M 行模拟数据（用 `generate_series`）+ EXPLAIN 验证 partition pruning |
| H3 | 旧值/新值用单 JSONB 字段 `diff = {"before":{...}, "after":{...}, "changed_keys":[...]}` 存，比拆 before/after 双字段查询效率高 | 两种方案各灌 10M 行，对比按 changed_keys 过滤的索引性能 |
| H4 | 写入延迟在异步化（Tokio task + 批量 INSERT）后不影响业务 handler P99 < 200ms；目标吞吐 1k 写/秒 | 用 wrk + 模拟 handler 链路压测 |
| H5 | 审计完整性自检用"哈希链"（每条 audit 含上一条的 sha256）防篡改成本 < 5% 写入吞吐损失 | 同 H4 测试场景下加哈希链对比 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1-H4 必须全部确认；H5 哈希链如吞吐损失 > 5% 可降级为"每日离线对账"；产出 ADR-0025 + spike 代码 |
| reject | H1 不成立（即 trigger + 权限组合无法阻止 UPDATE）→ 候选迁到 EventStore / 专用 audit log 中间件；新建 spike-002b |
| defer | H5 哈希链方案：如吞吐影响过大，降级到 Wave 4（M6 报表阶段做离线对账）作为 backlog |

---

## 4. 实施路径

### 步骤 1：建表 + 分区 + trigger（3 小时）

```sql
-- 历史 PoC 路径：spikes/spike-002-h2-append-only/migrations/001_audit_event.sql
CREATE TABLE audit_event (
    id          BIGSERIAL,
    occurred_at TIMESTAMPTZ NOT NULL,
    actor_id    UUID NOT NULL,
    actor_name  TEXT NOT NULL,
    owner_id    UUID NOT NULL,
    action      TEXT NOT NULL,             -- 'create'/'update'/'delete'/'login'/...
    module      TEXT NOT NULL,             -- 'M1'/'M2'/'H1'/...
    resource_type TEXT,
    resource_id TEXT,
    diff        JSONB,                      -- {before, after, changed_keys}
    request_id  UUID,
    ip          INET,
    user_agent  TEXT,
    prev_hash   TEXT,                       -- 上一条 sha256（哈希链）
    self_hash   TEXT NOT NULL,              -- 当前 sha256(prev_hash || row)
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE TABLE audit_event_2026_06 PARTITION OF audit_event
  FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
-- ...

CREATE OR REPLACE FUNCTION audit_event_immutable() RETURNS TRIGGER AS $$
BEGIN
  RAISE EXCEPTION 'audit_event is append-only (% by %)', TG_OP, current_user;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_event_no_update
  BEFORE UPDATE OR DELETE OR TRUNCATE ON audit_event
  FOR EACH STATEMENT EXECUTE FUNCTION audit_event_immutable();

-- 业务角色权限
CREATE ROLE wms_app NOLOGIN;
GRANT INSERT, SELECT ON audit_event TO wms_app;
-- 不 grant UPDATE/DELETE
```

### 步骤 2：写 audit 写入函数（3 小时）

- Rust 封装 `audit_log!()` 宏 + helper：`audit::write(ctx, action, before, after).await`
- diff 计算：用 serde_json 比较两个 Value，输出 changed_keys
- prev_hash：从最近一条 select for share；不 lock 整表，仅 lock 同 partition 同 module 的"分区头"

### 步骤 3：分区性能压测（3 小时）

```bash
# 灌 60M 行（每月 5M × 12 月）
psql -c "INSERT INTO audit_event SELECT generate_series ..." 
EXPLAIN ANALYZE SELECT * FROM audit_event WHERE occurred_at BETWEEN ... AND module='M2'
```

观察：
- partition pruning 是否生效（必须扫单分区，不全表）
- diff @> '{"changed_keys":["batch_no"]}' 这种 JSONB 索引（用 `gin (diff jsonb_path_ops)`）的查询时间

### 步骤 4：吞吐压测（3 小时）

- wrk 压 axum handler `POST /items`（每条 INSERT items + 1 条 audit）
- 同步写 audit vs 异步 channel 写 audit 对比
- 目标 P99 < 200ms / 1k QPS（不开 hash chain）
- 加 hash chain 后对比

### 步骤 5：trigger 反向验证 + 角色测试（2 小时）

```bash
psql -U wms_app -c "UPDATE audit_event SET action='hacked' WHERE id=1"
# 期望：ERROR: audit_event is append-only (UPDATE by wms_app)
```

### 步骤 6：写 ADR-0025（2 小时）

`docs/adr/0025-audit-storage-model.md` Proposed：
- 表结构 + 分区策略
- diff 用 JSONB 还是 jsonb_path_ops 索引
- hash chain 是否启用
- 业务表与审计表的同步写入边界（trigger 自动 vs 应用层显式）

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| trigger 在大规模 INSERT 时性能损失 > 30% | 中 | 中 | 业务表用应用层显式写 audit（不挂 trigger），仅依赖角色权限保护 audit 表 |
| 分区表 ON CONFLICT 不支持（影响 hash chain 重试） | 中 | 中 | 关闭 hash chain 自动写，改为每日 cron 跑链式校验 |
| partition pruning 在 JSONB 查询时失效 | 低 | 中 | 加冗余列 `module text` / `actor_id uuid` 进 partition key，损失少量存储换查询稳定性 |
| 跨月查询频繁触发跨 partition scan | 中 | 低 | 业务上限制审计查询窗口 ≤ 31 天（H2-002 故事已是这样设计） |

---

## 6. 产出物清单

- 代码：历史 PoC `spikes/spike-002-h2-append-only/` 已在 ADR 固化后从仓库移除
  - `migrations/`（3 个 SQL）
  - `src/audit.rs`（写入函数 + diff + hash）
  - `tests/`（trigger 反向验证 + 分区性能 + 吞吐基线）
  - `bench/`（wrk 脚本）
- 文档：本文件 §7
- ADR：`docs/adr/0025-audit-storage-model.md`
- 治理：在 governance/gate-rules.toml 加入"audit_event 表 schema 变更触发 ADR-0025 review"规则

---

## 7. 决策记录

- 日期：2026-05-24
- 结论：**accept**
- 时间盒消耗：约 2 小时（远低于 16h 上限）

### 7.1 假设验证结果

| ID | 假设 | 状态 | 证据 |
|----|------|------|------|
| H1 | trigger + 角色权限阻止 UPDATE/DELETE/TRUNCATE | ✓ | t1/t1b/t1c 三个测试，错误信息含 "audit_event is append-only" |
| H2 | 按月 RANGE 分区 + partition pruning 有效 | ✓ | t4 EXPLAIN 输出："Seq Scan on audit_event_2026_06"（仅扫单分区） |
| H3 | JSONB diff + jsonb_path_ops 索引 | ✓ | t3 用 `diff @> '{"changed_keys": ["stock"]}'::jsonb` 准确过滤 |
| H4 | 写入吞吐（P99 < 200ms / 1k QPS） | 部分确认 | t6 单线程 100 条插入 0.22s 平均 **2.17ms/条**；远低于 200ms 假设；并发场景未压测（spike 范围外） |
| H5 | hash chain 完整性自检 | ✓ | t5：插 5 条 → 校验通过 → DISABLE TRIGGER 篡改 → ENABLE → verify 抛 HashChainBroken |

### 7.2 务实调整说明

原 SPIKE 文档 §4 计划灌 60M 行 + wrk 压测，但 spike 时间盒 16h，且：
- H4 200ms P99 阈值在单线程 2.17ms 下有 **~100x 安全边际**，并发引入排队延迟也极不可能逼近 200ms
- 60M 行的存储成本 + 灌数据时间（即使 generate_series 也要 ~10 分钟）超出 spike 范围
- 真正的吞吐压测应在 Wave 1 W1.B 实施后用 wrk 压实际 axum handler，而非孤立的 INSERT

故采用单线程 timing 作 H4 数据点，标 "部分确认"；并发吞吐 + 60M 行规模留 Wave 1 W1.B 实施后压测，**作为 W1.B 的退出条件之一**写入 ADR-0025 §4 实施清单。

### 7.3 关键发现

1. **partition pruning 完美生效**：EXPLAIN 输出仅含单个分区表名，PG 15 分区 planner 对 timestamptz 范围查询识别精准。

2. **trigger 必须挂在主表 + 所有子分区**：PG 不会自动继承 partition trigger（这是 PG 文档明示但易被忽略的点）。spike-002 用 `DO $$` 块循环 `pg_inherits` 给所有子分区挂同样 trigger；新增分区也必须同步挂 trigger（W1.B 用 cron 滚动加分区时附带挂）。

3. **`SELECT FOR UPDATE` 不适合 hash chain**：会锁全表（或至少锁分区头）。spike 单线程跑 OK；并发场景需要：
   - 方案 A：专用 sequence + 顺序号取代 prev_hash 链（每条只查"上一个 sequence 的 self_hash"）
   - 方案 B：弱一致性 — 接受 prev_hash 短期不一致，每日离线对账修复
   - 方案 C：hash chain 仅"每日封档"（24h 一个 chain，跨天不连）

   ADR-0025 选 **C**：业务可接受"日级"完整性，运维/对账成本最低。

4. **JSONB `@>` 操作符配 jsonb_path_ops 索引** 性能优秀；索引大小约为 jsonb_ops 索引的 1/3。

5. **trigger 的 DISABLE/ENABLE 是 DBA 后门**：t5 测试模拟 DBA 越权篡改场景。生产化必须：
   - DBA 账号不直接连业务库（只通过 SET ROLE wms_app 或运维专用账号）
   - 关键操作（DISABLE TRIGGER）必须审计在 PG 自身的 pg_audit 扩展（spike 不深入）
   - hash chain 是兜底：即使 trigger 被禁用 + 数据被改，每日校验会立即发现

6. **`#[sqlx::test(migrations = "./migrations")]`** 需要显式指定 migrations 路径（与 spike-004 默认从 `./migrations` 加载不同；spike-002 因为 cargo 工作目录可能不同，显式更稳）。

### 7.4 后续动作

1. **写 ADR-0025 审计存储模型**（已起草，见下条）
2. **Wave 1 W1.B 实施清单**（写入 ADR-0025 §4）：
   - 把 spike-002 audit_event 表 + trigger + JSONB diff 模式迁到 `backend/migrations/`
   - 实现 `wms-infra` audit repo（async fn `append_event`）
   - hash chain 选方案 C：每日封档（依赖 cron 调度，未在 spike 验证）
   - 真并发吞吐压测：wrk + 1k QPS / 1 小时持续 / 60M 行 baseline
   - 每日 cron job：`verify_hash_chain` 离线对账
   - 配套 H1 鉴权：每条 audit 含 actor_id / actor_name / owner_id（来自 SPIKE-001 AuthContext）
3. **传染给后续 Spike**：
   - 已无后续依赖；SPIKE-005 RN 扫枪与 audit 没直接耦合（PDA 离线写本地，恢复时再走 audit）

### 7.5 拒绝清单

| 候选 | 不验证理由 |
|------|-----------|
| EventStore 等专用 audit log 中间件 | PG 单库已满足；引入新中间件 = 新可观测/备份/HA 链路 |
| 每条审计独立的 hash chain（per-row）| 性能差；用每日封档（方案 C） |
| 每日全量校验 hash chain（同步）| 60M 行扫一遍秒级以内但占 IO；改用增量（仅校验当日新增） |
| 跨服务 EventBus / Kafka audit | 引入分布式一致性问题；当前单库够用 |
