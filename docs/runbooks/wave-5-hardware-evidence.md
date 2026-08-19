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

- 环境为 `dev` 或 `staging`，不得使用 `local` / `prod` / `production`。
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
以下 JSON 仅为字段结构示例，不得复制为真实 evidence；真实 evidence 必须由 record 命令生成。

```json
{
  "environment": "staging",
  "station_code": "PK-STAGING-01",
  "scale_device_ref": "asset://wms-staging/hardware/scale-01",
  "bluetooth_printer_ref": "asset://wms-staging/hardware/bluetooth-printer-01",
  "waybill_printer_ref": "asset://wms-staging/hardware/waybill-printer-01",
  "calibration_record_ref": "s3://wms-staging-evidence/wave5/hardware/calibration-20260603.pdf",
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

所有设备、校准、打印、称重和审计证据引用必须包含当前 `environment` 标记（`dev` 或 `staging`），并且不能指向 local / prod / production / mock / fake / stub / example。

先导出现场材料变量模板。该命令只输出变量清单和 check-only 命令，不连接硬件，不写 `docs/retros/wave-5-hardware-evidence.json`，不能关闭 W6.F gate：

```bash
just wave-5-hardware-materials --export-template
```

```bash
just wave-5-hardware-materials --from-env --json
just wave-5-hardware-readiness --from-env --json
just wave-5-hardware-evidence-record --from-env --check-only --json
just wave-5-hardware-evidence-record --from-env --json
just wave-5-hardware-evidence-validate
```

`just wave-5-hardware-materials --from-env --json`、`just wave-5-hardware-readiness --from-env --json` 和 `just wave-5-hardware-evidence-record --from-env --check-only --json` 只校验字段、证据引用和 dev/staging 边界；不连接真实硬件，不写 `docs/retros/wave-5-hardware-evidence.json`，不能关闭 W6.F gate。设备联调必须先在真实 dev/staging 环境完成；`record --from-env --json` 写入真实 evidence 后，`validate` 只验证 evidence JSON 的完整性和边界。

### 现场执行包完成标准

W6.F 现场执行包完成，不等于真实硬件 evidence 完成。现场执行包完成标准是：

1. `just wave-5-hardware-materials --export-template` 能输出完整 `WAVE_5_*` 变量清单和后续命令。
2. 现场设备负责人只需要填入真实设备资产、校准记录、称重日志、打印日志和 `audit_event` 查询引用，不需要拼长参数。
3. `just wave-5-hardware-materials --from-env --json` 和 `just wave-5-hardware-readiness --from-env --json` 能定位缺失变量及负责人。
4. `just wave-5-hardware-evidence-record --from-env --check-only --json` 通过后，现场同事执行一条正式命令 `just wave-5-hardware-evidence-record --from-env --json` 生成 `docs/retros/wave-5-hardware-evidence.json`。
5. 正式 record 后必须立即执行 `just wave-5-hardware-evidence-validate`；只有 validator 通过才关闭 W6.F。

## 拒绝边界

- `environment` 是 `local` / `prod` / `production`。
- 任一证据引用包含 `localhost`、`127.0.0.1`、`0.0.0.0`、`local`、`prod`、`production`、`mock`、`fake`、`stub`、`example`。
- 任一证据引用保留模板占位，如 `YYYY`、`<...>`、`TODO`、`TBD`、`待填`、`待确认`。
- 设备引用只指向本地模拟器、单元测试、截图占位或人工描述。
- 计数为 0。
- 未人工核对打印产物。
- 查不到对应 `audit_event`。

## 完成判定

W6.F 的完成判定以 `just wave-5-hardware-evidence-validate` 通过为准。没有真实设备和真实 dev/staging 日志时，只能完成 runbook / validator，不能关闭 gate。
