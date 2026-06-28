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

## 测试环境查看标准

测试环境只保留三个入口：真实前端入口、业务原型入口和截图证据入口。截图证据不接后端，不嵌入 `apps/web-admin`，不另建截图服务。

三个入口都必须绑定 `0.0.0.0`，浏览器使用测试机局域网 IP 访问；测试机防火墙需放行 `9002`、`5174` 和 `9001`。

### 真实前端入口

```bash
pnpm -C apps/web-admin dev --host 0.0.0.0 --port 9002 --strictPort
```

本机人工联调如果暂时没有可用测试后端，可以显式启用开发 API mock：

```bash
WMS_WEB_ADMIN_DEV_MOCK=1 pnpm -C apps/web-admin dev --host 0.0.0.0 --port 9002 --strictPort
```

该模式只允许人工点通 9002 页面，不得用于真实截图证据、视觉基线或 Wave evidence。登录三元组为：货主 `PY_OWNER`，账号 `admin`，密码 `CorrectHorse1!`。

浏览器访问：

```text
http://<测试机 IP>:9002/
```

真实前端截图必须来自 `apps/web-admin`。接口尚未接入测试后端时，只能在截图脚本层 mock API 响应，并在截图证据页标明数据来源；不得把 `prototypes/` 截图归入真实前端分类。

### 业务原型入口

```bash
pnpm -C prototypes dev --host 0.0.0.0 --port 5174 --strictPort
```

浏览器访问：

```text
http://<测试机 IP>:5174/#<tab>
```

示例：`http://192.168.1.20:5174/#pc-m2-002` 查看 PC Web 收货原型。

### 截图证据入口

先生成真实前端或 Matrix E2E 截图，再用统一静态目录查看证据：

```bash
python3 -m http.server 9001 --bind 0.0.0.0 --directory artifacts/screenshot-portal
```

浏览器访问：

```text
http://<测试机 IP>:9001/
http://<测试机 IP>:9001/real-web/
http://<测试机 IP>:9001/prototype/
```

历史链接 `/screenshots/` 仅做兼容，指向原型 M2 截图分类；新证据链接必须使用 `real-web/` 或 `prototype/` 分类入口。

`artifacts/screenshot-portal/` 是测试产物目录，不入库；需要长期留存时使用 CI artifact 或 PR 摘录报告摘要。

目录固定分层：

```text
artifacts/screenshot-portal/
├── real-web/<模块>/
└── prototype/<模块>/
```

### 按模块查看截图

模块入口以 `manifest.toml` 的 `tab` 为准，截图命名固定为：

```text
screenshots/<tab>.initial.png
screenshots/<tab>.after-interaction.png
results/<tab>.json
```

例如 M2 PC Web 入库：

| 功能 | tab | 原型入口 | 截图 |
|---|---|---|---|
| 创建 ASN | `pc-m2-001` | `/#pc-m2-001` | `screenshots/pc-m2-001.*.png` |
| PC Web 收货 | `pc-m2-002` | `/#pc-m2-002` | `screenshots/pc-m2-002.*.png` |
| PC Web 验收 | `pc-m2-003` | `/#pc-m2-003` | `screenshots/pc-m2-003.*.png` |
| PC Web 上架 | `pc-m2-005` | `/#pc-m2-005` | `screenshots/pc-m2-005.*.png` |
| 收货异常处理 | `pc-m2-006` | `/#pc-m2-006` | `screenshots/pc-m2-006.*.png` |
| 收货单据打印 | `pc-m2-007` | `/#pc-m2-007` | `screenshots/pc-m2-007.*.png` |
| 收货进度看板 | `pc-m2-008` | `/#pc-m2-008` | `screenshots/pc-m2-008.*.png` |
| 打印模板设计 | `pc-m2-009` | `/#pc-m2-009` | `screenshots/pc-m2-009.*.png` |
| 上架策略配置 | `pc-m2-010` | `/#pc-m2-010` | `screenshots/pc-m2-010.*.png` |

模块级截图命令示例：

```bash
python3 scripts/governance/run_matrix_e2e_screenshots.py \
  --base-url http://127.0.0.1:5174 \
  --tab pc-m2-001 --tab pc-m2-002 --tab pc-m2-003 --tab pc-m2-005 \
  --tab pc-m2-006 --tab pc-m2-007 --tab pc-m2-008 --tab pc-m2-009 --tab pc-m2-010
python3 scripts/governance/check_matrix_e2e_report.py --allow-partial
```

### 变更约束

- 测试环境截图查看方案以本节为准。
- 如需改变入口、端口、目录或是否接入后端，必须先更新本文件并说明原因。
- 预期视觉变化仍需人工 review 后再走 `accept_baseline.py`，不能用截图证据入口替代 baseline 接受。

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
