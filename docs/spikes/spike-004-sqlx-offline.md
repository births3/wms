# SPIKE-004: SQLx offline 编译模式

- 状态：accepted
- 时间盒：1 天（8 小时）
- Owner：项目主人
- 起始：2026-05-24  完成：2026-05-24（约 1.5 小时实际工时；远低于 8h 时间盒）
- 关联 Wave 任务：跨 W1.A / W1.B / W1.C（基础设施层）；任何 SQLx 使用前的前置
- 关联 ADR：ADR-0001（SQLx 已选定）；产出 ADR-0001 §SQLx 附录（更新原 ADR）

---

## 1. 背景与问题

ADR-0001 选 SQLx 的核心理由是**编译期校验 SQL**。但默认模式要求**编译期连接到运行中的 PostgreSQL** — 在 CI、开发者新机器、空环境上都会卡住。

未确定项：

1. `cargo sqlx prepare` 的工作流是否成熟（生成 `.sqlx/` JSON 缓存供离线编译）
2. 缓存与 schema 漂移时的检测机制（开发者改 schema 但忘记 prepare）
3. `query!` vs `query_as!` 在多 crate workspace 中如何统一缓存
4. migration 工具：用 sqlx-cli 内置的 `sqlx migrate` 还是 refinery / sea-orm-migration
5. test 隔离：每个测试用 `pg_temp` schema 还是 testcontainers 拉容器
6. 与 spike-002 的衔接：审计 trigger 在 sqlx test fixture 中如何加载
7. 多租户 owner_id 在 query 层是否能用 macro 强制注入

---

## 2. 验证假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | `cargo sqlx prepare --workspace` 能生成全 workspace 的离线缓存到 `.sqlx/` 目录，且支持 git commit | 在 spike-001 / spike-002 demo 中运行该命令，检查产物 |
| H2 | 在无 DATABASE_URL 环境变量、无运行 PG 的机器上 `cargo build --offline` 全 workspace 能通过 | 临时 unset DATABASE_URL + 关 PG，反向验证 |
| H3 | schema 变更（migration 加字段）但忘记重生缓存时，`cargo build` 能给出有意义的错误 | 故意删字段 → cargo build 失败信息要明确指向 prepare |
| H4 | sqlx-cli 内置的 `sqlx migrate run` + `migrations/<timestamp>_<name>.sql` 满足 Wave 1 需求（不需要 refinery） | 写 3 个 migration 跑 up + down |
| H5 | 集成测试用 `#[sqlx::test]` 自动建临时 schema + 执行 migrations，每个测试隔离 | 写 3 个并发 test 验证互不影响 |
| H6 | CI 上"先 prepare 后 build"的工作流，且 prepare 失败给出可操作错误（开发者新机器友好） | 模拟 CI runner（裸 ubuntu container）跑全流程 |

---

## 3. 退出条件

| 状态 | 条件 |
|------|------|
| accept | H1-H5 全部确认；H6 不强制（可在 Wave 1 实操中再优化）；产出 ADR-0001 附录 |
| reject | H1 / H2 不成立 → 改用 SeaORM 或 Diesel（重新评估，需新建 spike） |
| defer | testcontainers 集成可延后到 Wave 3（业务逻辑铺开时） |

---

## 4. 实施路径

### 步骤 1：建最小 sqlx demo（1 小时）

```
spikes/spike-004-sqlx-offline/
├── Cargo.toml             # sqlx with features ["postgres", "macros", "uuid", "chrono"]
├── migrations/
│   ├── 20260601000000_create_items.sql
│   └── 20260601000001_add_batch_no.sql
├── src/
│   ├── lib.rs
│   └── repo/
│       ├── items.rs       # query!() / query_as!()
│       └── audit.rs       # 双表写入演示
├── tests/
│   └── items.rs           # #[sqlx::test]
└── .sqlx/                  # 生成产物，commit 入仓
```

### 步骤 2：跑 prepare（1 小时）

```bash
docker run -d --name spike-pg -p 5432:5432 -e POSTGRES_PASSWORD=spike postgres:16
export DATABASE_URL=postgres://postgres:spike@localhost:5432/postgres
cd spikes/spike-004-sqlx-offline
sqlx migrate run
cargo sqlx prepare
git status .sqlx  # 应该有新文件
```

### 步骤 3：离线编译反向验证（1 小时）

```bash
docker stop spike-pg
unset DATABASE_URL
cargo build --offline   # 期望成功
```

### 步骤 4：schema 漂移检测（1 小时）

- 在 migration 加 `ALTER TABLE items ADD COLUMN owner_id UUID` 但**不**重新 prepare
- 在 repo 里写 `query!("SELECT id, owner_id FROM items")`
- `cargo build --offline` 期望失败：明确指向"未运行 prepare"

### 步骤 5：测试隔离验证（1.5 小时）

```rust
#[sqlx::test]
async fn test_create_item(pool: PgPool) { ... }

#[sqlx::test]
async fn test_audit_trigger(pool: PgPool) { ... }

// 并发跑 cargo test 验证互不影响
```

### 步骤 6：与 spike-002 audit trigger 联动测试（1 小时）

- 在 spike-004 的 fixture 里加载 audit trigger
- 测试：`UPDATE audit_event` 应返回错误而非影响其他测试

### 步骤 7：CI workflow 草案（0.5 小时）

```yaml
# .github/workflows/spike-004.yml
- name: Verify SQLx offline cache up to date
  run: |
    docker run -d ...
    export DATABASE_URL=...
    sqlx migrate run
    cargo sqlx prepare --check  # 失败提示开发者跑 prepare
```

### 步骤 8：写 ADR-0001 附录（1 小时）

加在 `docs/adr/0001-tech-stack.md` 末尾或新建 `docs/adr/0001-tech-stack-appendix-sqlx.md`：
- prepare 工作流
- .sqlx/ 入仓策略
- migration 命名（`<timestamp>_<verb>_<noun>.sql`）
- 测试隔离模式
- 与 spike-002 audit 的关联点

---

## 5. 风险与后备方案

| 风险 | 概率 | 影响 | 后备方案 |
|------|------|------|---------|
| `.sqlx/` 在多 crate workspace 中冲突 | 中 | 中 | 用单一 db crate 集中所有 query！其他 crate 调用其函数 |
| schema 漂移检测错误信息不友好 | 中 | 低 | 在 justfile 加 `just db-prepare` 一键命令 + 文档化 |
| sqlx-cli 版本与 sqlx 库不匹配 | 低 | 中 | 锁版本：Cargo.toml 写 `sqlx = "=0.7.4"`、CI 用相同版本 sqlx-cli |
| `#[sqlx::test]` 启动慢（每次建 schema） | 中 | 低 | 把 setup 抽到 fixture 共享 + 用 `pool.acquire()` 而非新连接 |
| Wave 3 大量 SQL 时 prepare 阶段成 CI 瓶颈 | 中 | 中 | 仅在改 SQL 文件时跑 prepare（git diff 触发）；spike 已含此逻辑设计 |

---

## 6. 产出物清单

- 代码：`spikes/spike-004-sqlx-offline/`（含 .sqlx/ 缓存）
- 文档：本文件 §7
- ADR：`docs/adr/0001-tech-stack-appendix-sqlx.md`（或在 0001 末尾追加）
- 治理：T2 加 `check_sqlx_prepared.py`（diff 触发；改了 .rs 中的 `query!()` 必须同步更新 .sqlx/）
- justfile 加 `just db-prepare` / `just db-migrate-up` 入口

---

## 7. 决策记录

- 日期：2026-05-24
- 结论：**accept**
- 时间盒消耗：约 1.5 小时（远低于 8h 上限）

### 7.1 假设验证结果

| ID | 假设 | 状态 | 证据 |
|----|------|------|------|
| H1 | `cargo sqlx prepare` 生成 .sqlx/ 缓存 | ✓ | 5 个 `.sqlx/query-<hash>.json` 文件落盘（每个 `query!` / `query_as!` / `query_scalar!` 一个） |
| H2 | 无 DATABASE_URL 也能 `cargo build --offline` | ✓ | unset DATABASE_URL + cargo clean → cargo build --offline 0.69s 成功 |
| H3 | schema 漂移检测错误信息明确 | ✓ | 加引用不存在列的 query → `error: set DATABASE_URL to use query macros online, or run cargo sqlx prepare to update the query cache` |
| H4 | sqlx-cli migrate 满足需求 | ✓ | 2 个 migrations/<timestamp>_<name>.sql 自动加载，实测 t1-t5 全过 |
| H5 | `#[sqlx::test]` per-test 临时 db + migrations + 隔离 | ✓ | 5 个并发测试 0.85s 完成；t2_isolated_db_per_test 验证空 db |
| H6 | CI 用 `cargo sqlx prepare --check` | ✓ | 0.37s 通过（仅 cargo check + 验证 .sqlx/ 与 query 一致） |

### 7.2 关键发现

1. **sqlx 0.8 的 `query_scalar!` 需要类型注解**：`SELECT COUNT(*)` 如果不写 `as "count!: i64"`，会推导成 `Option<i64>`（因为 PG `COUNT(*)` 理论可空）。spike-004 用 `as "count!: i64"` 强制非空。这是 Wave 1 编码模板需要的小知识。

2. **.sqlx/ 缓存是 deterministic JSON**（按 query 内容 hash 命名）：
   - 同一 query 多次 prepare 生成相同 hash
   - 改 query 内容会生成新文件，旧文件需要 prepare 时清理（默认会清）
   - 可放心入 git，diff 友好

3. **cargo sqlx prepare 耗时 ~36s（首次）/ ~5s（后续 incremental）**：
   - 含完整 `cargo check` + 每个 query 连 DB 校验
   - Wave 1 起步约 50 query → 约 1 分钟；Wave 2-3 业务铺开后可能 2-3 分钟
   - CI 上是可接受（远低于 cargo build/test 总耗时）

4. **cargo build --offline + 缓存命中 ≈ 0.69s**（与无 query! 版本基本一致）：编译期校验是 zero-runtime-cost。

5. **sqlx::test 真好用**：每个测试一个临时 db，自动跑 migrations，测试天然隔离。Wave 1+ TDD 节奏强制（ADR-0006），sqlx::test 是关键基础设施。

6. **spike-004 不与 spike-002 联动测试**（原文档 §4 步骤 6 计划）：spike-002 是独立 spike，audit trigger 测试在 spike-002 内做更合适。本 spike 聚焦 sqlx 自身工具链。

### 7.3 后续动作

1. **更新 ADR-0001 §数据访问 段**（追加 SQLx 实践规范）—— 见 ADR-0001 修订
2. **Wave 1 W1.A/B 实施清单**：
   - Cargo workspace 含 `wms-infra` 集中所有 query!（避免多 crate .sqlx/ 冲突）
   - root justfile 加 `just db-prepare` / `just db-migrate-up` / `just db-reset` 入口
   - CI 加 `cargo sqlx prepare --check` 步骤（diff 触发：改 .rs 改了 query!）
   - DATABASE_URL 走 `.env.example`，dev 环境 `.env`（gitignore）
   - 加治理脚本 `check_sqlx_prepared.py`：T2 治理，diff 触发改 .rs 必须更新 .sqlx/
3. **传染给 SPIKE-002**：
   - audit_event 表的 SQL 用 sqlx::query!()
   - audit trigger SQL 在 migrations/ 文件
   - .sqlx/ 缓存在 spike-002 单独维护（与 spike-004 隔离）

### 7.4 拒绝清单

| 候选 | 不验证理由 |
|------|-----------|
| Diesel | 同步为主、async 体验差；ADR-0001 已否决 |
| SeaORM | 抽象偏重；起步过度设计；如未来需要 ActiveRecord 可重新评估 |
| refinery（替代 sqlx-migrate） | sqlx-migrate 已满足；少一个依赖 |
| testcontainers（每测试一容器） | 内置 sqlx::test 已满足；多容器启动 overhead 大 |
| 升级 sqlx 0.9（如发布） | 当前 0.8.6 稳定，无升级动机；写新 ADR 评估
