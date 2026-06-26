# Wave 6 Evidence Preflight Runbook

> 用途：在真实 dev/staging、硬件、TMS、码上放心和灰度发布环境到位前，先确认 Wave 6 证据收口链路、命令入口和外部资源清单完整。本文档不会写入 runtime evidence，也不能关闭 gate。

## 完成边界

`just wave-6-evidence-preflight` 只证明以下事项：

- 8 个 gate 的 runbook、evidence 文件名、readiness / record / validate 命令已登记。
- `justfile` 入口和底层执行文件存在，后续环境到位后可以直接执行对应命令。
- 每个 gate 都保留 dev/staging、environment 标记和禁止 local/mock/fake/stub/example/prod/production 的边界。
- 每个 gate 都保留模板占位拒绝边界：`YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认` 不能进入真实 evidence。
- 所有 evidence 写入器默认拒绝覆盖已有文件；`--force` 只用于人工确认的异常修正路径，正常 closeout 与 runbook 示例不得使用。
- Wave 6 closeout runbook 已把 preflight 纳入执行顺序。

它不证明：

- dev/staging 已稳定。
- PDA、电子秤、蓝牙打印机、面单打印机已接入。
- TMS 或“码上放心”已经开通并返回真实回执。
- 灰度发布、回滚和审批已经执行。

上述真实证据仍必须写入对应 `docs/retros/*.json`，并通过各自 validator。

## 一键预检

```bash
just wave-6-evidence-preflight
```

推荐在以下场景运行：

- dev/staging 环境准备前，确认缺口只剩外部状态。
- 任一 Wave 6 runbook、validator、record 脚本或 justfile 变更后。
- 真实采集 evidence 前，避免到现场才发现命令入口缺失。
- 写 Wave 6 retro 前，确认所有收口文档和命令入口仍一致。

`python3 scripts/governance/check_wave6_evidence_preflight.py --json` 的机器消费字段以以下 key 为准：

- `error_count` / `top_error_count` / `gate_error_count`：总错误、顶层错误、gate 内部错误数量；`error_count` 必须等于后两者之和。
- `error_details` / `top_error_details` / `gate_error_details`：机器可消费的结构化错误，包含 `scope`、`gate_id` 和 `message`。
- `schema_version`：JSON 合同版本，当前为 `1`。
- `mode`：固定为 `static-preflight`，表示只做静态预检。
- `writes_runtime_evidence`：固定为 `false`，表示本脚本不会写入 runtime evidence。
- `closes_gate`：固定为 `false`，表示本脚本不能关闭任何 evidence gate。
- `preflight_command`：机器可复现本次静态预检 JSON 的完整命令。
- `gate_count` / `ok_gate_count` / `failed_gate_count`：8 个 gate 的总数、通过数量和失败数量；`ok_gate_count + failed_gate_count` 必须等于 `gate_count`。
- `evidence_gate_ids`：W6.A 到 W6.H 的 gate ID 清单，必须与 Wave 6 report 对齐。
- `evidence_gate_evidence_files`：8 个真实 evidence JSON 文件路径清单。
- `evidence_gate_runbooks`：每个 gate 对应的 runbook 路径清单。
- `evidence_gate_just_entries`：每个 gate 对应的 readiness / record-check-only / record / validate just 入口映射。
- `evidence_gate_execution_files`：每个 gate 的 readiness / record-check-only / record / validate just 入口推导出的底层执行文件映射。
- `required_top_level_files`：preflight 要求存在的顶层收口文件清单。
- `required_runbooks`：8 个 gate 需要的 runbook 去重清单。
- `required_execution_files`：preflight 要求存在的底层执行文件清单；每个 gate 的 readiness / record-check-only / record / validate 入口能推导出的执行文件必须包含在此清单内。
- `overwrite_guard_execution_files`：必须具备默认防覆盖保护的 evidence 写入器清单。
- `overwrite_guard_required_markers`：preflight 静态检查 evidence 写入器时要求出现的防覆盖标记。
- `gate_commands_by_phase`：每个 gate 按 readiness / record_check_only / record / validate 分组后的完整 `just ...` 命令。
- `validation_commands`：所有 evidence validator 的完整 `just ...` 命令清单。
- `closeout_just_entries`：最终收口命令入口清单，包含状态报告、写 retro 前 evidence 检查、缺失 evidence 命令清单和 complete-check。
- `top_errors`：顶层收口链路错误，例如缺少总入口文件、closeout 入口或范围文档登记。
- `gate_errors`：具体 W6.A 到 W6.H gate 内部错误，例如 runbook 缺字段、just 入口缺失或示例引用违规。
- `failed_gate_ids`：未通过 preflight 的 gate ID 清单；长度必须等于 `failed_gate_count`。
- `failed_gates`：未通过 gate 的结构化条目，含 `gate_id`、`title` 和 `errors`。
- `gate_specs`：W6.A 到 W6.H 的静态规格清单，含 evidence 文件、runbook、just 入口、执行文件、必备术语和禁用边界。
- `gates`：本次检查的 gate 结果清单，含 `gate_id`、`title`、`ok` 和 `errors`。

## Gate 矩阵

| Gate | Evidence 文件 | Runbook | record / readiness 入口 | validate 入口 |
|------|---------------|---------|-------------------------|---------------|
| W6.A | `docs/retros/wave-1-h2-runtime-evidence.json` | `docs/runbooks/wave-1-runtime-evidence.md` | `just wave-1-runtime-prereq-h2` / `just wave-1-h2-runtime-readiness` / `just wave-1-h2-runtime-evidence` | `just wave-1-runtime-evidence-validate` |
| W6.B | `docs/retros/wave-1-runtime-evidence.json` | `docs/runbooks/wave-1-runtime-evidence.md` | 二选一：k8s 路径运行 `just wave-1-runtime-prereq-rollback-k8s` / `just wave-1-rollback-runtime-readiness-k8s` / `just wave-1-rollback-runtime-evidence-k8s`；docker-compose 路径运行 `just wave-1-runtime-prereq-rollback-compose` / `just wave-1-rollback-runtime-readiness-compose` / `just wave-1-rollback-runtime-evidence-compose`；两者共用 `just wave-1-runtime-evidence-validate`，不得两组都跑来补 evidence | `just wave-1-runtime-evidence-validate` |
| W6.C | `docs/retros/wave-2-runtime-evidence.json` | `docs/runbooks/wave-2-runtime-evidence.md` | `just wave-2-runtime-evidence-readiness` / `just wave-2-runtime-evidence-smoke`；手动备选 `just wave-2-runtime-evidence-record` | `just wave-2-runtime-evidence-validate` |
| W6.D | `docs/retros/wave-3-pda-runtime-evidence.json` | `docs/runbooks/wave-3-pda-readiness.md` | `just wave-3-pda-preaudit-kit` / `just wave-3-pda-materials-checklist` / `just wave-3-pda-field-work-request` / `just wave-3-pda-field-execution-summary` / `just wave-3-pda-field-precheck-summary` / `just wave-3-pda-field-owner-gap-actions` / `just wave-3-pda-field-handoff-bundle` / `just wave-3-pda-evidence-package-template` / `just wave-3-pda-intake-template` / `just wave-3-pda-intake-check` / `just wave-3-pda-intake-record` / `just wave-3-pda-service-precheck` / `just wave-3-pda-trace-code-openapi-precheck` / `just wave-3-pda-runtime-readiness` / `just wave-3-pda-runtime-evidence-record` | `just wave-3-pda-runtime-evidence-validate` |
| W6.E | `docs/retros/wave-4-external-dependencies.json` | `docs/runbooks/wave-4-external-dependencies.md` | `just wave-4-external-dependencies-readiness` / `just wave-4-external-dependencies-record` | `just wave-4-external-dependencies-validate` |
| W6.F | `docs/retros/wave-5-hardware-evidence.json` | `docs/runbooks/wave-5-hardware-evidence.md` | `just wave-5-hardware-materials` / `just wave-5-hardware-readiness` / `just wave-5-hardware-evidence-record` | `just wave-5-hardware-evidence-validate` |
| W6.G | `docs/retros/wave-5-tms-evidence.json` | `docs/runbooks/wave-5-tms-evidence.md` | `just wave-5-tms-materials` / `just wave-5-tms-readiness` / `just wave-5-tms-evidence-record` | `just wave-5-tms-evidence-validate` |
| W6.H | `docs/retros/wave-6-deploy-evidence.json` | `docs/runbooks/wave-6-deploy-evidence.md` | `just wave-6-deploy-materials` / `just wave-6-deploy-readiness` / `just wave-6-deploy-audit` / `just wave-6-deploy-evidence-record` | `just wave-6-deploy-evidence-validate` |

## 外部资源清单

| Gate | 采集前必须到位 | 最小证据引用 |
|------|----------------|--------------|
| W6.A | dev PostgreSQL、最新 migration、60M `audit_event` 基线、wrk、7 天 seal cron 0 失败 | wrk 原始日志、seal cron 日志、DB readiness 输出 |
| W6.B | dev/staging k8s 或 docker-compose、真实 smoke gate 或 Prometheus rollback 信号、上一稳定版本 | rollback 日志、smoke 或 Prometheus 触发日志 |
| W6.C | dev/staging `wms-api`、H1 鉴权、配置中心、W1 文件版 flag 快照、审计链路 | smoke 日志、reconcile 日志、旧文件归档引用 |
| W6.D | 真 PDA、实体扫码键、dev/staging M2/M3 API、离线 replay 条件、幂等 replay 条件、L7 执行环境、人工易用性走查人 | PDA 资产引用、扫码日志、离线 replay 日志、idempotency replay 日志、audit_event 查询、L7 执行记录、走查记录 |
| W6.E | “码上放心”账号 / 租户、正式接口文档、鉴权方式、错误码、频率限制、真实测试环境 | 文档归档、Vault 凭证引用、成功回执、失败重试日志、audit_event 查询 |
| W6.F | 电子秤、蓝牙打印机、面单打印机、校准记录、dev/staging 包装站工位 | 设备资产引用、校准记录、称重日志、打印产物、audit_event 查询 |
| W6.G | TMS dev/staging endpoint、回调鉴权、调度结果格式、失败重试条件、Vault 凭证引用 | 推送日志、回调日志、失败重试日志、audit_event 查询 |
| W6.H | staging 发布环境、release plan、构建产物、灰度配置、smoke gate、dashboard、回滚链路、双人审批 | 发布计划、artifact、灰度配置、smoke、dashboard、rollback、审批、audit_event 查询 |

## 执行顺序

1. 当前阶段先运行：
   ```bash
   just wave-6-evidence-preflight
   just wave-6-status
   just wave-6-missing-evidence-commands
   ```
   `just wave-6-missing-evidence-commands` 是只读命令清单；当前缺 evidence 时会因 `--strict --evidence-only` 返回非零，非零仅表示仍有待采集 gate，不代表写入 runtime evidence 或关闭 gate。
2. 外部资源逐项到位后，只运行对应 gate 的 materials / readiness / record / validate 命令。
3. 每次 record 前检查证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`）；W6.H 是 staging 灰度发布 gate，必须使用 `staging` 标记，不能用 `dev` evidence 关闭。
4. 禁止把 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example` 放进真实 evidence 引用。
5. 禁止把模板占位保留在真实 evidence 引用中，包括 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`。
6. record / evidence 命令默认不得覆盖已有 `docs/retros/*.json`；如需修正已写 evidence，必须按人工确认的异常修正路径显式追加 `--force` 并保留 superseded 原因。
7. W6.A-W6.H 8 个 evidence gate 均已通过各自 validator（其中 W6.A/W6.B 共用 `just wave-1-runtime-evidence-validate`）后，再运行：
   ```bash
   just wave-6-evidence-check
   ```
8. `just wave-6-evidence-check` 通过后才写 `docs/retros/wave-6-retro.md`，再跑 `just wave-6-complete-check`。

## 采购 / 对接待办

- 稳定 dev/staging 环境：API URL、PostgreSQL、CI、证据库路径、Vault 路径和可观测 dashboard 命名必须统一包含 `dev` 或 `staging`。
- PDA：至少 1 台真实设备，记录资产编号、系统版本、扫码键模式、离线网络切换方式。
- M-PK 硬件：电子秤、蓝牙打印机、面单打印机都要有设备型号、资产编号、校准或测试记录。
- TMS：确认 dev/staging endpoint、回调签名、失败码、重试策略和调度结果字段。
- 码上放心：确认账号、租户、正式接口文档、鉴权方式、错误码、频率限制和测试回执查询方式。
- 灰度发布：确认 docker-compose 或 Kubernetes 路径、release approval 流程、rollback drill 责任人和审计查询方式。
