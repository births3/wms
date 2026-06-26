# Wave 3 PDA Readiness Runbook

> 适用范围：SPIKE-005 RN 扫枪 + 离线队列、SPIKE-005B WebView/Capacitor PDA 可行性验证的启动前置。当前决策为先落 readiness/runbook，不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖（含 `devDependencies`），不显式把 `apps/pda-mobile` 加入 `pnpm-workspace.yaml`，不创建 `apps/pda-mobile` 生产 app。

## 目标

在真 PDA 和稳定 dev/staging 可用前，先明确启动 SPIKE-005 / SPIKE-005B 所需的输入边界，避免用本地模拟、纯浏览器或手机摄像头扫码结果替代真机结论。

## 范围

- 记录设备清单与借测状态
- 准备扫码样本与 M2/M3 API 验证路径
- 明确 dev/staging、账号、API Key 与日志引用要求
- 定义 SPIKE-005 / SPIKE-005B 启动/停止条件

不包含：

- 不引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖（含 `devDependencies`）
- 不显式把 `apps/pda-mobile` 加入 `pnpm-workspace.yaml`
- 不接实体扫码键 SDK
- 不实现离线队列持久化
- 不写 L7 性能阈值默认值

## 前置条件

- Wave 1 H1 鉴权模型可用
- Wave 1 H3 `@wms/api-client` 可用
- Wave 3 M2/M3 handler 已接 PostgreSQL repository、`Idempotency-Key` 与 H2 审计
- 至少一台业务方采购或借测 PDA 可用
- 目标环境为真实 `dev` 或 `staging`

## 设备清单

| 设备型号 | 来源 | Android 版本 | 扫码输入方式 | 状态 | 备注 |
|---|---|---|---|---|---|
| Honeywell EDA52 | 待借测 | 待确认 | 实体扫码键 / Intent | 未到位 | SPIKE-005 / 005B 候选 |
| Urovo i6310 | 待借测 | 待确认 | 实体扫码键 / KeyEvent | 未到位 | SPIKE-005 / 005B 候选 |
| Zebra TC52 | 待借测 | 待确认 | DataWedge / Intent | 未到位 | SPIKE-005 / 005B 候选 |

## 扫码样本

| 类型 | 数量 | 用途 |
|---|---:|---|
| GS1 追溯码 | 待确认 | M2 收货 / 验收扫描 |
| Code128 批号 / 箱码 | 待确认 | M2 上架 / M3 库存定位 |
| 二维码任务号 | 待确认 | PDA 任务流转 |

样本总量沿用 SPIKE-005 的 50 个不同条码方向；分类拆分等 SPIKE-005 / SPIKE-005B 启动时按设备和流程确认。样本必须来自脱敏测试数据，不使用生产真实药品追溯码。

## 术语速查

这张表给现场执行人统一口径。术语不清时先对照本表，不要用浏览器、模拟器、手机摄像头或本地脚本替代真 PDA 事实。

| 术语 | 现场含义 | 不能混淆 |
|------|----------|----------|
| 真 PDA | 业务方采购或借测的手持 PDA 设备，带实体扫码键或厂商扫码通道 | 不能用普通手机、浏览器、模拟器、摄像头扫码替代 |
| 实体扫码键 / scan-key | PDA 上可稳定触发扫码的物理按键，或厂商提供的 KeyEvent / Intent / DataWedge 通道 | 不能只证明网页输入框能粘贴条码 |
| `scan_input_method` | 记录扫码输入来自 scan-key / KeyEvent / Intent / DataWedge 中哪一种 | 不能写 camera / phone / browser |
| L7 | 真 PDA 上按 M2/M3、offline replay、Idempotency-Key replay 场景跑出的端到端执行记录 | 当前只记录实测事实，不发明本地性能阈值 |
| offline replay | PDA 断网后先暂存操作，恢复网络后按顺序重放并记录结果 | 不能用后端本地脚本 replay 替代真机断网恢复 |
| Idempotency-Key replay | 使用同一个 `Idempotency-Key` 重放请求，证明重复提交不会产生重复业务结果 | 不能只写“重试成功”，必须记录首次请求、重放请求和响应一致性 |
| H2 `audit_event` | H2 审计追踪表中的事件记录，用来证明 M2/M3/PDA 操作和 replay 已落审计 | 不能用应用日志替代审计查询引用 |
| evidence ref | 指向日志、证据包、截图、查询结果或资产登记的引用；Evidence JSON 只保存引用 | 不能把大体积日志、截图、视频或真实 key 直接塞进 evidence JSON |
| trace-code OpenAPI precheck | 只读检查追溯码查询接口 OpenAPI 版本、required GET/POST operations 与 `X-API-Key` 认证头 | 只能作为前置附件，不能关闭 W6.D gate |

## 证据命名与归档规则

证据引用必须让 reviewer 能定位到“谁、在什么环境、用哪台 PDA、执行了哪个场景、结果是什么”。Evidence JSON 只保存引用，证据正文放在 CI、对象存储、票据系统、资产系统或数据库查询归档中。

| 证据类型 | 推荐引用形态 | 必须包含 |
|----------|--------------|----------|
| PDA 资产 | `asset://wms-staging/pda/honeywell-eda52-01` | `asset://`、`/pda/`、真实设备编号、dev 或 staging 环境标记 |
| scan / replay 日志 | `ci/staging/wave3-pda-m2-scan/run-20260614-01` | 环境、`wave3-pda`、场景名、run ID 或 trace ID |
| `audit_event` 查询 | `ci/staging/wave3-pda-audit-event/query-20260614-01` | 环境、`audit-event`、查询时间、M2/M3/replay resource ID |
| L7 记录 | `ci/staging/wave3-pda-l7/run-20260614-01` | 环境、设备型号、样本批次、场景计数、结果摘要 |
| 易用性走查 | `s3://wms-staging-evidence/wave3/pda/usability-review-20260614-01.md` | 环境、操作员角色、设备握持、扫码键触达、离线 / 错误提示、结论 |
| trace-code OpenAPI 预检附件 | `ci/staging/wave3-pda-trace-code-openapi-precheck/run-20260614-01` | 环境、OpenAPI 3.0.3、required GET/POST operations 摘要、`X-API-Key` header scheme 摘要；不得包含 key 值 |

场景名建议固定为 `m2-scan`、`m3-scan`、`offline-replay`、`idempotency-replay`、`audit-event`、`l7`、`usability-review`、`trace-code-openapi-precheck`。所有引用必须显式包含 `dev` 或 `staging`；不得包含 local、prod、production、mock、fake、stub、example、browser、simulator、emulator、phone、camera，也不得保留模板占位。API key、token、密码不得出现在文件名、截图、日志正文、证据包或 evidence JSON 中。

## 执行步骤

1. 确认设备清单中至少一台 PDA 到位，并记录型号、系统版本和扫码输入方式。
2. 确认 dev/staging 已部署包含 Wave 3 M2/M3 handler 的 `wms-api`。
3. 准备具备 `m2.write` / `m3.write` 权限的测试账号。
4. 准备 M2 收货单、M2 验收、M2 上架、M3 状态变更的测试数据。
5. 启动 SPIKE-005 与 / 或 SPIKE-005B，两天时间盒重新计时；如做技术栈对比，两者必须使用同一设备、条码样本和 M2/M3 测试数据。
6. 采集扫码延迟、离线队列重放、幂等 replay、审计落库证据。
7. 输出 SPIKE-005 / SPIKE-005B 结论，并更新 ADR-0027 PDA 离线模型与技术栈定版框架。

## 启动条件

满足以下全部条件才启动 SPIKE-005 / SPIKE-005B：

- 至少一台真 PDA 到位
- dev/staging 服务可访问
- M2/M3 测试数据可重建
- 可以保存测试日志引用

## 拒绝边界

- 使用纯浏览器、模拟器或手机摄像头代替 PDA 实体扫码键
- 使用 local / prod / production / mock / fake / stub / example 作为证据
- 任一证据引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`
- 未记录 `Idempotency-Key` 与 H2 `audit_event` 的链路证据
- 在未完成 SPIKE-005 / SPIKE-005B 前引入 RN / Expo / EAS / Capacitor 生产 workspace 依赖（含 `devDependencies`）
- 在 ADR-0027 Accepted 前显式把 `apps/pda-mobile` 加入 `pnpm-workspace.yaml`
- 未说明蓝牙打印能力的证据归属：如 W3 PDA 真机未覆盖打印 PoC，必须由 Wave 5 M-PK hardware evidence（`docs/retros/wave-5-hardware-evidence.json`）验证蓝牙打印机 / 面单打印机后，才能关闭完整生产硬件能力 gate

## Evidence JSON

真实证据写入 `docs/retros/wave-3-pda-runtime-evidence.json`。clarifications #67 已确认当前不发明本地 L7 阈值，因此本 evidence 只验证真实 PDA、真实 dev/staging、日志引用、审计链路和人工易用性走查是否存在。
以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

```json
{
  "environment": "staging",
  "pda_model": "Honeywell EDA52",
  "android_version": "Android 11",
  "scan_input_method": "physical-scan-key-intent",
  "pda_stack_candidate": "react-native",
  "pda_device_ref": "asset://wms-staging/pda/honeywell-eda52-01",
  "spike005_result_ref": "docs/spikes/spike-005-rn-scanner.md#staging-runtime-20260603",
  "m2_scan_log_ref": "ci/staging/wave3-pda-m2-scan/123",
  "m3_scan_log_ref": "ci/staging/wave3-pda-m3-scan/123",
  "offline_replay_log_ref": "ci/staging/wave3-pda-offline-replay/123",
  "idempotency_replay_log_ref": "ci/staging/wave3-pda-idempotency-replay/123",
  "audit_event_query_ref": "ci/staging/wave3-pda-audit-event/123",
  "l7_run_ref": "ci/staging/wave3-pda-l7/123",
  "usability_review_ref": "s3://wms-staging-evidence/wave3/pda/usability-review-20260603T120000.md",
  "barcode_samples_scanned": 50,
  "m2_operations_exercised": 1,
  "m3_operations_exercised": 1,
  "offline_replays_exercised": 50,
  "idempotency_replays_exercised": 50,
  "real_pda_used": true,
  "physical_scan_key_verified": true,
  "dev_or_staging_service_verified": true,
  "audit_event_verified": true,
  "l7_review_completed": true,
  "usability_review_completed": true
}
```

执行验证：

所有 PDA 设备、SPIKE、扫描、离线 replay、审计、L7 和易用性证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / production / mock / fake / stub / example / browser / simulator / emulator / phone / camera，也不能保留 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认` 等模板占位。`pda_device_ref` 必须是 `asset://.../pda/...` 形式的 PDA 设备资产引用，不能用普通日志路径替代。`scan_input_method` 必须记录 PDA 实体扫码键或厂商扫码通道（如 scan-key / KeyEvent / Intent / DataWedge），不能用手机摄像头扫码替代。`m2_scan_log_ref` / `m3_scan_log_ref` 必须分别指向 M2 / M3 scan 证据；`offline_replay_log_ref` 必须指向 offline replay 证据；`idempotency_replay_log_ref` 必须指向 Idempotency-Key replay 证据，不能用普通重试日志替代。`l7_run_ref` 必须指向 L7 执行记录；`usability_review_ref` 必须指向 usability review 走查记录。`pda_stack_candidate` 必须为 `react-native` 或 `webview-capacitor`，并且要和 `spike005_result_ref` 指向的 SPIKE-005 / SPIKE-005B 实测结果一致。`spike005_result_ref` 字段名为现有 validator 兼容保留；record 命令可用 `--spike-result-ref` 中性参数写入同一字段。

当 `pda_stack_candidate=webview-capacitor` 时，还必须提供 `native_shell_ref` 与 `native_scan_plugin_ref`，分别指向 Android native shell 证据和 native scan plugin 证据；这两个引用同样必须包含当前 `environment` 标记，且不能指向 local / prod / production / mock / fake / stub / example / browser / simulator / emulator / phone / camera。

ADR-0027 Accepted 后启动生产 `apps/pda-mobile` 时，生产依赖、lockfile importers / packages / snapshots 条目和 Android native 打包脚本必须与 `pda_stack_candidate` 一致：`react-native` 候选不得混入 Capacitor 生产链路，`webview-capacitor` 候选不得混入 React Native / Expo / EAS 生产链路。Expo / EAS Android build 或 prebuild 命令同样视为 RN 生产链路。

record 前先运行只读 readiness。该命令复用 evidence 字段校验，额外探测 `/healthz` 和 Wave3 `/api/v1/inventory/batches` 未授权鉴权边界；不会写入 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。staging 服务可达或 Wave3 路由返回 401 只能证明服务前置，不得替代真 PDA、实体扫码键、离线 replay、幂等 replay、L7 和易用性 evidence。

`readiness --from-env --json` 遇到缺失变量时会输出 `missing_env_vars` 和 `missing_env_var_owners`，用于现场自动采集链定位未填写的 `WAVE_3_PDA_*` 变量以及对应负责人；该错误输出同样不会写 evidence，也不能关闭 W6.D。

无 PDA 阶段也可以运行追溯码 OpenAPI 只读预检，验证外部查询接口合约包含 `GET /api/codes/{code}`、`GET /api/codes/{code}/children`、`POST /api/codes/batch`、`POST /api/codes/verify`、`POST /api/wms-products` 以及 `X-API-Key` header 认证方案。该命令只使用 `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` 和 `WAVE_3_PDA_TRACE_CODE_API_KEY` 发起只读 OpenAPI 请求，不打印 key，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate，也不能替代真 PDA 扫码日志、离线 replay、幂等 replay、L7 和易用性 evidence。

正式 record 前，用同一组参数追加 `--check-only` 做一次 recorder 级预检。该模式只复用正式 validator 校验证据字段和引用边界，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

`record --from-env --check-only --json` 遇到缺失变量时会输出 `missing_args`、`missing_env_vars` 和 `missing_env_var_owners`，用于现场自动采集链定位未填写的 `WAVE_3_PDA_*` 变量以及对应负责人；如果字段已齐但确认布尔项仍不是 `true`，会输出 `false_flag_env_vars` 和 `false_flag_env_var_owners`。该错误输出同样不会写 evidence，也不能关闭 W6.D。

## 模板命令

材料仅用于本次 runtime evidence 采集，不得写入真实 evidence JSON。先执行一次导出模板，补齐 `WAVE_3_PDA_*` 后直接拷贝命令执行：

```bash
just wave-3-pda-preaudit-kit --json
```

`just wave-3-pda-preaudit-kit --json` 只输出 W6.D 预审包，用于项目负责人在真 PDA 实测前确认“现在就能推进”的事项、必须等真机采集的字段和禁止事项；不探测服务，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

```bash
just wave-3-pda-materials-checklist --json
```

`just wave-3-pda-materials-checklist --json` 只输出现场字段分工、来源、无 PDA 阶段可准备项和必须真机采集项；不探测服务，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

```bash
just wave-3-pda-field-work-request
```

`just wave-3-pda-field-work-request` 只输出可转发资源申请包，用于把真 PDA、扫码键、条码样本、M2/M3 测试数据、L7 和易用性走查任务分派给现场负责人；不探测服务，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。需要结构化分派时可追加 `--json`。

`--json` 输出保留英文资源字段，并追加中文资源项、负责人、交付物、验证变量、`execution_order_zh`、`troubleshooting` 和 `next_commands`，用于把现场任务直接拆成工单；这些字段仍然只是分派材料，不能替代真 PDA evidence。

```bash
just wave-3-pda-field-execution-summary --json
```

`just wave-3-pda-field-execution-summary --json` 只汇总当前 shell 中已设置 / 缺失的现场变量、无 PDA 阶段可运行的只读预检命令、必须等真 PDA 的字段、仍未置 `true` 的布尔确认项、正式 record 前命令顺序和禁止事项；不探测服务，不打印环境变量值，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

如果已经归档脱敏前置附件（例如 `docs/retros/wave-3-pda-field-precheck-2026-06-14.json`），可以追加 `--field-precheck-attachment docs/retros/wave-3-pda-field-precheck-2026-06-14.json` 生成现场摘要。该模式只把附件中已通过且事实完整的 `service_precheck` / `trace_code_openapi_precheck` 视为 no-PDA 前置已满足，减少重复索要 `WAVE_3_PDA_ENVIRONMENT`、`WAVE_3_PDA_SERVICE_URL`、`WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` 和 `WAVE_3_PDA_TRACE_CODE_API_KEY`。附件必须证明 service `/healthz` 为 200、Wave3 受保护路由返回 `AUTH-001`，并证明 trace-code OpenAPI 为 200、OpenAPI 版本为 3.0.3、认证头为 `X-API-Key`、5 个 required GET/POST operations 全部存在；缺任一事实都会被拒绝。它仍会保留真 PDA、实体扫码键、M2/M3 scan、offline replay、idempotency replay、`audit_event`、L7 和易用性缺口，不写 runtime evidence，不能关闭 W6.D gate。

```bash
just wave-3-pda-field-precheck-summary --from-env
```

`just wave-3-pda-field-precheck-summary --from-env` 是现场前置一键预检，默认输出 Markdown 总览，便于直接转发给现场负责人。它只读组合 `service_precheck`、`trace_code_openapi_precheck` 和 `field_execution_summary` 三段结果；会探测 dev/staging `/healthz`、Wave3 鉴权边界和追溯码 OpenAPI 合约，并输出当前变量缺口。缺少当前可准备变量时，Markdown 会在 `Missing Now Env Vars` 段列出变量名和负责人；不打印 key，不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

需要结构化附件时追加 `--json`：`just wave-3-pda-field-precheck-summary --from-env --json`。JSON 输出适合作为现场执行前的总览附件；真正关闭仍必须等真 PDA evidence record 和 validate 通过。

```bash
just wave-3-pda-field-owner-gap-actions
```

`just wave-3-pda-field-owner-gap-actions` 默认输出 Markdown owner 缺口动作单，便于直接转发给现场负责人。它把当前 `field_execution_summary` 的缺失变量和未置 `true` 的确认项按 `source_owner` 聚合，并分列显示 `Missing now`、`Real evidence vars` 和 `False flags`，让现场区分现在可补变量、必须等真 PDA 的 evidence 变量和仍未置 `true` 的确认项；不联网、不打印变量值、不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

需要结构化附件时追加 `--json`：`just wave-3-pda-field-owner-gap-actions --json`。JSON 输出包含 `field_owner_gap_actions`，每项包含 `env_vars`、`missing_env_vars`、`missing_now_env_vars`、`false_truth_flag_env_vars`、`evidence_requirements`、`no_pda_stages`、`requires_real_pda` 和 `action`，用于现场系统派单或归档。

```bash
just wave-3-pda-field-handoff-bundle --json
```

`just wave-3-pda-field-handoff-bundle --json` 只读聚合 W6.D 现场交接总包，包含 `preaudit_kit`、`materials_checklist`、`field_work_request`、`field_execution_summary`、`field_owner_gap_actions`、`evidence_package_template` 和 `intake_template`。默认不联网、不打印变量值、不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。需要把 service precheck 和 trace-code OpenAPI precheck 一并纳入交接附件时，追加 `--from-env --json`；已有脱敏附件时也可以追加 `--field-precheck-attachment docs/retros/wave-3-pda-field-precheck-2026-06-14.json --json`，用于让 `field_execution_summary` 和 `field_owner_gap_actions` 消化已完成的 no-PDA 前置。需要把交接总包落盘归档时，追加 `--field-handoff-output docs/retros/wave-3-pda-field-handoff-2026-06-14.json --json`；如果该交接附件已存在，默认拒绝覆盖，只有确认重生成时才追加 `--field-handoff-force`。该文件仍只是现场交接附件，不是 `docs/retros/wave-3-pda-runtime-evidence.json`。真实 key 仍只从环境变量或 secret 管理系统读取，不进入输出。

```bash
just wave-3-pda-evidence-package-template
```

`just wave-3-pda-evidence-package-template` 只输出 Markdown 证据包模板，用于现场记录 M2/M3 scan、offline replay、Idempotency-Key replay、`audit_event`、L7 和易用性走查材料；不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。结构化输出 `--json` 会附带 `owner_actions` 和 `record_gate_after_owner_actions`，按负责人列出回填变量、验收条件、`can_write_runtime_evidence=false` 边界和采齐后必须运行的 record gate 命令顺序。

需要结构化证据包模板时可追加 `--json`，输出 `sections`、`mapping_variables`、`blocked_flags_until_refs_present` 和 `warnings`，用于现场归档系统生成待填表单；`mapping_variables` 会包含 WebView/Capacitor 路径必需的 `WAVE_3_PDA_NATIVE_SHELL_REF` 与 `WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF`，但这两个字段只有在 `pda_stack_candidate=webview-capacitor` 时才会进入正式 evidence 校验。这些字段仍然只是模板，不能替代真 PDA evidence。

## W6.D JSON intake 采集流程

当现场不方便维护一长串 `WAVE_3_PDA_*` 环境变量时，可以用 JSON intake 文件收集同一批真实材料。intake 只是现场填写和预检载体，不是 runtime evidence；导出、填写和 check-only 都不写 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

先导出 intake 模板：

```bash
just wave-3-pda-intake-template --json
```

`just wave-3-pda-intake-template --json` 是 `record --export-intake-template --json` 的短入口。模板输出的 `mode` 是 `wave3-pda-runtime-evidence-intake-template`，`kind` 是 `wave3-pda-runtime-evidence-intake`，`writes_runtime_evidence=false`，`closes_gate=false`。现场只填写 `evidence` 对象里的字段；不得把 trace-code API key 写入 intake 文件，追溯码 key 仍只允许放环境变量或 secret 管理系统。需要把模板落盘给现场填写时，追加 `--intake-template-output docs/retros/wave-3-pda-intake-template-2026-06-14.json --json`；已有模板文件默认拒绝覆盖，确认重生成才追加 `--intake-template-force`。该文件仍只是待填 intake 模板，不是 runtime evidence。

intake 模板里的空字符串表示对应证据字段仍未填写，`false` truth flags 表示对应真实证据尚未确认。现场直接校验未填模板时，`just wave-3-pda-intake-check --json` 会同时输出 `missing_env_vars`、`missing_env_var_owners`、`false_flag_env_vars` 和 `false_flag_env_var_owners`，用于按负责人补齐材料；这些错误输出仍不写 runtime evidence，不能关闭 W6.D gate。

填完后先做只读校验现场 JSON：

单行入口：`just wave-3-pda-intake-check --json`

```bash
just wave-3-pda-intake-check --json
```

`just wave-3-pda-intake-check --json` 从 `WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE` 读取 intake 文件，并执行 `record --from-intake-file <path> --check-only --json`；它会复用正式 validator 校验证据字段、引用边界、计数下限和 truth flags，不写 runtime evidence。`real_pda_used`、`physical_scan_key_verified`、`audit_event_verified`、`l7_review_completed`、`usability_review_completed` 只能在对应真 PDA、实体扫码键、H2 `audit_event`、L7 和易用性走查证据全部归档后置为 `true`。

如果 `pda_stack_candidate=webview-capacitor`，intake 里还必须填写 `native_shell_ref` 和 `native_scan_plugin_ref`；两者必须指向 dev/staging 的 Android native shell 与 native scan plugin 实测证据，不能指向 local、browser、simulator、emulator、phone、camera、mock、fake、stub、example 或 prod。

record check-only 通过后，正式 record 仍必须使用同一份真实材料，并按 gate 顺序执行。选择 from-env 路径时运行：

```bash
just wave-3-pda-runtime-readiness --from-env --json
just wave-3-pda-runtime-evidence-record --from-env --check-only --json
just wave-3-pda-runtime-evidence-record --from-env --json
just wave-3-pda-runtime-evidence-validate
```

如果现场选择 intake 路径，正式 record 使用同一份 intake 文件运行：

```bash
just wave-3-pda-intake-check --json
just wave-3-pda-intake-record --json
just wave-3-pda-runtime-evidence-validate
```

`just wave-3-pda-intake-record --json` 从同一个 `WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE` 读取真实材料，调用 `record --from-intake-file <path> --json` 写入 `docs/retros/wave-3-pda-runtime-evidence.json`。它是正式 record 入口，不是只读预检；前提仍是真 PDA evidence 已齐，且 `just wave-3-pda-intake-check --json` 已通过。任何 intake 模板、intake 文件或 check-only 输出都不能替代最终 `docs/retros/wave-3-pda-runtime-evidence.json`。

```bash
just wave-3-pda-trace-code-openapi-precheck --from-env --json
```

`just wave-3-pda-trace-code-openapi-precheck --from-env --json` 只读验证追溯码 OpenAPI 合约和 `X-API-Key` 认证头。运行前由追溯码接口负责人 / 运维设置 `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` 和 `WAVE_3_PDA_TRACE_CODE_API_KEY`；真实 key 只能放在环境变量或 secret 管理系统，不得写入文档、仓库、截图或 evidence JSON。该命令输出可作为证据包附件说明外部查询接口前置已检查，但不能关闭 W6.D gate。

```bash
just wave-3-pda-runtime-evidence-record --export-template
```

仅记录与真实前置相关的只读边界：

- `--check-only` 是预检模式，不写入 `docs/retros/wave-3-pda-runtime-evidence.json`。
- `--check-only` 仍受参数必填约束，不会因缺参而静默通过。
- `--export-template` 仅输出模板文本；填写 PDA 型号、Android 版本、证据引用等值时保留单引号，避免 `Honeywell EDA52` / `Android 11` 这类带空格值被 shell 拆词。
- 无 PDA 阶段可先运行 `just wave-3-pda-preaudit-kit --json`，只读导出预审包；该命令不检查服务，不校验 PDA evidence 字段，不能关闭 W6.D gate。
- 无 PDA 阶段可先运行 `just wave-3-pda-materials-checklist --json`，只读导出现场字段分工；该命令不检查服务，不校验 PDA evidence 字段，不能关闭 W6.D gate。
- 无 PDA 阶段可运行 `just wave-3-pda-field-work-request`，只读导出可转发资源申请包；该命令不检查服务，不生成真实 evidence JSON，不能关闭 W6.D gate。
- 无 PDA 阶段可运行 `just wave-3-pda-field-execution-summary --json`，只读汇总当前变量缺口和后续命令；该命令不检查服务，不打印变量值，不生成真实 evidence JSON，不能关闭 W6.D gate。
- 无 PDA 阶段可运行 `just wave-3-pda-field-precheck-summary --from-env`，默认输出 Markdown 现场总览；结构化附件用 `just wave-3-pda-field-precheck-summary --from-env --json`。它只读组合服务前置、追溯码 OpenAPI 和字段摘要；不打印 key，不生成真实 evidence JSON，不能关闭 W6.D gate。
- 无 PDA 阶段可运行 `just wave-3-pda-evidence-package-template`，只读导出现场 Markdown 证据包模板；该命令不生成真实 evidence JSON，不能关闭 W6.D gate。
- 无 PDA 阶段可运行 `just wave-3-pda-field-handoff-bundle --json`，只读聚合预审、材料清单、资源申请、owner 缺口和证据包模板；需要带预检结果时运行 `just wave-3-pda-field-handoff-bundle --from-env --json`；归档到 `--field-handoff-output` 时默认拒绝覆盖，确认重生成才追加 `--field-handoff-force`。这些模式都不能关闭 W6.D gate。
- 无 PDA 阶段可先运行 `just wave-3-pda-service-precheck --json`，只读验证服务和 Wave3 鉴权边界；该模式不校验 PDA evidence 字段，不能关闭 W6.D gate。
- 无 PDA 阶段可先运行 `just wave-3-pda-trace-code-openapi-precheck --from-env --json`，只读验证追溯码查询 OpenAPI 和 `X-API-Key` 认证头；该模式不校验 PDA evidence 字段，不打印 key，不能关闭 W6.D gate。
- 模板中的命令先运行 `just wave-3-pda-service-precheck`，让现场先确认 dev/staging 服务和 Wave3 鉴权边界可达；该命令不会进入 Wave 6 missing-evidence 采集链。
- `--service-url` 必须指向真实 dev/staging 服务，不能指向 local / prod / production / mock / fake / stub / example 地址；命中这些边界时 readiness 会直接失败且不发起探测。
- 模板中的完整命令先运行 `just wave-3-pda-runtime-readiness`，用 `WAVE_3_PDA_SERVICE_URL` 只读验证服务和 Wave3 鉴权边界，并同时校验 PDA evidence 字段；追溯码查询接口需要单独运行 `just wave-3-pda-trace-code-openapi-precheck --from-env --json`；再运行 `just wave-3-pda-runtime-evidence-record --from-env --check-only --json` 做 recorder 级预检，避免现场重复粘贴长参数。

## Staging 服务前置 Dry Run - 2026-06-08

本 dry run 只记录当前 staging 服务前置事实，不是 runtime evidence，不能写入 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

命令：

```bash
just wave-3-pda-service-precheck \
  --environment staging \
  --service-url http://wms-staging.internal:18080 \
  --json
```

结果摘要：

```text
ok=true
mode=wave3-pda-service-precheck
healthz_status=200
healthz_payload_status=ok
wave3_route_status=401
wave3_route_error_code=AUTH-001
```

含义：

- staging 服务可达，`/healthz` 返回 200 且 payload `status=ok`。
- Wave3 `/api/v1/inventory/batches` 无 token 返回 401 / `AUTH-001`，说明受保护 API 边界存在。
- 当前没有 PDA / M2 / M3 runtime 审计事件，不能设置 `audit_event_verified=true`。
- 当前仍缺真 PDA、实体扫码键、离线 replay、幂等 replay、L7 和易用性 evidence。

## Staging + 追溯码 OpenAPI 组合前置 Dry Run - 2026-06-14

本 dry run 只记录现场执行前的服务和外部接口前置事实，不是 runtime evidence，不能写入 `docs/retros/wave-3-pda-runtime-evidence.json`，不能关闭 W6.D gate。

脱敏附件已归档到 `docs/retros/wave-3-pda-field-precheck-2026-06-14.json`。该附件只保存服务和 OpenAPI 预检摘要、剩余真 PDA 阻塞项、`owner_actions` 现场采集动作、正式 record gate 命令顺序和 `<secret-from-secret-manager>` 占位，不保存真实 API key，不能替代 `docs/retros/wave-3-pda-runtime-evidence.json`。

命令形态：

```bash
NO_PROXY='*' no_proxy='*' \
  WAVE_3_PDA_ENVIRONMENT=staging \
  WAVE_3_PDA_SERVICE_URL=http://wms-staging.internal:18080 \
  WAVE_3_PDA_TRACE_CODE_OPENAPI_URL=http://43.128.77.47:9100/openapi/wms-openapi.yaml \
  WAVE_3_PDA_TRACE_CODE_API_KEY=<secret-from-secret-manager> \
  just wave-3-pda-field-precheck-summary --from-env --json
```

网络排障补充：

- 追溯码 OpenAPI 当前可用入口是 `43.128.77.47:9100`；远端 `9200` 当前连接超时，不能作为本轮 W6.D OpenAPI 前置入口。
- 如果现场 curl 默认走代理后返回 `502`，先用 `NO_PROXY='*' no_proxy='*'` 或 `curl --noproxy '*'` 直连重试；直连返回 `200` 时，不要把代理 `502` 判定为 OpenAPI 合约失败。
- `192.168.124.5:7890` 代理路径当前连接超时；真实 key 仍只允许放环境变量或 secret 管理系统，不得为了排障写入命令记录、截图或 evidence JSON。

结果摘要：

```text
ok=true
writes_runtime_evidence=false
closes_gate=false

service_precheck.ok=true
service_precheck.healthz_status=200
service_precheck.healthz_payload_status=ok
service_precheck.wave3_route_status=401
service_precheck.wave3_route_error_code=AUTH-001

trace_code_openapi_precheck.ok=true
trace_code_openapi_precheck.status=200
trace_code_openapi_precheck.openapi=3.0.3
trace_code_openapi_precheck.title=药品追溯码库 WMS 外部接口
trace_code_openapi_precheck.api_key_header_name=X-API-Key
trace_code_openapi_precheck.missing_required_paths=null

trace_code_network_diagnostics.direct_no_proxy_status=200
trace_code_network_diagnostics.default_proxy_path_status=502
trace_code_network_diagnostics.alternate_proxy_192_168_124_5_7890_status=timeout
trace_code_network_diagnostics.remote_9200_status=timeout

field_execution_summary.ready_for_record_from_env_vars=false
field_execution_summary.missing_now_env_vars=[]
field_execution_summary.real_pda_missing_env_vars_count=23
field_execution_summary.false_truth_flag_env_vars_count=5
no_pda_precheck_verified_flag_env_vars=[WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED]
remaining_no_pda_precheck_false_flag_env_vars=[]
remaining_real_evidence_false_flag_env_vars_count=5
```

含义：

- staging WMS 服务前置已通过，`/healthz` 与 Wave3 鉴权边界可达。
- 追溯码 OpenAPI 前置已通过，5 个必需 path 和 `X-API-Key` 认证方案存在。
- 追溯码 OpenAPI 排障结论是直连可用、代理路径不稳定；现场执行这一步优先显式设置 `NO_PROXY='*' no_proxy='*'`，单独 curl 排查时使用 `--noproxy '*'`。
- `WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED` 可由本次 service precheck 附件支撑；正式 record 时仍必须在 evidence payload 中写入 `dev_or_staging_service_verified=true`，但它不能替代真 PDA evidence。
- 当前可准备的环境变量已齐，但 `ready_for_record_from_env_vars=false`，仍缺真 PDA runtime evidence。
- 其余 5 个真实 evidence 布尔确认项仍必须保持 `false`，直到 PDA 资产、实体扫码键、M2/M3 scan、offline replay、Idempotency-Key replay、H2 `audit_event`、L7 和易用性走查证据全部归档。
- 真实 key 只能来自环境变量或 secret 管理系统，不得写入文档、仓库、截图、证据包或 evidence JSON。

## 当前无 PDA 时的推进口径

没有真 PDA 时，W6.D 只能推进“准备能力”，不能关闭 gate：

先运行 `just wave-3-pda-preaudit-kit --json` 给项目负责人确认推进边界；再运行 `just wave-3-pda-materials-checklist --json` 拆出字段、来源和负责人；然后运行 `just wave-3-pda-field-work-request` 生成可转发资源申请包，并运行 `just wave-3-pda-field-execution-summary --json` 汇总当前变量缺口；最后运行 `just wave-3-pda-service-precheck --json` 验证 dev/staging 服务前置，运行 `just wave-3-pda-trace-code-openapi-precheck --from-env --json` 验证追溯码查询 OpenAPI 前置，也可以运行 `just wave-3-pda-field-precheck-summary --from-env` 一次性汇总服务、追溯码和字段缺口。这些都只是准备能力，不写 evidence，也不能关闭 W6.D。

| 当前可做 | 不能做 |
|----------|--------|
| 用 `--service-precheck-only` 验证 dev/staging `/healthz` 和 Wave3 鉴权边界 | 不写 `docs/retros/wave-3-pda-runtime-evidence.json` |
| 导出并补齐现场采集模板 | 不把 `real_pda_used` 预填为 `true` |
| 准备 50 个脱敏条码样本、M2/M3 测试数据和测试账号 | 不把 `physical_scan_key_verified` 预填为 `true` |
| 用后端/API 测试覆盖 M2/M3、离线 replay payload 和 Idempotency-Key 语义 | 不把浏览器、模拟器、手机摄像头或本地脚本当成 PDA evidence |

真 PDA 到位后才执行 SPIKE-005 / SPIKE-005B 真机对比，采集扫码日志、离线 replay、幂等 replay、`audit_event` 查询、L7 执行记录和人工易用性走查记录，再进入 readiness、record check-only 和正式 record。

## W6.D 现场采集字段表

这张表用于现场执行前分工，不是 evidence JSON。无 PDA 阶段只能提前准备材料和检查环境；所有真机字段必须来自 dev/staging + 真 PDA 实扫结果。

| 字段 / 变量 | 来源 / 负责人 | 证据来源 | 无 PDA 阶段能否提前准备 |
|-------------|---------------|----------|--------------------------|
| `WAVE_3_PDA_ENVIRONMENT` | 运维 / 部署负责人 | dev/staging 环境标识，必须与所有证据引用中的环境 token 一致 | 可以，先确认本轮采集使用 dev 还是 staging |
| `WAVE_3_PDA_SERVICE_URL` | 运维 / 部署负责人 | dev/staging `wms-api` 地址，readiness 会检查 `/healthz` 和 Wave3 鉴权边界 | 可以，先用只读 readiness 验证服务前置 |
| `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` / `WAVE_3_PDA_TRACE_CODE_API_KEY` | 追溯码接口负责人 / 运维 | 追溯码 OpenAPI 只读合约和 `X-API-Key` 认证头；真实 key 只放环境变量或 secret 管理系统 | 可以提前准备并运行 trace-code OpenAPI precheck；不得把真实 key 写入仓库、截图或 evidence JSON |
| `WAVE_3_PDA_PDA_MODEL` / `WAVE_3_PDA_ANDROID_VERSION` | 设备借测 / 资产负责人 | 真 PDA 设备资产登记或现场照片归档 | 不能预填，必须等设备到位 |
| `WAVE_3_PDA_SCAN_INPUT_METHOD` | PDA 技术验证负责人 | 实体扫码键或厂商扫码通道记录，如 scan-key / KeyEvent / Intent / DataWedge | 不能用浏览器、模拟器、手机摄像头替代 |
| `WAVE_3_PDA_STACK_CANDIDATE` / `WAVE_3_PDA_SPIKE_RESULT_REF` | PDA 技术验证负责人 | SPIKE-005 或 SPIKE-005B 真机实测结论 | 可以提前决定候选测试计划，但不能提前写实测结论 |
| `WAVE_3_PDA_PDA_DEVICE_REF` | 资产负责人 | `asset://.../pda/...` 设备资产引用 | 只能在真 PDA 登记后填写 |
| `WAVE_3_PDA_M2_SCAN_LOG_REF` / `WAVE_3_PDA_M3_SCAN_LOG_REF` | 测试执行人 | 真 PDA 扫描 M2 / M3 流程的 dev/staging 日志引用 | 可以提前准备条码清单和 M2/M3 测试数据；不能提前生成日志引用 |
| `WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF` | 测试执行人 | 真 PDA 离线暂存后恢复网络 replay 的日志引用 | 可以提前准备断网/恢复步骤；不能用本地脚本 replay 替代 |
| `WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF` | 测试执行人 / 后端负责人 | 同一 `Idempotency-Key` replay 的真实日志引用 | 可以提前准备重放用例；不能提前标记通过 |
| `WAVE_3_PDA_AUDIT_EVENT_QUERY_REF` | 后端 / 数据库操作人 | 查询 H2 `audit_event` 中 M2/M3/PDA 操作落库的证据引用 | 只有真机执行后才能查询到对应事件 |
| `WAVE_3_PDA_L7_RUN_REF` / `WAVE_3_PDA_USABILITY_REVIEW_REF` | 测试负责人 / 业务走查人 | L7 执行记录和人工易用性走查记录；操作员现场走查清单归档到 `WAVE_3_PDA_USABILITY_REVIEW_REF` | 可以提前准备走查表；不能提前标记完成 |
| `WAVE_3_PDA_NATIVE_SHELL_REF` / `WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF` | PDA 技术验证负责人 | WebView/Capacitor 候选的 Android native shell 与 native scan plugin 证据 | 只在 `WAVE_3_PDA_STACK_CANDIDATE=webview-capacitor` 时填写；不能提前写实测结论 |
| `WAVE_3_PDA_BARCODE_SAMPLES_SCANNED` / `WAVE_3_PDA_M2_OPERATIONS_EXERCISED` / `WAVE_3_PDA_M3_OPERATIONS_EXERCISED` | 测试负责人 / 后端负责人 | 现场扫码样本、M2 操作、M3 操作计数 | 可以提前准备目标清单和测试数据；不能提前计数 |
| `WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED` / `WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED` | 测试负责人 / 后端负责人 | 现场离线 replay 和 `Idempotency-Key` replay 计数，目标各 50 次 | 可以提前准备执行步骤；不能提前计数 |
| `WAVE_3_PDA_REAL_PDA_USED` / `WAVE_3_PDA_PHYSICAL_SCAN_KEY_VERIFIED` | 现场负责人确认 | 真 PDA 与实体扫码键证据到位后确认 | 只能在真 PDA 实扫后置为 `true` |
| `WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED` | 运维 / 部署负责人 | service-precheck 或 readiness 输出确认 dev/staging 服务前置可达 | 可以在无 PDA 阶段先验证；不能替代真 PDA evidence |
| `WAVE_3_PDA_AUDIT_EVENT_VERIFIED` / `WAVE_3_PDA_L7_REVIEW_COMPLETED` / `WAVE_3_PDA_USABILITY_REVIEW_COMPLETED` | 后端 / 测试负责人 / 业务走查人 | 对应 `audit_event` 查询、L7 执行记录和易用性走查记录全部到位后确认 | 只能在对应真实 evidence 到位后置为 `true` |

## W6.D 角色-命令-预期输出表

这张表用于现场排班。除正式 record 外，其余命令都是只读准备或预检；看到 `ok=true` 只能说明该步骤前置通过，不能单独关闭 W6.D。

| 顺序 | 角色 | 命令 / 动作 | 预期输出 | 失败时先看 |
|------|------|-------------|----------|------------|
| 1 | 测试负责人 | `just wave-3-pda-preaudit-kit --json` | `ok=true`，输出 `now_actions`、`current_env_status`、`blocked_until_real_pda` | `missing_now_env_vars` 和 `missing_now_env_var_owners` |
| 2 | 测试负责人 | `just wave-3-pda-materials-checklist --json` | `ok=true`，字段表覆盖所有 `WAVE_3_PDA_*` 变量和负责人 | 是否漏填运维、资产、测试、后端、业务走查责任人 |
| 3 | 测试负责人 | `just wave-3-pda-field-work-request --json` | 输出 `resources`、`execution_order_zh`、`troubleshooting`、`next_commands` | 资源项是否缺真 PDA、追溯码 OpenAPI、M2/M3 数据或 L7 走查人 |
| 4 | 测试负责人 | `just wave-3-pda-field-execution-summary --json` | 输出 `current_env_status`、`real_pda_missing_env_vars`、`real_pda_missing_env_var_owners`、`truth_flag_env_vars`、`no_pda_precheck_truth_flag_env_vars`、`satisfied_by_precheck_attachment_truth_flag_env_vars`、`false_truth_flag_env_vars`、`false_truth_flag_env_var_owners`、`false_no_pda_precheck_truth_flag_env_vars`、`false_no_pda_precheck_truth_flag_env_var_owners`、`false_real_evidence_truth_flag_env_vars`、`false_real_evidence_truth_flag_env_var_owners`、`record_commands` | 是否仍缺当前可准备变量；owner 明细只用于定位负责人，不输出变量值；`WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED` 可由 service-precheck 附件支撑，但正式 evidence payload 仍必须写入 `dev_or_staging_service_verified=true`；真机布尔变量仍必须等对应真实 evidence |
| 5 | 运维 / 部署负责人 | 设置 `WAVE_3_PDA_ENVIRONMENT`、`WAVE_3_PDA_SERVICE_URL`，运行 `just wave-3-pda-service-precheck --from-env --json` | `ok=true`，`/healthz` 为 200，Wave3 受保护路由无 token 返回 401 / `AUTH-001` | 服务 URL 是否是 dev/staging，是否误用 local / prod / production |
| 6 | 追溯码接口负责人 / 运维 | 设置 `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`、`WAVE_3_PDA_TRACE_CODE_API_KEY`，运行 `just wave-3-pda-trace-code-openapi-precheck --from-env --json` | `ok=true`，OpenAPI status 200，OpenAPI 3.0.3，5 个 required GET/POST operations 存在，认证头为 `X-API-Key` | OpenAPI URL、网络、key 权限；不要把 key 粘进日志或截图 |
| 6a | 测试负责人 | `just wave-3-pda-field-precheck-summary --from-env`；结构化附件用 `just wave-3-pda-field-precheck-summary --from-env --json` | 默认输出 Markdown 总览；JSON 中 `ok=true` 时 `service_precheck` 与 `trace_code_openapi_precheck` 都通过，并附 `field_execution_summary` 当前缺口；`writes_runtime_evidence=false`、`closes_gate=false` | `issues` 中的 `service:` / `trace-code-openapi:` 前缀；这只是现场总览附件，不是 evidence JSON |
| 6b | 测试负责人 | `just wave-3-pda-field-owner-gap-actions`；结构化附件用 `just wave-3-pda-field-owner-gap-actions --json` | 默认输出 Markdown 派单表，分列 `Missing now`、`Real evidence vars` 和 `False flags`；JSON 输出按 `source_owner` 聚合的 `field_owner_gap_actions`，包含 `env_vars`、`evidence_requirements`、`no_pda_stages` 和 `requires_real_pda` | 哪个负责人仍缺什么变量、证据要求和布尔确认；这只是派单材料，不是 evidence JSON |
| 6c | 测试负责人 | `just wave-3-pda-field-handoff-bundle --json`；带预检结果用 `just wave-3-pda-field-handoff-bundle --from-env --json`；归档附件用 `--field-handoff-output docs/retros/wave-3-pda-field-handoff-2026-06-14.json`，确认覆盖旧附件时才加 `--field-handoff-force` | 输出一份现场交接总包，聚合 `preaudit_kit`、`materials_checklist`、`field_work_request`、`field_execution_summary`、`field_owner_gap_actions`、`evidence_package_template` 和 `intake_template`；`writes_runtime_evidence=false`、`closes_gate=false` | 默认不联网；`--from-env` 模式只读探测服务和 OpenAPI，不输出 key；`--field-handoff-output` 只写交接附件，仍不能当作 evidence JSON；默认拒绝覆盖已有文件 |
| 7 | 资产负责人 / PDA 技术验证负责人 | 登记 PDA 型号、Android 版本、`asset://.../pda/...` 引用和 scan-key / KeyEvent / Intent / DataWedge | 设备资产引用和扫码输入方式可填写到 `WAVE_3_PDA_*` | 设备是否为真 PDA，扫码输入是否来自实体扫码键或厂商通道 |
| 8 | 测试执行人 / 后端负责人 | 用真 PDA 执行 M2/M3 scan、offline replay、Idempotency-Key replay，并归档 `audit_event` 查询 | M2/M3/replay 日志和 `audit_event` 查询引用全部包含 dev/staging | 日志引用是否缺场景名、环境、trace ID、resource ID |
| 9 | 测试负责人 / 业务走查人 | 填写 L7 执行记录和易用性走查记录，运行 `just wave-3-pda-runtime-readiness --from-env --json` | `ok=true`，字段和服务前置全部通过 | 布尔变量是否在对应真实引用到位后才设为 `true` |
| 10 | 测试负责人 | `just wave-3-pda-runtime-evidence-record --from-env --check-only --json` | `ok=true`，正式 recorder 预检通过但不写 evidence JSON | `missing_args`、`missing_env_vars`、引用边界错误 |
| 11 | 测试负责人 | from-env 路径运行 `just wave-3-pda-runtime-evidence-record --from-env --json`，或 intake 路径运行 `just wave-3-pda-intake-check --json` 后 `just wave-3-pda-intake-record --json`；二选一完成后运行 `just wave-3-pda-runtime-evidence-validate` | 生成 `docs/retros/wave-3-pda-runtime-evidence.json`，validator 通过 | evidence 文件已存在时不要直接覆盖，先按异常修正流程确认；不要连续执行两个正式 record 路径 |

现场准备顺序：

1. 测试负责人先运行 `just wave-3-pda-preaudit-kit --json`，把可推进项、真机阻塞项和禁止事项发给项目负责人确认。
2. 测试负责人运行 `just wave-3-pda-materials-checklist --json`，按字段清单分派运维、资产、测试、后端和业务走查责任人。
3. 测试负责人运行 `just wave-3-pda-field-work-request`，把资源申请包转给业务方、运维、设备方和测试执行人。
4. 测试负责人运行 `just wave-3-pda-field-execution-summary --json`，确认当前变量缺口、真机字段和后续 record 命令顺序。
5. 运维提供 `WAVE_3_PDA_SERVICE_URL`，执行只读 service precheck 验证服务前置。
6. 追溯码接口负责人 / 运维提供 `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` 和 `WAVE_3_PDA_TRACE_CODE_API_KEY`，执行只读 trace-code OpenAPI precheck。
7. 测试负责人运行 `just wave-3-pda-field-precheck-summary --from-env`，把服务、追溯码和字段缺口汇总成 Markdown 现场总览；需要结构化附件时再运行 `just wave-3-pda-field-precheck-summary --from-env --json`。
8. 测试负责人运行 `just wave-3-pda-field-owner-gap-actions`，把 Markdown 缺口表按 owner 派给运维、资产、测试执行人、后端和业务走查人；需要结构化附件时再运行 `just wave-3-pda-field-owner-gap-actions --json`。
9. 测试负责人运行 `just wave-3-pda-field-handoff-bundle --json` 生成现场交接总包；需要纳入服务和追溯码 OpenAPI 预检结果时运行 `just wave-3-pda-field-handoff-bundle --from-env --json`。
10. 业务方或资产负责人提供至少一台 PDA，并登记型号、Android 版本、扫码输入方式和 `asset://.../pda/...` 引用。
11. 测试负责人准备 50 个脱敏条码样本、M2/M3 测试数据、具备 `m2.write` / `m3.write` 权限的测试账号。
12. PDA 技术验证负责人用同一设备、同一条码样本、同一 M2/M3 测试数据执行 SPIKE-005 / SPIKE-005B。
13. 后端负责人归档 M2/M3 scan、offline replay、Idempotency-Key replay 和 `audit_event` 查询引用。
14. 测试负责人补 L7 执行记录和人工易用性走查记录，再选择 from-env 或 intake 路径执行 check-only、正式 record、validate；两个正式 record 路径二选一。

## W6.D 预审包

`just wave-3-pda-preaudit-kit --json` 是无 PDA 阶段的第一份汇总材料，可直接给项目负责人确认当前推进方式。它不会联网，不会写 runtime evidence，不会关闭 W6.D gate。

预审包包含：

| 内容 | 用途 |
|------|------|
| `now_actions` | 列出现在就能推进的动作：确认 dev/staging URL、确认追溯码 OpenAPI URL / API key 并完成只读预检、准备 50 个脱敏条码样本和 M2/M3 测试数据、导出 evidence package 模板、借测或采购真 PDA |
| `current_env_status` | 当前 shell 的前置环境变量状态；只输出 `required_now_env_vars`、`set_now_env_vars`、`missing_now_env_vars` 和 `missing_now_env_var_owners`，不输出环境变量值 |
| `blocked_until_real_pda` | 列出必须等真 PDA 实扫后才能填写的 `WAVE_3_PDA_*` 字段，如 `WAVE_3_PDA_PDA_MODEL`、`WAVE_3_PDA_SCAN_INPUT_METHOD`、`WAVE_3_PDA_M2_SCAN_LOG_REF`、`WAVE_3_PDA_AUDIT_EVENT_QUERY_REF`、`WAVE_3_PDA_L7_RUN_REF`、`WAVE_3_PDA_USABILITY_REVIEW_REF` |
| `must_not_do` | 列出禁止事项：不得伪造 evidence JSON，不得用浏览器 / 模拟器 / 手机摄像头替代真 PDA，不得提前把布尔变量设为 `true` |
| `next_commands` | 给出后续短命令顺序，继续进入 materials checklist、field-work-request、field-execution-summary、evidence package、service precheck、trace-code OpenAPI precheck、readiness、record check-only、record 和 validate |

`current_env_status.required_now_env_vars` 当前要求 `WAVE_3_PDA_ENVIRONMENT`、`WAVE_3_PDA_SERVICE_URL`、`WAVE_3_PDA_TRACE_CODE_OPENAPI_URL` 和 `WAVE_3_PDA_TRACE_CODE_API_KEY`。如果 dev/staging 变量缺失，`missing_now_env_var_owners` 会指向运维 / 部署负责人；如果 trace-code 变量缺失，会指向追溯码接口负责人 / 运维。预审包只显示变量名是否已设置，不打印 URL、API key 或其他环境变量值。

Markdown 版可运行：

```bash
just wave-3-pda-preaudit-kit
```

## W6.D 现场资源申请包

这张表可直接转给业务方、运维、设备方和测试负责人做资源申请与分工。它不能把这张表当作 evidence JSON，也不能替代真 PDA 执行记录；只能用于推动现场材料到位。

可用命令导出同一内容：

```bash
just wave-3-pda-field-work-request
```

补充生成当前执行摘要：

```bash
just wave-3-pda-field-execution-summary --json

just wave-3-pda-field-precheck-summary --from-env

just wave-3-pda-field-precheck-summary --from-env --json

just wave-3-pda-field-owner-gap-actions

just wave-3-pda-field-owner-gap-actions --json

just wave-3-pda-field-handoff-bundle --json
```

| 资源项 | 负责人 | 交付物 | 验证命令 / 证据变量 |
|--------|--------|--------|----------------------|
| dev/staging 服务地址 | 运维 / 部署负责人 | `wms-api` 地址，环境只能是 dev 或 staging | `WAVE_3_PDA_SERVICE_URL`；先跑 `just wave-3-pda-service-precheck --from-env --json` |
| 追溯码 OpenAPI 合约 | 追溯码接口负责人 / 运维 | 只读 OpenAPI 地址和仓库外 secret 管理的 API key | `WAVE_3_PDA_TRACE_CODE_OPENAPI_URL`、`WAVE_3_PDA_TRACE_CODE_API_KEY`；先跑 `just wave-3-pda-trace-code-openapi-precheck --from-env --json`；不得把真实 key 写入仓库或 evidence JSON |
| 至少一台真 PDA | 业务方 / 资产负责人 / 设备方 | PDA 型号、Android 版本、现场照片或资产登记 | `WAVE_3_PDA_PDA_MODEL`、`WAVE_3_PDA_ANDROID_VERSION`、`WAVE_3_PDA_PDA_DEVICE_REF`，设备引用必须是 `asset://.../pda/...` |
| 实体扫码键 / 厂商扫码通道 | PDA 技术验证负责人 | scan-key / KeyEvent / Intent / DataWedge 等输入方式说明 | `WAVE_3_PDA_SCAN_INPUT_METHOD`，不能用浏览器、模拟器、手机摄像头替代 |
| 50 个脱敏条码样本 | 测试负责人 / 业务数据负责人 | GS1 追溯码、Code128 批号 / 箱码、二维码任务号样本清单 | `WAVE_3_PDA_BARCODE_SAMPLES_SCANNED`，目标沿用 50 |
| M2/M3 测试数据 | 后端 / 测试负责人 | M2 收货 / 验收 / 上架、M3 库存查询 / 状态变更可重建数据 | `WAVE_3_PDA_M2_OPERATIONS_EXERCISED`、`WAVE_3_PDA_M3_OPERATIONS_EXERCISED` |
| 离线 replay 执行条件 | 测试负责人 | 断网、恢复网络、离线队列 replay 的执行记录 | `WAVE_3_PDA_OFFLINE_REPLAY_LOG_REF`、`WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED` |
| Idempotency-Key replay 条件 | 测试负责人 / 后端负责人 | 同一 `Idempotency-Key` 首次请求和重放请求日志 | `WAVE_3_PDA_IDEMPOTENCY_REPLAY_LOG_REF`、`WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED` |
| H2 `audit_event` 查询 | 后端 / 数据库操作人 | M2/M3/PDA scan、offline replay、idempotency replay 对应审计查询引用 | `WAVE_3_PDA_AUDIT_EVENT_QUERY_REF` |
| L7 执行人 | 测试负责人 | L7 执行记录，记录实测事实，不设本地阈值 | `WAVE_3_PDA_L7_RUN_REF` |
| WebView/Capacitor Android native shell | PDA 技术验证负责人 | 当技术栈候选为 webview-capacitor 时，归档 Android native shell 真机证据 | `WAVE_3_PDA_NATIVE_SHELL_REF` |
| WebView/Capacitor native scan plugin | PDA 技术验证负责人 | 当技术栈候选为 webview-capacitor 时，归档 native scan plugin 实体扫码键证据 | `WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF` |
| 人工易用性走查人 | 业务走查人 / 测试负责人 | 设备握持、扫码键触达、扫码反馈、离线提示、错误提示、恢复网络确认路径和结论 | `WAVE_3_PDA_USABILITY_REVIEW_REF` |

快速排障顺序：

1. 如果正式 `record` 提示 evidence JSON 已存在，先备份或确认替换范围，再按需追加 `--force` 重跑。
   正常 closeout 不追加 `--force`；只有需要修正已写入 evidence 时，先保留原 evidence 引用，再确认替换。
2. 如果 service precheck 失败，先确认 `/healthz` 为 HTTP 200 且 payload `status=ok`，再确认 Wave3 路由无鉴权返回 `401` / `AUTH-001`。
3. 如果 `WAVE_3_PDA_SCAN_INPUT_METHOD` 校验失败，改用包含 scan-key / KeyEvent / Intent / DataWedge 的写法，不写 camera / phone / browser。
4. 如果 Spike 引用校验失败，确认 `react-native` 对应 SPIKE-005，`webview-capacitor` 对应 SPIKE-005B。
5. 如果证据引用校验失败，确认每个引用路径都显式包含 `dev` 或 `staging`，不要写 `stg`、prod、local、mock、fake、stub 或 example。
6. 如果 readiness 或 check-only 提示布尔项仍为 false，先补齐对应证据，再把对应 `WAVE_3_PDA_*` 布尔变量设为 `true`。

## W6.D 证据包最小内容

Evidence JSON 只保存引用，不保存大体积日志、截图或视频。每个日志或文档引用至少记录以下内容，便于 reviewer 复核引用是真实 dev/staging + 真 PDA 采集结果：

现场执行前可运行 `just wave-3-pda-evidence-package-template` 输出 Markdown 证据包模板，或运行 `just wave-3-pda-evidence-package-template --json` 输出结构化证据包模板；JSON 中的 `owner_actions` 可直接转成现场派单。模板本身不是 evidence，必须填写真实 dev/staging + 真 PDA 执行事实并归档后，才能把对应引用写入 `WAVE_3_PDA_*` 变量或同一份 JSON intake。正式 record 可用 `--from-env` 从环境变量构造 evidence payload，也可用 `just wave-3-pda-intake-record --json` 从 intake 文件构造 evidence payload；两条正式 record 路径二选一。`WAVE_3_PDA_SERVICE_URL` 只用于 service-precheck / readiness，不进入 evidence JSON。

如果需要把预审包、材料清单、资源申请、owner 缺口、证据包模板和 JSON intake 模板统一交给现场系统，运行 `just wave-3-pda-field-handoff-bundle --json`；如果还需要把当前 service precheck 和 trace-code OpenAPI precheck 结果纳入同一附件，运行 `just wave-3-pda-field-handoff-bundle --from-env --json`；如果需要归档成文件，追加 `--field-handoff-output docs/retros/wave-3-pda-field-handoff-2026-06-14.json --json`。已有交接附件不会被默认覆盖，确认重生成时才追加 `--field-handoff-force`。这些命令只生成交接总包，不写 runtime evidence，也不能关闭 W6.D gate。JSON 输出中的 `intake_template` 可直接交给现场系统生成待填 intake 文件；也可以用 `just wave-3-pda-intake-template --json --intake-template-output docs/retros/wave-3-pda-intake-template-2026-06-14.json` 单独落盘，已有模板文件默认拒绝覆盖，确认重生成才追加 `--intake-template-force`。它仍然只是模板，不是 `docs/retros/wave-3-pda-runtime-evidence.json`。

| 材料类型 | 最小内容 |
|----------|----------|
| 通用日志 / 文档引用 | 环境（dev 或 staging）、PDA 设备资产引用、执行时间、测试账号 / 租户、场景名称、关键业务 ID、结果摘要 |
| M2 / M3 scan 日志 | 条码样本批次、扫描入口、请求 trace ID、业务单据 ID、接口响应、对应 `audit_event` resource ID |
| offline replay 日志 | 断网开始 / 恢复时间、离线队列条数、replay 顺序、成功 / 失败结果、冲突处理摘要 |
| Idempotency-Key replay 日志 | 首次请求 ID、重放请求 ID、相同 `Idempotency-Key`、响应一致性摘要 |
| L7 执行记录 | 设备型号、网络条件、M2/M3 场景、样本数量、执行人、结果摘要；不设本地性能阈值，只记录实测事实 |
| 操作员现场走查清单 | 操作员角色、设备握持和扫码键触达、扫码反馈、离线提示、错误提示、恢复网络后的确认路径、走查结论 |
| 追溯码 OpenAPI 预检附件 | `trace-code OpenAPI precheck` 输出、OpenAPI 3.0.3、必需 GET/POST operations 摘要、`X-API-Key` header scheme 摘要、附件归档引用；`WAVE_3_PDA_TRACE_CODE_API_KEY` 不得写入证据包、截图或 evidence JSON |

现场执行时可以保存 readiness `--json` 输出作为证据包附件，用来说明服务前置和字段预检状态；但不能把 readiness 输出当作关闭 W6.D gate 的 evidence。也可以保存 trace-code OpenAPI precheck `--json` 输出作为准备附件，用来说明追溯码查询接口前置已检查；但不能把 trace-code OpenAPI precheck 输出当作关闭 W6.D gate 的 evidence。正式关闭仍以 `docs/retros/wave-3-pda-runtime-evidence.json` 通过 validator 为准。

## W6.D L7 与易用性走查模板

以下模板用于现场归档材料，不是 evidence JSON。L7 只记录实测事实，不设本地性能阈值；操作员现场走查清单归档为 `WAVE_3_PDA_USABILITY_REVIEW_REF`，L7 执行记录归档为 `WAVE_3_PDA_L7_RUN_REF`。

L7 执行记录最小模板：

| 项目 | 填写要求 |
|------|----------|
| 执行人 | 记录姓名或工号；不得只写“测试人员” |
| 执行时间 | 使用 dev/staging 证据库时间戳 |
| 环境 | 只能是 dev 或 staging，必须和 evidence JSON `environment` 一致 |
| 设备型号 | 和 `WAVE_3_PDA_PDA_MODEL` 一致 |
| Android 版本 | 和 `WAVE_3_PDA_ANDROID_VERSION` 一致 |
| 网络条件 | 记录仓库 Wi-Fi / 蜂窝网络 / 断网恢复步骤 |
| 条码样本批次 | 对应 50 个脱敏条码样本清单 |
| M2 scan | 记录业务单据 ID、请求 trace ID、响应摘要 |
| M3 scan | 记录库存批次 / 状态变更 ID、请求 trace ID、响应摘要 |
| offline replay | 记录离线队列条数、replay 顺序、结果摘要 |
| Idempotency-Key replay | 记录首次请求 ID、重放请求 ID、响应一致性摘要 |
| `audit_event` resource ID | 记录可查询到的审计事件 resource ID |
| 结果摘要 | 只写实测事实；不得用 readiness 结果替代真机执行记录 |

操作员现场走查清单最小模板：

| 项目 | 填写要求 |
|------|----------|
| 操作员角色 | 记录保管员 / 复核员 / 测试执行人等现场角色 |
| 设备握持 | 记录单手 / 双手握持是否影响扫描和确认动作 |
| 扫码键触达 | 记录实体扫码键或厂商扫码通道是否可稳定触发 |
| 扫码反馈 | 记录声音 / 震动 / 页面状态反馈是否可识别 |
| 离线提示 | 记录断网时页面是否明确提示离线暂存 |
| 错误提示 | 记录扫错码、重复码、无权限或接口失败时的提示 |
| 恢复网络确认 | 记录恢复网络后 replay 结果是否可被操作员确认 |
| 走查结论 | 记录通过 / 不通过 / 需整改；必须附问题 ID 或日志引用 |

## W6.D 现场短命令

先运行 `just wave-3-pda-runtime-evidence-record --export-template`，填写其中所有 `WAVE_3_PDA_*` 变量。`WAVE_3_PDA_BARCODE_SAMPLES_SCANNED`、`WAVE_3_PDA_M2_OPERATIONS_EXERCISED`、`WAVE_3_PDA_M3_OPERATIONS_EXERCISED`、`WAVE_3_PDA_OFFLINE_REPLAYS_EXERCISED` 与 `WAVE_3_PDA_IDEMPOTENCY_REPLAYS_EXERCISED` 都由 `--from-env` 读取，不再手工拼长参数。

如果 `WAVE_3_PDA_STACK_CANDIDATE=webview-capacitor`，from-env 路径的 readiness、record check-only 和正式 record 都通过 `--from-env` 读取 `WAVE_3_PDA_NATIVE_SHELL_REF` 与 `WAVE_3_PDA_NATIVE_SCAN_PLUGIN_REF`；intake 路径则通过 `just wave-3-pda-intake-check --json` 和 `just wave-3-pda-intake-record --json` 从同一份 intake 文件读取这些 native refs。正式 record 使用 `--json` 输出写入结果，便于现场归档。

`WAVE_3_PDA_DEV_OR_STAGING_SERVICE_VERIFIED` 可以在 service precheck 输出归档后设为 `true`。真机、扫码键、审计、L7 和易用性布尔确认只能在对应真实 evidence 到位后设为 `true`；无 PDA 阶段不要把这些真实 evidence 变量预填为 `true`。

```bash
just wave-3-pda-service-precheck --from-env --json

just wave-3-pda-trace-code-openapi-precheck --from-env --json

just wave-3-pda-field-precheck-summary --from-env

just wave-3-pda-field-precheck-summary --from-env --json

just wave-3-pda-field-handoff-bundle --json

just wave-3-pda-runtime-readiness --from-env --json

just wave-3-pda-runtime-evidence-record --from-env --check-only --json

just wave-3-pda-runtime-evidence-record --from-env --json

just wave-3-pda-intake-check --json

just wave-3-pda-intake-record --json

just wave-3-pda-runtime-evidence-validate
```

## 输出

- SPIKE-005 / SPIKE-005B §7 追加本轮决策与实测结果
- 如蓝牙打印硬件未在 W3 PDA Spike 中到位，SPIKE-005 / SPIKE-005B §7 必须记录 defer 到 Wave 5，并由 `docs/retros/wave-5-hardware-evidence.json` 关闭打印硬件证据
- 如 accept：更新 ADR-0027 PDA 离线模型与技术栈定版框架，并在满足 evidence 前置后改为 Accepted
- 如 reject：记录替代方案，例如 native Android + bridge，或保留另一条已通过的 Spike 路线
- 如 defer：更新 ROADMAP backlog 与下一次启动条件
