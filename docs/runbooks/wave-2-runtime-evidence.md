# Wave 2 Pre-release Runtime Evidence Runbook

> 适用范围：Wave 2 W2.G 配置中心版 Feature Flag 灰度链路。当前没有稳定 dev/staging 时，本 runbook 是预发布 gate，不阻断 Wave 2 开发完成。

## 目标

证明 W1 文件版 Feature Flag 已迁移到 M1-008 配置中心，且配置中心读取源在真实 dev/staging 环境可用：

- 文件版 flag 已迁移到配置中心
- 批量导入、导出、对账、切换读取源、旧文件归档链路可执行
- 对账结果无 `missing_in_config_center` / `mismatched`
- 应用读取源已切到 `config_center`
- 证据来自真实 dev 或 staging，不使用 local / prod / production / mock / fake / stub / example

## 前置条件

- 已部署包含 Wave 2 OpenAPI 的 `wms-api`
- 环境为 `dev` 或 `staging`
- 有一份当前生效的 W1 文件版 flag 快照
- H1 鉴权可用于调用配置中心接口
- 本次 W6.C evidence payload 不校验 H2 审计字段；生产环境仍应按 H2 治理要求记录 flag 迁移、导入、切源和归档操作

如果当前没有可用环境，先按 [Wave 2 Staging Environment Runbook](wave-2-staging-environment.md) 搭建测试用途的 `staging` 环境；不要把 evidence 环境值写成 `test`。

## 已确认范围

- 3A：接受当前内存态 config-center 作为 W6.C runtime smoke 范围；PostgreSQL 持久化配置中心另行立项，不作为本次关闭 W6.C 的前置条件。
- 4A：H2 审计不纳入本次 W6.C evidence payload；本次只验证配置中心迁移、对账、导出、切源、业务 smoke 和旧文件归档链路。
- 5A：`just wave-2-runtime-evidence-validate` 保持 Wave2 静态完成项 + runtime evidence 的全量 gate，不改成纯 runtime JSON validator。

## 现场环境变量

运行 readiness / smoke 前由现场 shell 注入以下变量，值必须包含当前 `environment` 标记（`dev` 或 `staging`），且不能指向 local / prod / production / mock / fake / stub / example：

```bash
export WAVE_2_ENVIRONMENT="$WAVE_2_ENVIRONMENT"
export WAVE_2_SERVICE_URL="$WAVE_2_SERVICE_URL"
export WAVE_2_H1_TOKEN="$WAVE_2_H1_TOKEN"
export WAVE_2_SMOKE_LOG_REF="$WAVE_2_SMOKE_LOG_REF"
export WAVE_2_RECONCILE_LOG_REF="$WAVE_2_RECONCILE_LOG_REF"
export WAVE_2_ARCHIVE_REF="$WAVE_2_ARCHIVE_REF"
```

## 执行步骤

1. 调用 `POST /api/v1/config-center/feature-flags/source`，请求 `config_center` 读取源。
2. 在迁移前调用 `GET /api/v1/inventory/batches`，确认未命中 flag 返回 `M1_CONFIG_FLAG_MISSING`，证明 fail-closed。
3. 调用 `POST /api/v1/config-center/feature-flags/migrate`，从 W1 文件版 flag 迁移到 M1-008。
4. 如需人工补录或跨环境导入，调用 `POST /api/v1/config-center/feature-flags/import`。
5. 调用 `GET /api/v1/config-center/feature-flags/reconcile`，确认：
   - `missing_in_config_center = []`
   - `mismatched = []`
6. 调用 `GET /api/v1/config-center/feature-flags/export`，确认导出结果包含 `m3_inventory_batches_config_center_smoke` 并保存导出结果引用。
7. 再次调用 `POST /api/v1/config-center/feature-flags/source`，确认读取源保持 `config_center`。
8. 迁移并切到 `config_center` 后，调用 `GET /api/v1/inventory/batches`，确认 `m3_inventory_batches_config_center_smoke` 启用读取且返回 200；smoke 必须使用带 `m3.read` 权限的真实 H1 token，并且 `wms-api` 必须连接真实 PostgreSQL。
9. 调用 `POST /api/v1/config-center/feature-flags/archive-file-source`，记录旧文件归档引用。
10. 先运行 readiness，只校验输入边界和 H1 token 环境变量已注入；不会解析 JWT 签名、权限或过期时间，不发 HTTP，也不写 `docs/retros/wave-2-runtime-evidence.json`：

```bash
just wave-2-runtime-evidence-readiness
```

11. 运行真实 smoke 采集命令，自动执行“切源到 `config_center` → fail-closed → migrate → reconcile → export → 切源到 `config_center` → 业务 200 smoke → 旧文件归档”，并写入 `docs/retros/wave-2-runtime-evidence.json`：

```bash
just wave-2-runtime-evidence-smoke
```

`just wave-2-runtime-evidence-collect` 是同一 collector 的兼容别名；新的 Wave 6 closeout 清单以 `just wave-2-runtime-evidence-smoke` 为正式采集入口。

如果 `docs/retros/wave-2-runtime-evidence.json` 已存在，smoke 默认拒绝覆盖，防止误写旧证据。确需重跑时，必须先确保 config-center 状态已回到可验证 fail-closed 的新实例或重置状态，再显式使用 collector 的 `--force` 选项。

12. 再运行验证：

```bash
just wave-2-runtime-evidence-validate
```

手动 `record` 只用于现场 smoke 已由外部系统执行、但需要补写同一份 JSON 的备选路径。正常 closeout 优先使用 `smoke`：

所有 `service_url` / log / archive 证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / production / mock / fake / stub / example。

```bash
just wave-2-runtime-evidence-record \
  --environment "$WAVE_2_ENVIRONMENT" \
  --service-url "$WAVE_2_SERVICE_URL" \
  --migrated-count 1 \
  --reconcile-matched 1 \
  --business-smoke-path /api/v1/inventory/batches \
  --business-smoke-enabled-flag m3_inventory_batches_config_center_smoke \
  --business-smoke-success-status 200 \
  --business-smoke-fail-closed-error-code M1_CONFIG_FLAG_MISSING \
  --smoke-log-ref "$WAVE_2_SMOKE_LOG_REF" \
  --reconcile-log-ref "$WAVE_2_RECONCILE_LOG_REF" \
  --archive-ref "$WAVE_2_ARCHIVE_REF"

just wave-2-runtime-evidence-validate
```

## Evidence JSON

以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

```json
{
  "environment": "dev",
  "service_url": "https://wms-dev.internal",
  "source_switched_to": "config_center",
  "migrated_count": 1,
  "reconcile": {
    "matched": 1,
    "missing_in_config_center": [],
    "mismatched": []
  },
  "business_smoke": {
    "path": "/api/v1/inventory/batches",
    "enabled_flag": "m3_inventory_batches_config_center_smoke",
    "success_status": 200,
    "fail_closed_error_code": "M1_CONFIG_FLAG_MISSING"
  },
  "smoke_log_ref": "ci/dev/wave2-feature-flags-smoke/123",
  "reconcile_log_ref": "ci/dev/wave2-feature-flags-reconcile/123",
  "archive_ref": "s3://wms-dev-audit/feature-flags/2026-06-03/feature_flags.toml"
}
```

## 拒绝边界

- `environment` 不是 `dev` 或 `staging`
- 任一引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example`
- 任一引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`
- `source_switched_to` 不是 `config_center`
- 对账结果存在缺失或不一致
- `business_smoke` 未记录 `/api/v1/inventory/batches` 的 200 成功与 `M1_CONFIG_FLAG_MISSING` fail-closed
- 缺少 smoke 或 reconcile 日志引用
