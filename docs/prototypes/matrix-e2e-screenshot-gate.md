# Matrix E2E Screenshot Gate

> 目标：把原型视觉验证从“静态 baseline 对比”升级为“全量矩阵 E2E 截图证据”。

## 范围

本门禁覆盖 `governance/visual-baselines/manifest.toml` 中登记的全部 prototype tab。当前矩阵包含手工高保真页与 `prototype-matrix-r3.md` 展开的全量 story/end 页面。

## 门禁层级

| 层级 | 命令 | 作用 |
|---|---|---|
| T1 | `python3 scripts/governance/check_e2e_matrix_completeness.py` | 静态校验 E2E 策略覆盖全部 tab |
| T4 partial | `just matrix-e2e-smoke` | 跑前 20 个 tab，供本地快速验证执行器 |
| T4 full | `just matrix-e2e-full` | 跑全部 tab，生成 DOM / 交互 / 截图证据 |
| T4 verify | `just verify` | 合并前全量门禁，会调用 `matrix-e2e-full` |

## 证据产物

默认输出目录：`prototypes/.e2e-artifacts/`。

| 文件 / 目录 | 内容 | 是否入库 |
|---|---|---|
| `matrix-input.json` | Playwright 输入场景 | 否 |
| `screenshots/*.initial.png` | 初始状态截图 | 否 |
| `screenshots/*.after-interaction.png` | 交互后截图 | 否 |
| `results/<tab>.json` | 单 tab 检查结果 | 否 |
| `matrix-e2e-report.json` | 聚合报告 | 否，PR 可摘录摘要 |
| Playwright trace | 失败定位证据 | 否，CI artifact 保留 |

## 检查项

每个 tab 至少检查：

- 页面能打开，`header` / `main` 可见
- `console.error` 和 `pageerror` 为 0
- `expected_keywords` 达到策略命中率
- `document` 无横向溢出
- 主要文本 / 表格 / 按钮无非预期 overflow
- 控件无明显重叠
- 执行一次主按钮交互并截图
- 产出 initial 与 after-interaction 两张截图

特殊规则：

- `m4-manifest` / `pc-m4-005` / `pad-m4-005` 启用 `detect_vertical_cjk_table`，防止随货同行单表格中文逐字竖排。

## 策略文件

策略文件为 `governance/visual-baselines/e2e-scenarios.toml`。

`manifest.toml` 仍是 tab / viewport / baseline PNG 的唯一真相源；`e2e-scenarios.toml` 只声明 E2E 检查策略和特殊页加严规则。

## 失败处理

| 失败类型 | 处理 |
|---|---|
| 真实视觉回归 | 修原型，不更新 baseline |
| 预期视觉变化 | 先跑截图与 E2E，人工 review 后再走 `accept_baseline.py` |
| 关键词误配 | 修 `manifest.toml expected_keywords`，并说明证据 |
| 策略误报 | 修 `e2e-scenarios.toml`，不得直接降低全局阈值 |
| `m4-manifest` 中文竖排 | 修 PrintPreview / 表格布局，不能豁免 |
