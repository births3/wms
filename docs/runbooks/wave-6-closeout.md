# Wave 6 Closeout Runbook

> 用途：集中关闭 Wave 6 预发布证据 gate。Wave 6 完成必须以 `just wave-6-complete-check` 通过为准；本 runbook 只定义执行顺序和证据记录命令，不能替代真实 dev/staging、真设备、真实外部系统或灰度发布证据。

## 完成口径

Wave 6 完成需要以下全部条件成立：

1. `just wave-6-status` 无阻塞缺口。
2. `just wave-6-complete-check` 退出 0。
3. `docs/retros/wave-6-retro.md` 已写入本轮真实 evidence 结果和剩余风险。
4. `just gov-t1`、`just task-check`、`git diff --check` 通过。

禁止用 `localhost`、`127.0.0.1`、`0.0.0.0`、`prod`、`production`、`mock`、`fake`、`stub`、`example` 证据替代真实 dev/staging 或外部系统证据。

所有 evidence 引用还必须包含当前 `environment` 标记（`dev` 或 `staging`）；缺少环境标记的证据库路径、CI 记录、Vault 引用或审批票据不能关闭 gate。

## 当前 Gate

| Gate | Evidence 文件 | 记录入口 | 验证入口 |
|------|---------------|----------|----------|
| W6.A | `docs/retros/wave-1-h2-runtime-evidence.json` | `just wave-1-h2-runtime-evidence` | `just wave-1-runtime-evidence-validate` |
| W6.B | `docs/retros/wave-1-runtime-evidence.json` | `just wave-1-rollback-runtime-evidence-k8s` 或 `just wave-1-rollback-runtime-evidence-compose` | `just wave-1-runtime-evidence-validate` |
| W6.C | `docs/retros/wave-2-runtime-evidence.json` | `just wave-2-runtime-evidence-record` | `just wave-2-runtime-evidence-validate` |
| W6.D | `docs/retros/wave-3-pda-runtime-evidence.json` | `just wave-3-pda-runtime-evidence-record` | `just wave-3-pda-runtime-evidence-validate` |
| W6.E | `docs/retros/wave-4-external-dependencies.json` | `just wave-4-external-dependencies-record` | `just wave-4-external-dependencies-validate` |
| W6.F | `docs/retros/wave-5-hardware-evidence.json` | `just wave-5-hardware-evidence-record` | `just wave-5-hardware-evidence-validate` |
| W6.G | `docs/retros/wave-5-tms-evidence.json` | `just wave-5-tms-evidence-record` | `just wave-5-tms-evidence-validate` |
| W6.H | `docs/retros/wave-6-deploy-evidence.json` | `just wave-6-deploy-evidence-record` | `just wave-6-deploy-evidence-validate` |

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

按实际部署形态二选一，不要同时用本地 stub 兜底：

```bash
just wave-1-runtime-prereq-rollback-k8s
just wave-1-rollback-runtime-readiness-k8s
just wave-1-rollback-runtime-evidence-k8s
just wave-1-runtime-evidence-validate
```

或：

```bash
just wave-1-runtime-prereq-rollback-compose
just wave-1-rollback-runtime-readiness-compose
just wave-1-rollback-runtime-evidence-compose
just wave-1-runtime-evidence-validate
```

### 3. Wave 2 config-center Feature Flag evidence

按 [Wave 2 Pre-release Runtime Evidence Runbook](wave-2-runtime-evidence.md) 完成真实 smoke、reconcile 和旧文件归档后，运行：

```bash
just wave-2-runtime-evidence-record <真实参数>
just wave-2-runtime-evidence-validate
```

### 4. Wave 3 PDA / L7 evidence

真 PDA、实体扫码键、M2/M3 dev/staging 日志、离线 replay、幂等 replay、审计查询和易用性走查全部到位后，运行：

```bash
just wave-3-pda-runtime-evidence-record <真实参数>
just wave-3-pda-runtime-evidence-validate
```

### 5. Wave 4 M-TC 码上放心 evidence

正式接口文档、鉴权、错误码、频率限制、成功上报、失败重试和 audit_event 查询证据到位后，运行：

```bash
just wave-4-external-dependencies-record <真实参数>
just wave-4-external-dependencies-validate
```

### 6. Wave 5 M-PK hardware evidence

电子秤、蓝牙打印机、面单打印设备、校准记录、打印产物核对和 audit_event 查询证据到位后，运行：

```bash
just wave-5-hardware-evidence-record <真实参数>
just wave-5-hardware-evidence-validate
```

### 7. Wave 5 M10 TMS+ evidence

真实 TMS dev/staging 推送、回调、失败重试、Vault 凭证引用和 audit_event 查询证据到位后，运行：

```bash
just wave-5-tms-evidence-record <真实参数>
just wave-5-tms-evidence-validate
```

### 8. Wave 6 gray release evidence

staging 灰度发布、smoke gate、dashboard、回滚演练、双人审批和 audit_event 查询证据到位后，运行：

```bash
just wave-6-deploy-evidence-record <真实参数>
just wave-6-deploy-evidence-validate
```

## 最终关闭

全部 evidence validator 通过后，先做一次状态与治理检查：

```bash
just wave-6-status
just wave-6-evidence-check
just gov-t1
just task-check
git diff --check
```

然后写 `docs/retros/wave-6-retro.md`，至少包含：

- 8 个 gate 的 evidence 文件路径和验证命令结果。
- 真实环境、设备、外部系统和灰度发布的剩余风险。
- 明确声明没有使用 local/mock/fake/stub/example/prod 证据。

写完 retro 后再次运行：

```bash
just wave-6-status
just wave-6-complete-check
just gov-t1
just task-check
git diff --check
```
