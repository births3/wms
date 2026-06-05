# Wave 1 收口记录

- 日期：2026-06-02
- 状态：开发完成；真实 runtime evidence 移至预发布 gate
- 范围：H1/H2/H3、文件版 Feature Flag、回滚演练资产、web-admin 壳工程、W1.F/G/H + H-FILE 契约联合评审

> 2026-06-03 口径更新：当前尚无稳定 dev/staging 环境，两份真实 runtime evidence 不再阻塞 Wave 1 开发完成；它们保留为预发布 gate，禁止用 localhost、stub、mock、fake、example 或 prod 证据替代。

---

## 1. 当前进度

| 项 | 当前证据 | 出口状态 |
|----|----------|----------|
| W1.A H1 | 已有 `AuthContext`、JWT claims、`owner_id` 隔离、非 auth handler 挂接测试；已补 Redis blacklist / `permissions_changed_at` runtime adapter 与 AUTH-004 / AUTH-009 单测 | 静态代码证据完成 |
| W1.B H2 | 已有 `audit_event` migration、append-only trigger、只读 `audit_chain_seal` 表、内存版 `commit_with_audit` helper、真实 PostgreSQL `append_event(pool, req)`、链头锁、当前月/下月分区维护函数、Rust 封档 helper（先校验 hash chain 再 INSERT seal）、hash chain 单测；真实 PostgreSQL append/seal 集成测试已通过 | 开发完成；真实 dev 压测与 7 天封档证据移至预发布 gate，实际证据需落 `docs/retros/wave-1-h2-runtime-evidence.json` |
| W1.C H3 | `openapi-export` 生成 `shared/openapi/openapi.json`，`@wms/api-client` 消费 | 完成 |
| W1.D | 文件版 `deploy/feature_flags.toml` + 后端 reader + `check_feature_flags.py`；`wave1_auto_rollback_probe.sh` 已改为真实 HTTP smoke / Prometheus 入口 | 开发完成；真实 dev/staging smoke gate 或监控触发证据移至预发布 gate |
| W1.E | `apps/web-admin` 接入 `@wms/ui` 与 `@wms/api-client`，H1/H2/H3 壳工程呈现 | 完成 |
| W1.F/G/H + H-FILE | ADR-0030/0031/0032/0033 已 Accepted，依赖图已登记 | 契约段完成 |

## 2. 已验证证据

| 项 | 证据 |
|----|------|
| H1 handler 挂接 | `backend/crates/api/src/auth.rs` 中 `AuthContext` 实现 `FromRequestParts`；`auth_context_extractor_is_demo_items_handler_compatible` 证明非 auth handler 可挂 H1 |
| H1 TTL / 隔离 / 失效 | `ACCESS_TOKEN_TTL_SECONDS = 3600`；claims 与 `AuthContext` 使用 `owner_id`，不引入 RLS；`AuthRuntimePolicy` 接入 Redis blacklist 与 `permissions_changed_at`，覆盖 AUTH-004 / AUTH-009 |
| H2 schema 起点 | `backend/migrations/202606020001_audit_event.sql` 固化 `audit_event` / trigger / `audit_chain_seal` 只读 trigger / `create_current_partition()` / `create_next_partition()`；已补 `audit_event_id_seq` 授权 |
| H2 helper 起点 | `backend/crates/api/src/audit.rs` 提供内存版 `append_event` 与 `commit_with_audit`；新增真实 PostgreSQL `append_event(pool, req)` 与 `seal_audit_chain(pool, date, sealed_at)`；hash 覆盖 canonical row，含 diff before/after；重复 seal 不覆盖旧记录 |
| H2 真实 PG 集成 | `backend/crates/api/tests/audit_postgres.rs` 通过 `#[sqlx::test]` 跑真实 PostgreSQL migration、append 两条审计事件、封档、重复封档拒绝；本地验证命令：`DATABASE_URL=postgres://wms_wave1_test:***@127.0.0.1:5434/postgres cargo test` |
| H2 runtime 证据格式 | `docs/retros/wave-1-h2-runtime-evidence.example.json` 给出格式；实际通过文件必须命名为 `docs/retros/wave-1-h2-runtime-evidence.json`，包含 dev、60M baseline、wrk 1k QPS × 3600s、P99 < 200ms、7 天封档 cron 0 失败 |
| W1.D runtime 入口 | `deploy/scripts/wave1_auto_rollback_probe.sh` 现在要求 `--smoke-url` 或 `PROMETHEUS_URL + PROMETHEUS_QUERY`；真实信号失败时才进入 `wave1_rollback.sh --execute`；缺少真实信号配置时退出非 0 |
| W1.C | `shared/openapi/openapi.json` 与 `packages/api-client/src/schema.ts` 已生成 |

## 3. dev/staging 回滚演练边界

当前脚本不再注入 `kubectl` / `docker` stub，也不再用 forced failure 冒充自动回滚证据。`deploy/scripts/wave1_auto_rollback_probe.sh` 仅在提供真实 dev/staging 信号时执行：

- HTTP smoke：`--smoke-url` 或 `SMOKE_URL`，2xx/3xx 视为健康；真实模式拒绝 localhost / 127.0.0.1 / 0.0.0.0
- Prometheus：`PROMETHEUS_URL` + `PROMETHEUS_QUERY`（或对应 CLI 参数），两者都必须包含当前 `environment` 标记；PromQL 结果 `0` 视为健康，`> 0` 触发回滚
- 缺少真实信号配置、边界命中 `prod/production/prodution`、或任一 evidence 引用环境标记不含当前 `dev/staging` 时，脚本直接退出非 0

结论：这仍然不是可发布前所需的真实自动回滚链路证据。必须在稳定 dev/staging 接入 smoke gate 或监控信号后复跑，并以 `docs/retros/wave-1-runtime-evidence.json` 记录非本机 signal URL、rollback log 引用、外部日志引用、触发结果与退出码，再回写 ADR-0016 v3.2 的阈值校准结果。

## 4. 四横向契约联合评审

| 契约 | 结论 |
|------|------|
| H-INT | 外部对接必须复用 ADR-0018 弹性、M-PM 字段规整、ADR-0013 凭证管理，并写 H2 审计 |
| H-FILE | 附件能力按 ADR-0031 作为共享能力接入，文件操作需要写 H2 审计 |
| H-APV | 审批统一经 H-APV 端口，审批留痕携带 `approval_source` / `approval_id` |
| H-SCH | 系统级定时任务由 H-SCH 注册，调度执行与失败重试写 H2 审计 |

联合核对结论：H-INT / H-FILE / H-APV / H-SCH 都依赖 H2 审计表承载 actor、owner、resource、action、diff 与 trace 信息。`approval_source` 暂不作为 ADR-0025 一等列进入 W1.B schema；审批相关信息在 Wave 1 通过 diff / details 进入审计事件，后续若要升为一等字段，需要单独 ADR 或 H2 schema 修订。

## 5. 预发布 Gate

| Gate | 需要补齐 |
|------|----------|
| H2 PG runtime | 按 [Wave 1 Pre-release Runtime Evidence Runbook](../runbooks/wave-1-runtime-evidence.md) 执行 wrk 1k QPS × 1 小时压测、dev 7 天封档 cron 验证；写入 `docs/retros/wave-1-h2-runtime-evidence.json` |
| W1.D 自动回滚 | 按 [Wave 1 Pre-release Runtime Evidence Runbook](../runbooks/wave-1-runtime-evidence.md) 用真实 dev/staging smoke gate 或监控信号触发回滚，不只 stub |

这些 gate 不阻塞 `just wave-1-complete-check` 的开发完成判定；预发布前必须用 `just wave-1-runtime-evidence-validate` 单独验证通过。

已清理的口径漂移：

| 项 | 处理 |
|----|------|
| H1 token TTL / RLS | ADR-0024 v0.3 与 H1 用户故事已对齐：Access Token 1h、Refresh Token 24h、Wave 1 使用 `AuthContext.owner_id` 隔离，PostgreSQL RLS 延后评估 |

## 6. 下一步

1. 运行 `just wave-1-complete-check` 作为 Wave 1 开发完成判定。
2. 有稳定 dev/staging 后，按 [Wave 1 Pre-release Runtime Evidence Runbook](../runbooks/wave-1-runtime-evidence.md) 执行 ADR-0025 要求的 wrk 1k QPS × 1 小时压测和 dev 7 天封档 cron 验证。
3. 按同一 runbook 在 dev/staging 接入 smoke gate 或 Prometheus 信号，复跑自动回滚演练，并在预发布前完成 `just wave-1-runtime-evidence-validate`。
