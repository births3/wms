# Wave 5 Retro：增值模块全面铺开

- 日期：2026-06-04
- 状态：开发完成；真实硬件 / TMS / 码上放心 / 灰度发布 evidence 移至 Wave 6
- 范围：M-PK 包装站、M8 连锁、M9 3PL 计费、M10 TMS+、owner 隔离、连锁端到端场景

---

## 1. 结论

Wave 5 开发完成。静态完成项、OpenAPI 契约、PostgreSQL migration、handler / repository、owner 隔离和一个连锁客户端到端场景均有当前仓库证据。

真实外部证据不伪造、不用 localhost / stub / mock / fake / example 替代，统一进入 Wave 6 预发布证据收口。

---

## 2. 已完成

| 项 | 交付 |
|----|------|
| W5.A M-PK | 包装工位、装箱任务、称重、面单打印 API；`packing_stations` / `packing_jobs` 表；OpenAPI schema |
| W5.B M8 | 门店补货建议、越库计划 API；`retail_replenishment_suggestions` / `crossdock_plans` 表；OpenAPI schema |
| W5.C M9 | 周期计费、计费明细、月结账单、账单确认 API；`billing_charge_calculations` / `billing_statements` / `billing_statement_charges` 表；OpenAPI schema |
| W5.D M10 | TMS 调度接收、在途温控、容器回收 API；`tms_dispatches` / `transit_temperature_readings` / `container_recoveries` 表；OpenAPI schema |
| owner 隔离 | 所有 Wave 5 写操作使用 `AuthContext.owner_id`，并由真实 PostgreSQL 测试覆盖 |
| 端到端场景 | 门店补货 → 越库 → 装箱 → 称重 → 面单 → TMS → 温控 → 容器回收 → 计费 → 月结确认 |

---

## 3. 验证

| 命令 | 结果 |
|------|------|
| `just wave-5-complete-check` | 通过 |
| `cargo fmt --check --all` | 通过 |
| `cargo check --manifest-path backend/Cargo.toml -p wms-api` | 通过 |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --lib -- --skip postgres_` | 64 passed |
| `cargo test --manifest-path backend/Cargo.toml -p wms-api --test wave5_postgres -- --nocapture` | 2 passed（临时 PostgreSQL） |
| `just openapi-check` | 通过 |
| `python3 -m pytest scripts/governance/tests/test_core_logic.py -q` | 112 passed |
| `just gov-t1` | 30/30 ok |
| `just task-check` | 6/6 ok |
| `git diff --check` | 通过 |

---

## 4. 运行时发现

真实 PostgreSQL 集成测试暴露并修复了两个问题：

1. M9 `calculate_period_charges` 不能用字符串直接与 PostgreSQL `DATE` 字段比较，已改为解析 `NaiveDate` 后绑定。
2. PostgreSQL `SUM(bigint)` 返回 `NUMERIC`，`generate_billing_statement` 已显式 cast 为 `BIGINT`。

这说明 Wave 5 的两份 runtime evidence 不是形式检查，确实覆盖了运行期 SQL 类型和事务行为。

---

## 5. 未关闭但不阻断开发完成

| Gate | 后续归属 |
|------|----------|
| M-PK 真实电子秤 / 蓝牙打印机 / 面单打印 evidence | Wave 6 W6.F |
| M10 真实 TMS dev/staging 推送、回调、失败重试 evidence | Wave 6 W6.G |
| M-TC “码上放心”真实 dev/staging evidence | Wave 6 W6.E |
| 首次试运行灰度发布 evidence | Wave 6 W6.H |

上述 gate 预发布前必须关闭，禁止用 local / mock / fake / stub / example 证据替代。

---

## 6. 依赖图复盘

Wave 5 依赖图基本准确：

- M-PK 依赖 W4.A 出库订单和 W4.B 冷链边界。
- M8 依赖 W4.A 出库订单。
- M9 自动计费依赖 W3.D 计费合同 / 规则和 W4.A 出库链路。
- M10 TMS+ 依赖 W4.A 出库订单和 W4.B 冷链边界。

需要新增的不是业务依赖，而是 Wave 6 证据依赖：稳定 dev/staging、硬件、TMS、码上放心账号和灰度发布链路。

---

## 7. 下一步

1. 按 ADR-0035 启动 Wave 6。
2. 先补 Wave 6 status / complete check，把所有真实 evidence gate 收口到一个报告。
3. 分组提交 Wave 4 / Wave 5 / Wave 6 closeout 变更，避免后续 PR 混杂。
