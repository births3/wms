# Wave 5 Hardware Evidence Runbook

> 用途：关闭 Wave 6 W6.F 中从 Wave 5 后移的 M-PK 真实硬件 evidence gate。覆盖电子秤、蓝牙打印机和面单打印设备的 dev/staging 联调证据。

## 目标

证明 M-PK 包装站在真实设备边界下可以完成称重、蓝牙标签打印、面单打印和审计记录：

- 真实包装工位已登记。
- 电子秤能返回至少 1 条称重读数。
- 蓝牙打印机能打印至少 1 张标签。
- 面单打印设备能打印至少 1 张面单。
- 称重、打印和面单动作均能查询到对应 `audit_event`。

## 前置条件

- 环境为 `dev` 或 `staging`，不得使用 `local` / `prod`。
- M-PK API、PostgreSQL migration、H2 audit_event 已部署到同一环境。
- 电子秤、蓝牙打印机和面单打印设备已接入测试工位。
- 设备校准记录已归档。
- 日志、截图、CI 记录或设备平台记录已归档到证据库，不把二进制附件直接提交到仓库。

## 必需证据

1. 工位和设备证据：
   - 包装工位编号。
   - 电子秤设备引用。
   - 蓝牙打印机设备引用。
   - 面单打印设备引用。
   - 设备校准记录引用。
2. 运行日志：
   - 电子秤读数日志。
   - 蓝牙标签打印日志。
   - 面单打印日志。
3. 审计证据：
   - `audit_event` 查询结果引用。
   - 审计事件能关联包装工位、包装作业或面单资源。
4. 人工确认：
   - 设备已真实连接。
   - 打印产物已人工核对。
   - 审计事件已验证。

## Evidence JSON

真实证据写入 `docs/retros/wave-5-hardware-evidence.json`：

```json
{
  "environment": "staging",
  "station_code": "PK-STAGING-01",
  "scale_device_ref": "asset://wms-staging/hardware/scale-01",
  "bluetooth_printer_ref": "asset://wms-staging/hardware/bluetooth-printer-01",
  "waybill_printer_ref": "asset://wms-staging/hardware/waybill-printer-01",
  "calibration_record_ref": "s3://wms-staging-evidence/wave5/hardware/calibration-YYYYMMDD.pdf",
  "scale_reading_log_ref": "ci/staging/wave5-hardware-scale/123",
  "bluetooth_print_log_ref": "ci/staging/wave5-hardware-bluetooth-print/123",
  "waybill_print_log_ref": "ci/staging/wave5-hardware-waybill-print/123",
  "audit_event_query_ref": "ci/staging/wave5-hardware-audit/123",
  "scale_readings_recorded": 1,
  "bluetooth_labels_printed": 1,
  "waybills_printed": 1,
  "hardware_connected": true,
  "print_artifacts_reviewed": true,
  "audit_event_verified": true
}
```

字段含义：

| 字段 | 要求 |
|------|------|
| `environment` | 只能是 `dev` 或 `staging` |
| `station_code` | 真实包装工位编号 |
| `*_ref` | 指向真实设备、校准记录、日志或审计查询的归档引用 |
| `scale_readings_recorded` | 至少 1 |
| `bluetooth_labels_printed` | 至少 1 |
| `waybills_printed` | 至少 1 |
| `hardware_connected` | 真实设备连接验证后为 `true` |
| `print_artifacts_reviewed` | 打印产物人工核对后为 `true` |
| `audit_event_verified` | 查询到对应审计事件后为 `true` |

## 验证命令

```bash
just wave-5-hardware-evidence-record \
  --environment staging \
  --station-code '<真实包装工位编号>' \
  --scale-device-ref '<电子秤设备引用>' \
  --bluetooth-printer-ref '<蓝牙打印机设备引用>' \
  --waybill-printer-ref '<面单打印设备引用>' \
  --calibration-record-ref '<设备校准记录引用>' \
  --scale-reading-log-ref '<电子秤读数日志引用>' \
  --bluetooth-print-log-ref '<蓝牙标签打印日志引用>' \
  --waybill-print-log-ref '<面单打印日志引用>' \
  --audit-event-query-ref '<audit_event 查询证据>' \
  --scale-readings-recorded 1 \
  --bluetooth-labels-printed 1 \
  --waybills-printed 1 \
  --hardware-connected \
  --print-artifacts-reviewed \
  --audit-event-verified

just wave-5-hardware-evidence-validate
```

该命令只验证 evidence JSON 的完整性和边界，不负责连接设备。设备联调必须先在真实 dev/staging 环境完成。

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- 设备引用只指向本地模拟器、单元测试、截图占位或人工描述。
- 计数为 0。
- 未人工核对打印产物。
- 查不到对应 `audit_event`。

## 完成判定

W6.F 的完成判定以 `just wave-5-hardware-evidence-validate` 通过为准。没有真实设备和真实 dev/staging 日志时，只能完成 runbook / validator，不能关闭 gate。
