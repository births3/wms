# Governance Tier 1 存量问题收口清单（2026-08-31）

## 1. 基线与目标

本清单用于收口 WMS 仓库现有的 Governance Tier 1 治理债务。

- 基线分支：`main`
- 基线提交：`636300fa9c1edd4f76662a5c2fec41f23cf2e19d`
- 取证运行：GitHub Actions Run `33408209404`
- 当前结果：`47/59` 项通过
- 失败检查脚本：12 个
- 归并后的独立根因：13 项
- 目标结果：Tier 1 达到 `59/59`，且不通过跳过检查、降低检查级别或伪造证据实现“变绿”

`check_doc_links.py` 和 `check_dead_code.py` 的失败分别由 `markdown`、`ruff` 缺失引起，因此在脚本统计中是两个失败项，在根因清单中归入对应的环境依赖项。

## 2. 修复原则

1. 不关闭、不绕过、不改成非阻塞模式来规避 Tier 1。
2. CI 所需工具必须锁定版本或使用可复现的安装方式。
3. 质量矩阵、E2E、截图和原型证据必须对应真实可执行命令与真实文件。
4. 同一事实只保留一个权威来源，索引、故事、组件注册表与原型文件保持同步。
5. 每完成一个根因项，先运行对应专项检查，再运行完整 Tier 1。

## 3. 13 项根因清单

| ID | 根因 | 受影响检查 | 当前证据 | 验收标准 |
|---|---|---|---|---|
| T1-01 | Runner 缺少 `gitleaks` | `validate_environment.py` | 环境校验报告命令不存在 | CI 与本地治理环境能执行锁定版本的 `gitleaks` |
| T1-02 | Runner 缺少 `ruff` | `validate_environment.py`、`check_dead_code.py` | 环境校验失败，死代码检查无法启动 | `ruff` 可执行且死代码检查真实运行通过 |
| T1-03 | Python 缺少 `pathspec` | `validate_environment.py` | Python 包导入失败 | 治理环境依赖安装后可稳定导入 `pathspec` |
| T1-04 | Python 缺少 `markdown` | `validate_environment.py`、`check_doc_links.py` | 文档链接检查无法启动 | 可导入 `markdown`，文档链接检查真实运行通过 |
| T1-05 | TypeScript Compiler API 不可用 | `check_admin_page_design_contract.py` | 检查器无法加载 TypeScript 编译器 API | 前端依赖安装完整，检查器能解析管理端页面并通过 |
| T1-06 | 质量矩阵存在 232 个证据问题 | `check_quality_matrix.py` | E2E 命令、配置、用例、截图或引用不一致 | 每条矩阵记录均指向真实存在且可执行、可复验的证据 |
| T1-07 | Scope Gap 存在 17 个硬缺口 | `check_scope_gap_discovery.py` | 菜单页面缺少真实 E2E 截图证据 | 17 个硬缺口全部由真实 Playwright 流程和截图闭环 |
| T1-08 | 缺少 `docs/prototypes/index.toml` | `check_prototype_index_consistency.py` | 原型索引文件不存在 | 恢复或生成唯一权威索引，索引内容与仓库文件一致 |
| T1-09 | 原型索引与用户故事不同步 | `check_prototype_story_sync.py` | 因索引缺失或映射不完整而失败 | 所有受管原型均能追溯到有效用户故事，且无悬空记录 |
| T1-10 | 23 个组件目录未登记 | `check_component_registry_consistency.py` | 组件注册表与源码目录不一致 | 23 个目录完成登记、合并或删除，注册表与源码一一对应 |
| T1-11 | 原型保真度检查缺少 5 个文件 | `check_prototype_fidelity.py` | 检查器引用的原型实现不存在 | 恢复真实实现或修正权威清单，保真度检查通过 |
| T1-12 | 原型导航检查缺少 2 个文件 | `check_prototype_navigation.py` | `App.tsx`、`Tabs.tsx` 等导航基线不完整 | 原型入口和导航结构真实存在，所有目标可达 |
| T1-13 | 原型基线不完整 | `check_baseline_completeness.py` | 缺少 `prototypes/src/Tabs.tsx` 等基线文件 | 基线所要求的文件、导出与结构全部完整且检查通过 |

> T1-08 至 T1-13 可能引用同一批原型文件，但它们分别代表索引、故事映射、组件登记、保真度、导航和基线六类不同治理契约，必须分别验收。

## 4. 实施顺序

### 阶段 A：治理环境可复现（T1-01～T1-05）

- 建立统一的 Governance Python 依赖清单并锁定版本。
- 在 CI 中安装 `gitleaks`、`ruff`、`pathspec`、`markdown`。
- 完整安装前端依赖，确保 TypeScript Compiler API 可被治理脚本加载。
- 验证 `validate_environment.py`、`check_doc_links.py`、`check_dead_code.py`、`check_admin_page_design_contract.py`。

### 阶段 B：原型与注册表契约修复（T1-08～T1-13）

- 先确认原型目录中的真实文件和废弃文件，不盲目补空壳。
- 建立 `docs/prototypes/index.toml` 的唯一权威来源。
- 同步用户故事映射、组件注册表、导航入口和原型基线。
- 对重复检查引用的文件一次修复、多项复验。

### 阶段 C：真实质量证据闭环（T1-06～T1-07）

- 逐条核对质量矩阵中的命令、配置、测试文件、截图和页面引用。
- 对 17 个硬缺口补充真实 Playwright 流程及可复验截图。
- 禁止仅创建占位文件、空截图或不可执行命令来满足路径检查。

### 阶段 D：完整回归

依次执行：

```bash
python3 scripts/governance/validate_environment.py
python3 scripts/governance/check_doc_links.py
python3 scripts/governance/check_dead_code.py
python3 scripts/governance/check_admin_page_design_contract.py
python3 scripts/governance/check_quality_matrix.py
python3 scripts/governance/check_scope_gap_discovery.py
python3 scripts/governance/check_prototype_index_consistency.py
python3 scripts/governance/check_prototype_story_sync.py
python3 scripts/governance/check_component_registry_consistency.py
python3 scripts/governance/check_prototype_fidelity.py
python3 scripts/governance/check_prototype_navigation.py
python3 scripts/governance/check_baseline_completeness.py
python3 scripts/governance/governance_checks.py --tier T1
```

## 5. PR 完成条件

- [ ] T1-01：`gitleaks` 环境完成
- [ ] T1-02：`ruff` 与死代码检查完成
- [ ] T1-03：`pathspec` 依赖完成
- [ ] T1-04：`markdown` 与文档链接检查完成
- [ ] T1-05：TypeScript Compiler API 环境完成
- [ ] T1-06：232 个质量矩阵证据问题归零
- [ ] T1-07：17 个 Scope Gap 硬缺口归零
- [ ] T1-08：原型索引恢复并一致
- [ ] T1-09：原型与用户故事同步
- [ ] T1-10：23 个组件目录完成治理
- [ ] T1-11：原型保真度缺失文件完成治理
- [ ] T1-12：原型导航缺失文件完成治理
- [ ] T1-13：原型基线完整
- [ ] 完整 Tier 1 输出为 `59/59 ok`
- [ ] 未新增跳过规则、豁免规则、空壳证据或伪造截图
- [ ] 最终提交满足仓库提交约定，工作区无临时修复载荷

## 6. 非本 PR 阻塞范围

本次运行中出现但未导致 Tier 1 失败的角色白名单、可观测性 KPI、错误码、页面体积和多端一致性提示，暂作为后续优化项记录；若修复过程中升级为 Tier 1 阻塞，再纳入本 PR。