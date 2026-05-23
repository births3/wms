# SPIKE-004: SQLx offline 编译模式

- 状态：起草
- 时间盒：1 天（8 小时）
- Owner：项目主人
- 起始：— 完成：—
- 关联 Wave 任务：跨 W1.A / W1.B / W1.C（基础设施层）；任何 SQLx 使用前的前置
- 关联 ADR：ADR-0001（SQLx 已选定）

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

> spike 完成后填写。

- 日期：—
- 结论：—
- 关键发现：—
- 后续动作：—
