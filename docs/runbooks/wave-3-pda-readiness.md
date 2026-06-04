# Wave 3 PDA Readiness Runbook

> 适用范围：SPIKE-005 RN 扫枪 + 离线队列启动前置。当前决策为先落 readiness/runbook，不引入 RN 依赖，不创建 `apps/pda-mobile` 生产 app。

## 目标

在真 PDA 和稳定 dev/staging 可用前，先冻结启动 SPIKE-005 所需的输入边界，避免用本地模拟或浏览器扫码结果替代真机结论。

## 范围

- 记录设备清单与借测状态
- 准备扫码样本与 M2/M3 API 验证路径
- 明确 dev/staging、账号、API Key 与日志引用要求
- 定义 SPIKE-005 启动/停止条件

不包含：

- 不引入 RN 依赖
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
| Honeywell EDA52 | 待借测 | 待确认 | 实体扫码键 / Intent | 未到位 | SPIKE-005 候选 |
| Urovo i6310 | 待借测 | 待确认 | 实体扫码键 / KeyEvent | 未到位 | SPIKE-005 候选 |
| Zebra TC52 | 待借测 | 待确认 | DataWedge / Intent | 未到位 | SPIKE-005 候选 |

## 扫码样本

| 类型 | 数量 | 用途 |
|---|---:|---|
| GS1 追溯码 | 待确认 | M2 收货 / 验收扫描 |
| Code128 批号 / 箱码 | 待确认 | M2 上架 / M3 库存定位 |
| 二维码任务号 | 待确认 | PDA 任务流转 |

样本总量沿用 SPIKE-005 的 50 个不同条码方向；分类拆分等 SPIKE-005 启动时按设备和流程确认。样本必须来自脱敏测试数据，不使用生产真实药品追溯码。

## 执行步骤

1. 确认设备清单中至少一台 PDA 到位，并记录型号、系统版本和扫码输入方式。
2. 确认 dev/staging 已部署包含 Wave 3 M2/M3 handler 的 `wms-api`。
3. 准备具备 `m2.write` / `m3.write` 权限的测试账号。
4. 准备 M2 收货单、M2 验收、M2 上架、M3 状态变更的测试数据。
5. 启动 SPIKE-005，两天时间盒重新计时。
6. 采集扫码延迟、离线队列重放、幂等 replay、审计落库证据。
7. 输出 SPIKE-005 结论，并决定是否创建/更新 ADR-0027 PDA 离线模型。

## 启动条件

满足以下全部条件才启动 SPIKE-005：

- 至少一台真 PDA 到位
- dev/staging 服务可访问
- M2/M3 测试数据可重建
- 可以保存测试日志引用

## 拒绝边界

- 使用浏览器、模拟器或手机摄像头代替 PDA 实体扫码键
- 使用 local / mock / fake / example / prod 作为证据
- 未记录 `Idempotency-Key` 与 H2 `audit_event` 的链路证据
- 在未完成 SPIKE-005 前引入 RN 生产依赖

## Evidence JSON

真实证据写入 `docs/retros/wave-3-pda-runtime-evidence.json`。clarifications #67 已确认当前不发明本地 L7 阈值，因此本 evidence 只验证真实 PDA、真实 dev/staging、日志引用、审计链路和人工易用性走查是否存在。

```json
{
  "environment": "staging",
  "pda_model": "Honeywell EDA52",
  "android_version": "Android 11",
  "scan_input_method": "physical-scan-key-intent",
  "pda_device_ref": "asset://wms-staging/pda/honeywell-eda52-01",
  "spike005_result_ref": "docs/spikes/spike-005-rn-scanner.md#runtime-YYYYMMDD",
  "m2_scan_log_ref": "ci/staging/wave3-pda-m2-scan/123",
  "m3_scan_log_ref": "ci/staging/wave3-pda-m3-scan/123",
  "offline_replay_log_ref": "ci/staging/wave3-pda-offline-replay/123",
  "idempotency_replay_log_ref": "ci/staging/wave3-pda-idempotency-replay/123",
  "audit_event_query_ref": "ci/staging/wave3-pda-audit/123",
  "l7_run_ref": "ci/staging/wave3-pda-l7/123",
  "usability_review_ref": "s3://wms-staging-evidence/wave3/pda/usability-review-YYYYMMDD.md",
  "barcode_samples_scanned": 1,
  "m2_operations_exercised": 1,
  "m3_operations_exercised": 1,
  "offline_replays_exercised": 1,
  "idempotency_replays_exercised": 1,
  "real_pda_used": true,
  "physical_scan_key_verified": true,
  "dev_or_staging_service_verified": true,
  "audit_event_verified": true,
  "l7_review_completed": true,
  "usability_review_completed": true
}
```

执行验证：

```bash
just wave-3-pda-runtime-evidence-validate
```

## 输出

- SPIKE-005 §7 追加本轮决策与实测结果
- 如 accept：产出 ADR-0027 PDA 离线模型
- 如 reject：记录替代方案，例如 native Android + RN bridge
- 如 defer：更新 ROADMAP backlog 与下一次启动条件
