# Wave 6 Closeout Runbook

> 用途：集中关闭 Wave 6 预发布证据 gate。本 runbook 只定义执行顺序和证据记录命令，不能替代真实 dev/staging、真设备、真实外部系统或灰度发布证据；`docs/retros/wave-6-retro.md` 写入后，Wave 6 完成必须以 `just wave-6-complete-check` 通过为准。

## 完成口径

Wave 6 完成需要以下全部条件成立：

0. `just wave-6-evidence-preflight` 退出 0，确认收口链路、命令入口和 runbook 清单完整。
1. `just wave-6-evidence-check` 退出 0，确认 W6.A-W6.H 8 个真实 evidence gate 均已通过各自 validator（其中 W6.A/W6.B 共用 `just wave-1-runtime-evidence-validate`）。
2. `docs/retros/wave-6-retro.md` 已写入本轮真实 evidence 结果和剩余风险。
3. `just wave-6-status` 无阻塞缺口。
4. `just wave-6-complete-check` 退出 0。
5. `just gov-t1`、`just task-check`、`git diff --check` 通过。

禁止用 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example` 证据替代真实 dev/staging 或外部系统证据。

所有 evidence 引用还必须包含当前 `environment` 标记（`dev` 或 `staging`）；缺少环境标记的证据库路径、CI 记录、Vault 引用或审批票据不能关闭 gate。W6.H 是首次试运行灰度发布 gate，必须使用 `staging` evidence，不能用 `dev` evidence 关闭。

环境、硬件或外部系统尚未到位时，先按 [Wave 6 Evidence Preflight Runbook](wave-6-evidence-preflight.md)（`docs/runbooks/wave-6-evidence-preflight.md`）执行：

```bash
just wave-6-evidence-preflight
```

该命令只验证 runbook、`justfile` 和 validator / record 链路完整，不会写入 runtime evidence，不能替代下表 8 个真实 gate。

## 当前 Gate

| Gate | Evidence 文件 | 记录入口 | 验证入口 |
|------|---------------|----------|----------|
| W6.A | `docs/retros/wave-1-h2-runtime-evidence.json` | `just wave-1-h2-runtime-evidence` | `just wave-1-runtime-evidence-validate` |
| W6.B | `docs/retros/wave-1-runtime-evidence.json` | `just wave-1-rollback-runtime-evidence-k8s` 或 `just wave-1-rollback-runtime-evidence-compose` | `just wave-1-runtime-evidence-validate` |
| W6.C | `docs/retros/wave-2-runtime-evidence.json` | `just wave-2-runtime-evidence-readiness` / `just wave-2-runtime-evidence-smoke`；手动备选 `just wave-2-runtime-evidence-record` | `just wave-2-runtime-evidence-validate` |
| W6.D | `docs/retros/wave-3-pda-runtime-evidence.json` | `just wave-3-pda-preaudit-kit` / `just wave-3-pda-materials-checklist` / `just wave-3-pda-field-work-request` / `just wave-3-pda-field-execution-summary` / `just wave-3-pda-field-precheck-summary` / `just wave-3-pda-field-owner-gap-actions` / `just wave-3-pda-field-handoff-bundle` / `just wave-3-pda-evidence-package-template` / `just wave-3-pda-intake-template` / `just wave-3-pda-intake-check` / `just wave-3-pda-intake-record` / `just wave-3-pda-service-precheck` / `just wave-3-pda-trace-code-openapi-precheck` / `just wave-3-pda-runtime-readiness` / `just wave-3-pda-runtime-evidence-record` | `just wave-3-pda-runtime-evidence-validate` |
| W6.E | `docs/retros/wave-4-external-dependencies.json` | `just wave-4-external-dependencies-readiness` / `just wave-4-external-dependencies-record` | `just wave-4-external-dependencies-validate` |
| W6.F | `docs/retros/wave-5-hardware-evidence.json` | `just wave-5-hardware-materials` / `just wave-5-hardware-readiness` / `just wave-5-hardware-evidence-record` | `just wave-5-hardware-evidence-validate` |
| W6.G | `docs/retros/wave-5-tms-evidence.json` | `just wave-5-tms-materials` / `just wave-5-tms-readiness` / `just wave-5-tms-evidence-record` | `just wave-5-tms-evidence-validate` |
| W6.H | `docs/retros/wave-6-deploy-evidence.json` | `just wave-6-deploy-materials` / `just wave-6-deploy-readiness` / `just wave-6-deploy-audit` / `just wave-6-deploy-evidence-record` | `just wave-6-deploy-evidence-validate` |

## 推荐执行顺序

### 1. Wave 1 H2 runtime evidence

先确认 dev PostgreSQL、wrk 输出、7 天 seal cron 证据均到位：

```bash
just wave-1-runtime-prereq-h2
just wave-1-h2-runtime-readiness
just wave-1-h2-runtime-evidence
just wave-1-runtime-evidence-validate
```

### 2. Wave 1 W1.D rollback evidence

按实际部署形态二选一，不得用 local/prod/production/mock/fake/stub/example 证据兜底：

```bash
just wave-1-runtime-prereq-rollback-k8s
just wave-1-rollback-runtime-readiness-k8s
just wave-1-rollback-runtime-evidence-k8s
```

或：

```bash
just wave-1-runtime-prereq-rollback-compose
just wave-1-rollback-runtime-readiness-compose
just wave-1-rollback-runtime-evidence-compose
```

任一部署形态的 record 命令写入真实 evidence 后，再运行：

```bash
just wave-1-runtime-evidence-validate
```

### 3. Wave 2 config-center Feature Flag evidence

按 [Wave 2 Pre-release Runtime Evidence Runbook](wave-2-runtime-evidence.md) 完成真实 smoke、reconcile 和旧文件归档后，运行：

```bash
just wave-2-runtime-evidence-readiness
just wave-2-runtime-evidence-smoke
just wave-2-runtime-evidence-validate
```

`just wave-2-runtime-evidence-record` 仅作为外部 smoke 已完成后的手动备选写入路径，正常 closeout 使用 `just wave-2-runtime-evidence-smoke`。

### 4. Wave 3 PDA / L7 evidence

真 PDA、实体扫码键、M2/M3 dev/staging 日志、离线 replay、幂等 replay、审计查询和易用性走查全部到位后，运行：

```bash
just wave-3-pda-preaudit-kit --json
just wave-3-pda-materials-checklist --json
just wave-3-pda-field-work-request
just wave-3-pda-field-execution-summary --json
just wave-3-pda-service-precheck --from-env --json
just wave-3-pda-trace-code-openapi-precheck --from-env --json
just wave-3-pda-field-precheck-summary --from-env --json
just wave-3-pda-field-owner-gap-actions --json
just wave-3-pda-field-handoff-bundle --json
just wave-3-pda-evidence-package-template
just wave-3-pda-intake-template --json

# from-env 正式 record 路径
just wave-3-pda-runtime-readiness --from-env --json
just wave-3-pda-runtime-evidence-record --from-env --check-only --json
just wave-3-pda-runtime-evidence-record --from-env --json

# intake 正式 record 路径；与 from-env 正式 record 二选一
just wave-3-pda-intake-check --json
just wave-3-pda-intake-record --json

just wave-3-pda-runtime-evidence-validate
```

`just wave-3-pda-preaudit-kit --json`、`just wave-3-pda-materials-checklist --json`、`just wave-3-pda-field-work-request`、`just wave-3-pda-field-execution-summary --json`、`just wave-3-pda-field-precheck-summary --from-env --json`、`just wave-3-pda-field-owner-gap-actions --json`、`just wave-3-pda-field-handoff-bundle --json`、`just wave-3-pda-evidence-package-template`、`just wave-3-pda-intake-template --json` 和 `just wave-3-pda-intake-check --json` 只用于现场分工、资源申请、前置摘要、owner 派单、交接总包、证据包准备和 intake 预检，不写 runtime evidence、不能关闭 W6.D。`just wave-3-pda-service-precheck` 只读检查 `/healthz` 和 Wave3 路由鉴权边界；`just wave-3-pda-trace-code-openapi-precheck --from-env --json` 只读检查追溯码 OpenAPI 合约；`just wave-3-pda-runtime-readiness --from-env --json` 只读检查输入字段、引用边界、计数阈值和服务前置；`just wave-3-pda-runtime-evidence-record --from-env --check-only --json` 只复用正式 validator 校验证据字段和引用边界。正式 `just wave-3-pda-runtime-evidence-record --from-env --json` 或 `just wave-3-pda-intake-record --json` 写入 runtime evidence，并输出 JSON 结果用于现场归档；两者二选一，取决于现场使用环境变量还是 JSON intake。上述 readiness / check-only 命令都不会写入 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。staging 服务可达或 `/api/v1/inventory/batches` 返回 401 只能证明服务前置，不得替代真 PDA、实体扫码键、离线 replay、幂等 replay、L7 和易用性 evidence。

### 5. Wave 4 M-TC 码上放心 evidence

正式接口文档、鉴权、错误码、频率限制、成功上报、失败重试和 audit_event 查询证据到位后，运行：

```bash
just wave-4-external-dependencies-record --export-template
just wave-4-external-dependencies-readiness --from-env --json
just wave-4-external-dependencies-record --from-env --check-only --json
just wave-4-external-dependencies-record --from-env --json
just wave-4-external-dependencies-validate
```

`just wave-4-external-dependencies-readiness --from-env --json` 只读检查正式接口文档、鉴权、错误码、频率限制、Vault 凭证引用、成功上报、失败重试和 audit_event 查询引用；`just wave-4-external-dependencies-record --from-env --check-only --json` 只复用正式 validator 校验证据字段和引用边界。两者都不会写入 `docs/retros/wave-4-external-dependencies.json`，不能关闭 W6.E gate。

### 6. Wave 5 M-PK hardware evidence

电子秤、蓝牙打印机、面单打印设备、校准记录、打印产物核对和 audit_event 查询证据到位后，运行：

```bash
just wave-5-hardware-materials --export-template
just wave-5-hardware-materials --from-env --json
just wave-5-hardware-readiness --from-env --json
just wave-5-hardware-evidence-record --from-env --check-only --json
just wave-5-hardware-evidence-record --from-env --json
just wave-5-hardware-evidence-validate
```

`just wave-5-hardware-materials --from-env --json`、`just wave-5-hardware-readiness --from-env --json` 和 `just wave-5-hardware-evidence-record --from-env --check-only --json` 只读检查输入字段、引用边界和计数阈值；不会连接真实硬件，不会写入 `docs/retros/wave-5-hardware-evidence.json`，不能关闭 W6.F gate。

### 7. Wave 5 M10 TMS+ evidence

真实 TMS dev/staging 推送、回调、失败重试、Vault 凭证引用和 audit_event 查询证据到位后，运行：

```bash
just wave-5-tms-materials --export-template
just wave-5-tms-materials --from-env --json
just wave-5-tms-readiness --from-env --json
just wave-5-tms-evidence-record --from-env --check-only --json
just wave-5-tms-evidence-record --from-env --json
just wave-5-tms-evidence-validate
```

`just wave-5-tms-materials --from-env --json`、`just wave-5-tms-readiness --from-env --json` 和 `just wave-5-tms-evidence-record --from-env --check-only --json` 只读检查输入字段、引用边界和计数阈值；不会调用 TMS，不会写入 `docs/retros/wave-5-tms-evidence.json`，不能关闭 W6.G gate。

### 8. Wave 6 gray release evidence

staging 灰度发布、smoke gate、dashboard、回滚演练、双人审批和 audit_event 查询证据到位后，运行：

```bash
just wave-6-deploy-materials --export-template
just wave-6-deploy-materials --from-env --json
just wave-6-deploy-audit --from-env --check-only
just wave-6-deploy-audit --from-env
just wave-6-deploy-readiness --from-env --json
just wave-6-deploy-evidence-record --from-env --check-only --json
just wave-6-deploy-evidence-record --from-env --json
just wave-6-deploy-evidence-validate
```

## 最终关闭

W6.A-W6.H 8 个 evidence gate 均已通过各自 validator（其中 W6.A/W6.B 共用 `just wave-1-runtime-evidence-validate`）后，先做一次状态与治理检查：

```bash
just wave-6-evidence-preflight
just wave-6-status
just wave-6-evidence-check
just wave-6-missing-evidence-commands
just gov-t1
just task-check
git diff --check
```

写 retro 前运行 `just wave-6-status` 仅作状态盘点；此时唯一允许的非 evidence 缺口是 `W6-retro`。真正的“无阻塞缺口”状态必须在 `docs/retros/wave-6-retro.md` 写入后再由 `just wave-6-status` 和 `just wave-6-complete-check` 共同证明。

如果当前仍有 evidence gate 缺口，可以先输出只面向人工执行的命令清单：

```bash
just wave-6-missing-evidence-commands
```

该入口等价于 `python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only`。该模式首行输出只读边界，随后输出缺失 evidence gate 的命令清单，并按 readiness / record-check-only / record / validate 顺序列出下一步命令；W6.H 因 readiness 依赖正式 audit 写入后的 `audit_event_query_ref`，清单固定为 materials --export-template → materials --json → deploy audit --check-only → deploy audit → deploy readiness → evidence record --check-only → evidence record → validate。缺失 evidence 时会因 --strict 返回非零，这是阻塞信号，不代表命令写入或关闭 gate；它不会写入 runtime evidence，也不会关闭 evidence gate。`--commands-only` 是文本快捷入口，不能和 JSON report 混用；脚本会以 `--commands-only cannot be combined with --json` 拒绝 `--commands-only --json`。

just wave-6-status 普通文本报告和 commands-only 清单都会在 W6.B 缺失时提示二选一；二者都会在 W6.B evidence 行后固定输出 `choice: W6.B rollback 按实际部署形态二选一：k8s 或 docker-compose`。just wave-6-status 普通文本报告和 commands-only 清单都会用 `path: k8s` / `path: docker-compose` 分组，把两组 rollback readiness 与 record 命令分开，提醒执行人只选择当前真实部署形态对应的一组 rollback 命令。

缺失 evidence 时，首行固定为：

```text
Wave 6 missing evidence commands: 只读命令清单；不会写入 runtime evidence，不能关闭 evidence gate
Wave 6 missing evidence commands: 缺失 evidence 时 --strict 返回非零；这是阻塞信号，不代表命令写入或关闭 gate
```

无缺失 evidence 时，唯一输出行固定为：

```text
Wave 6 missing evidence commands: none；只读命令清单；不会写入 runtime evidence，不能关闭 evidence gate
```

complete 模式下如果没有缺失 evidence gate 的采集命令，但仍有 `W6-retro` 等非 evidence blocker，脚本会追加说明：

```text
Wave 6 missing evidence commands: none 只表示没有缺失 evidence gate 的采集命令；complete-check 仍可能因非 evidence blocker 返回非零
```

然后写 `docs/retros/wave-6-retro.md`，至少包含：

- W6.A 到 W6.H 这 8 个 gate ID、对应 evidence 文件路径和写 retro 前验证命令结果。
- 真实环境、设备、外部系统和灰度发布的剩余风险。
- 明确声明没有使用 localhost/127.0.0.1/0.0.0.0/local/mock/fake/stub/example/prod/production 证据。

`report_wave6_pre_release.py --strict --evidence-only --json` 的机器消费字段以以下 key 为准：

- `schema_version`：report JSON 合同版本。
- `available_modes`：当前支持的 `evidence-only` / `complete` 模式清单。
- `writes_runtime_evidence`：固定为 `false`；report 只读，不写任何 runtime evidence。
- `closes_gate`：固定为 `false`；report 不能替代真实 evidence gate。
- `report_command`：complete 模式 JSON 报告命令。
- `evidence_only_command`：只检查 8 个真实 evidence gate 的 JSON 报告命令。
- `commands_only_command`：输出缺失 evidence gate 人工采集命令清单的 just 入口。
- `evidence_gate_ids`：W6.A 到 W6.H 的 gate ID 清单。
- `evidence_gate_evidence_files`：8 个真实 evidence JSON 文件路径清单。
- `evidence_gate_just_entries`：每个 evidence gate 的 readiness / record / validate just 入口映射。
- `evidence_gate_execution_files`：每个 evidence gate 的底层 readiness / record / validate 执行文件映射。
- `required_top_level_files`：Wave 6 report / preflight 要求存在的顶层范围与收口文件清单。
- `required_runbooks`：8 个真实 evidence gate 依赖的 runbook 清单。
- `required_execution_files`：Wave 6 preflight 和 report 要求存在的底层执行文件清单。
- `validation_commands`：写 retro 前必须复跑并记录结果的验证命令清单。
- `closeout_just_entries`：Wave 6 最终收口命令入口清单，包含 `wave-6-status`、`wave-6-evidence-check`、`wave-6-missing-evidence-commands` 和 `wave-6-complete-check`。
- `evidence_gate_item_ids` / `non_evidence_item_ids`：完整 `items` 的分组索引，二者必须互斥且合并回 `items`。
- `blocking_count` / `ignored_count`：当前模式下真正阻塞与被模式忽略的 item 数量。
- `evidence_blocking_count` / `evidence_ignored_count`：8 个真实 evidence gate 中阻塞与被忽略的数量；complete 模式不允许 evidence gate 被忽略。
- `evidence_blocking_item_ids` / `non_evidence_blocking_item_ids`：`blocking_gaps` 的分组索引。
- `evidence_ignored_item_ids` / `non_evidence_ignored_item_ids`：`ignored_gaps` 的分组索引；evidence-only 模式只允许 W6-retro 进入非 evidence ignored。
- `blocking_details`：当前模式下阻塞项的结构化明细，包含 kind、gate ID、item ID、状态、需求描述和 gaps。
- `ignored_details`：当前模式下被忽略项的结构化明细，字段同 `blocking_details`。
- `missing_evidence_count`：当前缺失的真实 evidence JSON 文件数量，必须等于 `missing_evidence_files` 长度。
- `missing_evidence_item_ids` / `missing_evidence_files`：当前仍缺真实 evidence 的 gate 与 JSON 文件路径清单；写 retro 前必须为空。
- `missing_evidence_details`：当前仍缺真实 evidence 的结构化明细，包含 gate ID、item ID、evidence 文件、状态、需求描述和 gaps。
- `readiness_commands` / `record_check_only_commands` / `record_commands` / `validate_commands`：`missing_evidence_details` 内每个 gate 的准备、正式采集前只读预检、采集和验证命令清单。
- `deployment_choice_required` / `deployment_choice_label` / `deployment_choice_options` / `deployment_path_commands`：仅 W6.B 使用的部署路径二选一元数据；机器消费方必须按实际部署形态只选择 `k8s` 或 `docker-compose` 其中一组 readiness / record 命令。

写完 retro 后再次运行：

```bash
just wave-6-status
just wave-6-complete-check
just gov-t1
just task-check
git diff --check
```
