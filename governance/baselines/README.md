# wms 治理债务与元数据（baselines/）

> 本目录存放两类治理资产：
> 1. **各治理脚本的 baseline 文件**（`<check_name>.json`）— 锁定历史债务，单调下降
> 2. **治理元数据快照**（`baseline-health.json` / `tier-runtime.json`）— 守护治理体系自身健康
>
> 详细规则：见 ADR-0003 §机制 3 + ADR-0006 §4.3 + governance.md §4.6
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

## 治理元数据文件（v0.4 加入）

除了各 check 的 baseline 文件外，本目录还存放以下治理元数据：

| 文件 | 用途 | 入库 | 维护脚本 |
|------|----|----|----|
| `baseline-health.json` | 每个 baseline 数量上限快照（防止 baseline 膨胀）| ✅ 是 | `check_baseline_health.py --update-snapshot` |
| `tier-runtime.json` | T1-T4 最近一次实际耗时与预算对比 | ✅ 是 | `just tier-timing` |

**入库理由**：
- 上限快照：保证 PR 间稳定的"应许总量"，强制单调下降
- 耗时快照：用于发现 Tier 耗时退化趋势（governance.md §1.2 速度即采纳率）

**修改约定**：
- `baseline-health.json` 仅在人工评审通过后用 `--update-snapshot` 写入
- `tier-runtime.json` 由 `just tier-timing` 命令在每次跑后覆盖写入
