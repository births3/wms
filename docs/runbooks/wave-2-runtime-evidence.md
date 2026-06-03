# Wave 2 Pre-release Runtime Evidence Runbook

> 适用范围：Wave 2 W2.G 配置中心版 Feature Flag 灰度链路。当前没有稳定 dev/staging 时，本 runbook 是预发布 gate，不阻断 Wave 2 开发完成。

## 目标

证明 W1 文件版 Feature Flag 已迁移到 M1-008 配置中心，且配置中心读取源在真实 dev/staging 环境可用：

- 文件版 flag 已迁移到配置中心
- 批量导入、导出、对账、切换读取源、旧文件归档链路可执行
- 对账结果无 `missing_in_config_center` / `mismatched`
- 应用读取源已切到 `config_center`
- 证据来自真实 dev 或 staging，不使用 local / mock / fake / example / prod

## 前置条件

- 已部署包含 Wave 2 OpenAPI 的 `wms-api`
- 环境为 `dev` 或 `staging`
- 有一份当前生效的 W1 文件版 flag 快照
- H1 鉴权可用于调用配置中心接口
- H2 审计追踪可记录 flag 迁移、导入、切源和归档操作

## 执行步骤

1. 调用 `POST /api/v1/config-center/feature-flags/migrate`，从 W1 文件版 flag 迁移到 M1-008。
2. 如需人工补录或跨环境导入，调用 `POST /api/v1/config-center/feature-flags/import`。
3. 调用 `GET /api/v1/config-center/feature-flags/reconcile`，确认：
   - `missing_in_config_center = []`
   - `mismatched = []`
4. 调用 `GET /api/v1/config-center/feature-flags/export`，保存导出结果引用。
5. 调用 `POST /api/v1/config-center/feature-flags/source`，请求 `config_center` 读取源。
6. 跑一次真实 smoke：至少覆盖一个已迁移 flag 的启用读取和一个未命中 flag 的 fail-closed 行为。
7. 调用 `POST /api/v1/config-center/feature-flags/archive-file-source`，记录旧文件归档引用。
8. 写入 `docs/retros/wave-2-runtime-evidence.json`，再运行：

```bash
just wave-2-runtime-evidence-validate
```

## Evidence JSON

```json
{
  "environment": "dev",
  "service_url": "https://wms-dev.example.internal",
  "source_switched_to": "config_center",
  "migrated_count": 1,
  "reconcile": {
    "matched": 1,
    "missing_in_config_center": [],
    "mismatched": []
  },
  "smoke_log_ref": "ci/dev/wave2-feature-flags-smoke/123",
  "reconcile_log_ref": "ci/dev/wave2-feature-flags-reconcile/123",
  "archive_ref": "s3://wms-dev-audit/feature-flags/2026-06-03/feature_flags.toml"
}
```

## 拒绝边界

- `environment` 不是 `dev` 或 `staging`
- 任一引用包含 `localhost`、`127.0.0.1`、`example.com`、`prod`、`production`
- `source_switched_to` 不是 `config_center`
- 对账结果存在缺失或不一致
- 缺少 smoke 或 reconcile 日志引用
