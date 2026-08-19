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
/understand --full --language zh --no-auto-update
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
- `autoUpdate: false`；
- 保留代码、文档、SQL、迁移、配置、测试、治理和部署文件；
- 排除密钥、构建产物、截图/运行证据、本地数据库和缓存。

结构图只按用户明确指令更新：

1. 普通 commit、merge、rebase、cherry-pick、review、修复和分组提交不触发图谱更新；
2. 新鲜度检查失败时只报告图谱代表的历史基线，不自动修复或刷新；
3. 用户明确要求“更新图谱”“重建图谱”或明确调用图谱技能后，才按结构指纹选择最小充分动作；
4. 不允许把 `.ua/meta.json` 推进到尚未分析的相关文件之后；图谱输出可在输入提交之后单独提交。

业务域图从结构图派生，成本较低，但不由结构图 hook 自动重建。用户故事、术语、状态机、审批、合规规则或跨模块流程改变后，必须再调用 `understand-anything:understand-domain`。

## 新鲜度门禁

代理使用图谱作结论前：

1. 确认 `.ua/knowledge-graph.json`、`.ua/meta.json` 和 `.ua/fingerprints.json` 存在；
2. 运行 `python3 scripts/governance/check_knowledge_graph_freshness.py --json`；
3. `sourceCommitHash` 必须是当前 `HEAD` 或其祖先，且其后没有 `.ua/` 之外的已提交、暂存、未暂存或未跟踪输入变化；
4. `inputFingerprint` 必须与当前指纹输入一致；旧字段 `gitCommitHash` 不兼容读取；
5. 有未提交业务变更或检查结果为 stale 时，图谱只能说明已分析基线；不得自动更新，只有用户明确要求后才运行 `understand-diff` 或更新图谱；
6. Dashboard 已打开时，图谱更新后刷新浏览器页面。

`inputFingerprint` 是对 `fingerprints.json.files` 中按路径排序的
`[相对路径, 当前文件 SHA-256]` 数组做 JSON 紧凑编码后再计算 SHA-256；缺失输入使用
`<missing>` 标记。图谱提交只改变 `.ua/` 输出时不会制造输入变化。

不能因为 commit hash 相同就忽略未提交变更，也不能把业务域图当作实时状态。

## 按变更选择动作

以下选择规则只在用户明确要求更新图谱后生效。项目主人于 2026-08-03 持续批准当前版本
`.ua/.understandignore`；该文件和命令行额外 `--exclude` 范围不变时，代理按下表选择最小充分
动作，不再就全量或部分更新重复确认；边界模糊时选择较重一级。只有修改 ignore、额外 exclude
或分析目录范围时，才需要重新确认建图范围。

| 变更 | 明确要求更新后的动作 |
|---|---|
| 内容哈希变化，但受支持代码的函数、类型、import/export 结构均未变化 | 只更新元数据和指纹 |
| 同一目录或同一限界上下文内 `<= 10` 个结构/语义变化文件 | 部分更新相关节点、边和指纹 |
| `11-30` 个结构/语义变化文件，或新增/删除目录但未改变全局边界 | 部分更新，并重跑架构分层；分层显著变化时同步导览 |
| `> 30` 个结构/语义变化文件、影响超过图谱输入的 50%，或改变跨模块依赖、分层、入口 | 全量更新并复审结构图 |
| 图谱 schema、官方生成器、确定性关系生成逻辑或 canonical ID 规则变化 | 全量更新并复审结构图 |
| 用户故事、术语、业务状态、审批、合规规则或跨模块流程变化 | 按上述规则更新结构图后，再重建业务域图 |
| 审查当前分支或 PR | 在新鲜结构图上运行 `understand-diff` |
| 发布或重大架构调整前 | 全量更新、复审并打开 Dashboard 抽查 |

Markdown、SQL、TOML、YAML、OpenAPI、migration、runbook 和治理文件只要语义变化，均按结构
变化计数，不能用“非代码”降级为元数据更新。首次建图、缺少有效图谱/指纹、旧 meta schema
或 validator 无法证明完整性时，也直接全量重建。

## Git 与安全

- `.ua/` 是团队共享的生成资产；先提交源代码/文档等输入变化，再生成并单独提交最终图谱、元数据、指纹、配置和忽略规则。
- 不提交 `.ua/intermediate/`、`.ua/diff-overlay.json` 和 `.ua/.trash-*/`。
- 单个图谱 JSON 超过 10 MB 时再评估 Git LFS，未达到阈值不新增依赖。
- `.understandignore` 必须持续排除 `.env`、密钥、令牌、证书、生产数据导出、本地数据库和运行证据。
- 图谱摘要可能包含代码和文档语义，只在本地 Dashboard 中查看，不上传到未批准的外部服务。
