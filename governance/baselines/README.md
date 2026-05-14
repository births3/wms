# wms baseline（治理债务锁定）

> 本目录存放各治理脚本的 baseline 文件（`<check_name>.json`）。
> 详细规则：见 ADR-0003 §机制 3 + ADR-0006 §4.3
>
> 第 0 周：暂无业务代码，baseline 为空。
> 各 Wave 启动时，对应治理脚本上线后会在此生成 baseline 文件。
>
> 关键约束：
> - baseline 文件入库（治理资产）
> - 已修复的违规自动从 baseline 移除（auto-shrink）
> - baseline 数量必须**单调下降**，季度评审
> - 关键不变量（审计 append-only / domain 不依赖 infra / 密钥不入库）**禁止 baseline**

## 文件命名

`<check_name>.json`，与 `scripts/governance/<check_name>.py` 一一对应。

## 文件格式（示例）

```json
{
  "check": "check_handler_test_coverage",
  "version": 1,
  "generated_at": "2026-05-15T00:00:00Z",
  "ignored": [
    {
      "id": "backend/crates/api/src/handlers/inbound.rs::receive_handler",
      "reason": "MVP 期间临时跳过；TODO #123",
      "added_at": "2026-05-15",
      "expires_at": "2026-08-15"
    }
  ]
}
```

## 当前 baseline 列表

（无）
