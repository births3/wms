# AI 项目图谱运行手册

## 目标

项目只维护一套可复用图谱数据，不为架构、业务、测试和部署分别造互不一致的系统。`Understand-Anything` 从同一仓库基线生成结构图和业务域图，再通过过滤、变更覆盖层和交互式 Dashboard 提供多角度视图。

图谱用于导航、影响分析和理解代码，不替代源代码、ADR、用户故事、RTM、质量矩阵或 GSP 合规结论。

## 固定路径

| 路径 | 用途 | 是否入库 |
|---|---|---|
| `.ua/knowledge-graph.json` | 文件、函数、类、配置、文档、数据表、端点、依赖、分层和导览 | 是 |
| `.ua/domain-graph.json` | 业务域、业务流程、流程步骤及其代码证据 | 是 |
| `.ua/meta.json` | 最近分析时间、源提交 `sourceCommitHash`、输入指纹 `inputFingerprint` 和文件数 | 是 |
| `.ua/fingerprints.json` | 增量更新的结构指纹基线 | 是 |
| `.ua/config.json` | 输出语言和自动更新开关 | 是 |
| `.ua/.understandignore` | 建图范围；排除密钥、构建物、运行证据和本地数据 | 是 |
| `.ua/intermediate/` | 扫描和组装临时文件 | 否 |
| `.ua/diff-overlay.json` | 当前本地变更的临时影响覆盖层 | 否 |

## 多角度视图

| 视角 | 入口 | 主要回答 |
|---|---|---|
| 架构与依赖 | `understand-anything:understand` / `understand-chat` | 模块边界、分层、入口、调用和依赖方向 |
| 业务域与流程 | `understand-anything:understand-domain` | M1-M11、横向能力、业务流程和代码证据 |
| 局部深挖 | `understand-anything:understand-explain` | 某文件、函数、页面或服务在系统中的作用 |
| 变更影响 | `understand-anything:understand-diff` | 当前 diff 影响哪些组件、调用链和风险面 |
| 测试与治理 | 结构图中的 `tested_by`、文档和配置边 | 实现是否有测试关联，治理规则影响哪些资产 |
| 数据、API 与部署 | 表、endpoint、config、service、pipeline 节点及边 | PostgreSQL、OpenAPI、运行配置和部署入口如何连接 |
| 交互浏览 | `understand-anything:understand-dashboard` | 搜索、分层、邻居、导览、业务域与 diff 覆盖层 |

这些视角共用图谱节点，不复制仓库事实。需要正式架构图或评审材料时，仍按 `wms-plantuml-docs` 将确认后的结论沉淀为 PlantUML 和说明文档。

## 图谱 schema 与官方刷新命令

结构图的顶层 `version` 固定为 `1.0.0`；`traceability` 使用 schema `1.0`，并固定
`canonicalIdScheme` 为 `<type>:<relative-path>[:symbol]`。每条边必须保留合法的
`sourceSpan.filePath`（相对仓库路径）和 `confidence`（`0..1`）；计数字段由 validator
按实际边重新计算，不能手工填入统计值。

官方更新入口是 Understand-Anything skill，而不是手工编辑 `.ua`：

```text
understand-anything:understand
/understand --full --language zh --auto-update
```

运行前先确认 `.ua/.understandignore`，完成 skill 的确认门禁；运行后必须单独提交生成的
`.ua/knowledge-graph.json`、`.ua/fingerprints.json`、`.ua/meta.json` 和相关配置，再执行：

```bash
python3 scripts/governance/check_knowledge_graph_traceability.py --json
python3 scripts/governance/check_knowledge_graph_freshness.py --json
```

业务域图不由该结构图命令自动刷新；流程、术语或状态变化后另行调用
`understand-anything:understand-domain`，并在同一套 schema/新鲜度门禁下验收。

## 建立与更新

首次或需要重新校准全仓语义时，调用 `understand-anything:understand`，要求：

- 全仓扫描；
- 中文输出；
- LLM 复审；
- `autoUpdate: true`；
- 保留代码、文档、SQL、迁移、配置、测试、治理和部署文件；
- 排除密钥、构建产物、截图/运行证据、本地数据库和缓存。

结构图通过插件 hook 动态更新：

1. commit、merge、rebase 或 cherry-pick 后检测 Git 基线变化；
2. 对代码、Markdown、SQL、TOML、YAML、迁移、部署和治理文件做忽略规则与结构指纹判断；
3. 语义不变时只更新元数据，局部结构变化时增量更新，跨模块或大范围变化时全量更新；
4. 不允许把 `.ua/meta.json` 推进到尚未分析的相关文件之后；图谱输出可在输入提交之后单独提交。

业务域图从结构图派生，成本较低，但不由结构图 hook 自动重建。用户故事、术语、状态机、审批、合规规则或跨模块流程改变后，必须再调用 `understand-anything:understand-domain`。

## 新鲜度门禁

代理使用图谱作结论前：

1. 确认 `.ua/knowledge-graph.json`、`.ua/meta.json` 和 `.ua/fingerprints.json` 存在；
2. 运行 `python3 scripts/governance/check_knowledge_graph_freshness.py --json`；
3. `sourceCommitHash` 必须是当前 `HEAD` 或其祖先，且其后没有 `.ua/` 之外的已提交、暂存、未暂存或未跟踪输入变化；
4. `inputFingerprint` 必须与当前指纹输入一致；旧字段 `gitCommitHash` 不兼容读取；
5. 有未提交业务变更或检查结果为 stale 时，图谱只能说明已分析基线；使用 `understand-diff` 叠加影响，或先更新图谱；
6. Dashboard 已打开时，图谱更新后刷新浏览器页面。

`inputFingerprint` 是对 `fingerprints.json.files` 中按路径排序的
`[相对路径, 当前文件 SHA-256]` 数组做 JSON 紧凑编码后再计算 SHA-256；缺失输入使用
`<missing>` 标记。图谱提交只改变 `.ua/` 输出时不会制造输入变化。

不能因为 commit hash 相同就忽略未提交变更，也不能把业务域图当作实时状态。

## 按变更选择动作

| 变更 | 最小动作 |
|---|---|
| 注释、格式或纯文本措辞且语义不变 | 由 hook 更新元数据 |
| 单模块代码、测试、SQL、配置或文档结构变化 | 增量更新结构图 |
| 跨模块依赖、分层、入口或大量文件变化 | 全量更新并复审结构图 |
| 用户故事、术语、业务状态或流程变化 | 更新结构图后重建业务域图 |
| 审查当前分支或 PR | 在新鲜结构图上运行 `understand-diff` |
| 发布、重大架构调整前 | 全量更新、复审并打开 Dashboard 抽查 |

## Git 与安全

- `.ua/` 是团队共享的生成资产；先提交源代码/文档等输入变化，再生成并单独提交最终图谱、元数据、指纹、配置和忽略规则。
- 不提交 `.ua/intermediate/`、`.ua/diff-overlay.json` 和 `.ua/.trash-*/`。
- 单个图谱 JSON 超过 10 MB 时再评估 Git LFS，未达到阈值不新增依赖。
- `.understandignore` 必须持续排除 `.env`、密钥、令牌、证书、生产数据导出、本地数据库和运行证据。
- 图谱摘要可能包含代码和文档语义，只在本地 Dashboard 中查看，不上传到未批准的外部服务。
