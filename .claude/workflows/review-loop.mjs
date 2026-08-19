export const meta = {
  name: 'review-loop',
  description: 'WMS 变更多维度并行审查→独立验证→复审循环（review → fix → review）',
  phases: [
    { title: 'Review', detail: '多维度并行只读审查 git diff' },
    { title: 'Verify', detail: '每条发现独立对抗验证' },
    { title: 'Re-review', detail: '复审修复后 diff（修复由主代理完成）' },
  ],
}

// ============ 输出 schema（标准 JSON Schema） ============

const findingSchema = {
  type: 'object',
  properties: {
    file: { type: 'string', description: '文件路径与行号，格式 文件:行号' },
    severity: { type: 'string', enum: ['P0', 'P1', 'P2', 'P3'], description: 'P0 阻断 / P1 高 / P2 中 / P3 低' },
    issue: { type: 'string', description: '问题描述' },
    evidence: { type: 'string', description: '证据：引用实际代码片段或命令输出' },
    fix: { type: 'string', description: '最小修复建议' },
    verify: { type: 'string', description: '可复跑的最小验证命令' },
  },
  required: ['file', 'severity', 'issue'],
  additionalProperties: false,
}

const reviewSchema = {
  type: 'object',
  properties: {
    summary: { type: 'string', description: '一句话总结审查结论；无问题时写"无剩余问题"' },
    findings: { type: 'array', items: findingSchema, description: '按严重度降序排列的发现列表' },
  },
  required: ['summary', 'findings'],
  additionalProperties: false,
}

const verdictSchema = {
  type: 'object',
  properties: {
    verdict: { type: 'string', enum: ['real', 'refuted', 'uncertain'], description: 'real=问题成立 / refuted=反驳成功 / uncertain=无法完全确认' },
    reason: { type: 'string', description: '核对过程与结论依据，1-3 句中文' },
  },
  required: ['verdict', 'reason'],
  additionalProperties: false,
}

// ============ 只读硬门禁（对齐 wms-worktree-subagent read-only-current-diff） ============

const HARD_GATES = [
  '硬门禁（违反即按严重问题报告）：',
  '- 只读：禁止修改/创建/删除任何文件；禁止 git add / commit / push / reset / checkout / clean。',
  '- 单条命令输出不超过 120 行；禁止把完整源文件或完整 diff 粘贴进输出。',
  '- 禁止对仓库根目录、backend/、apps/、tests/ 做无范围的 rg --files / find / rg / grep；检索必须带明确路径前缀。',
  '- 整个过程最多读取 12 个文件。',
].join('\n')

const REVIEW_PRELUDE = [
  '你是 WMS 仓库（医药/多货主/多仓企业级 WMS）的只读审查 agent（read-only-current-diff 模式）。',
  '审查对象：当前仓库根目录下的未提交改动。',
  '开始步骤（按序执行）：',
  '1. pwd 与 git rev-parse --show-toplevel 确认仓库根；git status --short --branch 查看分支与工作区状态。',
  '2. git diff --stat 与 git diff 查看未暂存改动；git diff --cached --stat 与 git diff --cached 查看已暂存改动；两者都是审查对象。',
  '3. diff 较大时用 git diff -- <路径> 分文件查看；未跟踪文件（??）用 Read 直接读取。',
  HARD_GATES,
  '输出契约（必须遵守）：',
  '- 用 schema 返回 summary 与 findings 数组；findings 按严重度降序（P0 阻断 > P1 高 > P2 中 > P3 低）。',
  '- 每条发现必须含：file（文件:行号）、severity、issue（问题描述）、evidence（证据，引用实际代码或命令输出）、fix（最小修复建议）、verify（可复跑的最小验证命令）。',
  '- 不允许泛泛而谈；每条发现都要能定位到具体文件:行号。',
  '- 确认无问题时 findings 返回空数组，summary 写"无剩余问题"。',
  '- 只输出你自己的维度发现，不要猜测其他维度是否存在问题。',
].join('\n')

// ============ full 模式：4 个审查维度 ============

const reviewDimensions = [
  {
    name: '正确性',
    label: 'review-correctness',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：正确性（bug/逻辑/副作用）。',
      '重点：',
      '- 逻辑错误、边界条件（空值、越界、除零、缺字段）、状态机流转错误。',
      '- 执行顺序与竞态、失败路径与错误处理、未捕获异常、副作用（写库/发消息/幂等）。',
      '- 业务语义：仅当 diff 涉及时核对医药 GSP 约束（批号、效期、温控、特殊药品、双人作业、审计留痕、货主级配置）。',
      '- 改动是否与上下文意图一致（例如重构是否等价、修复是否覆盖根因）。',
      '可用只读验证命令核对（如 git diff --check、just gov-t1），但不得运行会修改文件的命令（安装、生成器、git add 等），不得修改任何文件。',
    ].join('\n'),
  },
  {
    name: '项目规范',
    label: 'review-conventions',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：项目规范与代码风格。',
      '对照仓库 AGENTS.md、CLAUDE.md、docs/coding-standards.md、docs/layered-design.md、docs/frontend-coding-standards.md（允许读取这些文档核对）。',
      '重点：',
      '- 分层违规：后端 bin/runtime -> handler -> service -> domain/repository；前端 app shell -> page -> feature -> api-client、page -> @wms/ui business -> @wms/ui ui；domain 不得依赖 infra/数据库/HTTP/Redis/环境变量。',
      '- 禁止项：裸 fetch（无超时与错误处理）、any、unwrap、注释掉的代码、硬编码密钥/令牌（.env、私钥、真实令牌、生产数据导出不得入库）、审计表非 INSERT 操作。',
      '- 提交粒度：diff 是否混多个主题/scope（跨主题应拆成多个提交）；生成文件（OpenAPI/api-client 等）是否手写而非由生成器产生。',
      '- 未暂存与已暂存改动的归属是否合理（同主题应统一）。',
    ].join('\n'),
  },
  {
    name: '文档与契约一致性',
    label: 'review-docs',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：文档/契约一致性。',
      '重点：',
      '- 文档与代码不一致：本批 diff 改动的字段/接口/状态/页面是否与 docs/、README、需求追踪矩阵不一致。',
      '- 质量矩阵：新增/修改用户故事、页面、API、字段时是否同步 governance/quality-matrix.toml 与生成页 docs/governance/quality-matrix.md。',
      '- CLAUDE.md 与 AGENTS.md 同源：改动是否只改了一边（两边应同步）。',
      '- 技能与规范一致：.agents/skills/ 引用的规则与 AGENTS.md / CLAUDE.md 是否冲突或过时。',
      '- 文档索引：docs/adr/README.md 等索引是否缺新条目。',
    ].join('\n'),
  },
  {
    name: '测试与门禁覆盖',
    label: 'review-tests',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：测试与门禁覆盖。',
      '重点：',
      '- 本批改动的非平凡逻辑是否缺最小相关测试（TDD：先写失败测试再写代码；无测试要明确说明缺什么）。',
      '- 是否缺门禁接线：just gov-t1、git diff --check、just task-check；前端改动是否满足截图/页面行数/菜单证据门禁（apps/AGENTS.override.md，.tsx 页面 600 行警告、800 行门禁）。',
      '- 是否缺文档索引或质量矩阵条目。',
      '- 每个功能点是否都有可复跑的验证命令；缺则给出建议验证命令。',
    ].join('\n'),
  },
]

// ============ re-review 模式：3 个复审维度 ============

const reReviewDimensions = (previousFindings) => [
  {
    name: '修复正确性',
    label: 're-review-fix-correctness',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：修复正确性（re-review）。',
      previousFindings.length > 0
        ? '上轮已确认发现（主代理声称已修复）如下，请逐条核对修复是否真正解决原问题、修复本身是否引入新缺陷：\n' + JSON.stringify(previousFindings, null, 2)
        : '未提供上轮发现清单；请基于当前 diff 自行判断哪些改动属于修复、修复是否完整。',
      '重点：修复是否覆盖根因、边界是否补全、修复是否引入新 bug 或破坏既有行为。',
    ].join('\n'),
  },
  {
    name: '遗留问题',
    label: 're-review-leftover',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：遗留问题（re-review）。',
      previousFindings.length > 0
        ? '上轮已确认发现清单如下，请逐条核对当前 diff 中是否仍存在未修复或修复不完整的项：\n' + JSON.stringify(previousFindings, null, 2)
        : '未提供上轮发现清单；请基于当前 diff 找出仍遗留的审查问题。',
      '重点：未修复项、只修了一半的项、用回避手段掩盖的项。',
    ].join('\n'),
  },
  {
    name: '新引入问题',
    label: 're-review-new',
    prompt: [
      REVIEW_PRELUDE,
      '审查维度：新引入问题（re-review）。',
      '重点：本轮修复 diff 新引入的正确性/规范/测试/文档问题（与上轮发现无关的新问题），按正确性、项目规范、文档一致性、测试门禁四个角度全面扫一遍。',
    ].join('\n'),
  },
]

// ============ light 模式：综合审查维度（合并四个维度要点） ============

const lightDimension = {
  name: '综合审查',
  label: 'review-light',
  prompt: [
    REVIEW_PRELUDE,
    '审查维度：综合（light 模式，合并正确性/项目规范/文档一致性/测试门禁四个维度要点）。',
    '重点：',
    '- 正确性：bug/逻辑/边界/竞态/错误处理/副作用；医药 GSP 语义（批号效期温控特殊药双人作业审计留痕货主配置，仅当 diff 涉及时）。',
    '- 项目规范：分层违规（后端 bin/runtime→handler→service→domain/repository；前端 app shell→page→feature→api-client、page→@wms/ui business→ui；domain 不依赖 infra）、裸 fetch/any/unwrap/注释代码/硬编码密钥、审计表非 INSERT、提交粒度混 scope、生成文件手写。',
    '- 文档一致性：文档与代码不一致、质量矩阵同步、CLAUDE.md 与 AGENTS.md 同源、技能与规范冲突、ADR 索引缺条目。',
    '- 测试门禁：缺最小测试（TDD）、缺 gov-t1/diff --check/task-check 接线、前端截图/页面行数/菜单证据门禁、缺可复跑验证命令。',
  ].join('\n'),
}

// ============ Verify：单条发现对抗验证 ============

const verifyPrompt = (item, index) => [
  '你是对抗验证 agent：你的任务是【反驳】下面这条审查发现，而不是复述它。',
  '默认立场：审查发现可能误报；无法确凿证实的问题一律判定 refuted。',
  '审查对象：当前仓库根目录的未提交改动（只读）。',
  '开始步骤（按序执行）：',
  '1. pwd 与 git rev-parse --show-toplevel 确认仓库根；git status --short 确认工作区状态。',
  '2. git diff / git diff --cached 定位发现引用的改动；未跟踪文件用 Read 读取。',
  '3. 按 file:行号 读取相关代码，核对：证据是否真实存在？行号是否对得上？问题在当前代码下是否确实成立？',
  '4. 主动反驳：可能是误报（行为已被其他代码/上下文处理）、可能是语义理解错误、可能是文件/行号张冠李戴、可能是对未发布版本不适用。',
  '判定规则：',
  '- 证据确凿且问题成立 -> verdict=real。',
  '- 无法证实、证据不足、或存在合理解释 -> verdict=refuted。',
  '- 部分证据成立但无法完全确认（缺关键文件/上下文）-> verdict=uncertain，并在 reason 写明还缺什么证据。',
  HARD_GATES,
  '',
  `来源维度：${item.dimension}`,
  `发现（第 ${index + 1} 条）：`,
  `- file: ${item.finding.file}`,
  `- severity: ${item.finding.severity}`,
  `- issue: ${item.finding.issue}`,
  item.finding.evidence ? `- evidence: ${item.finding.evidence}` : '',
  item.finding.fix ? `- 建议修复: ${item.finding.fix}` : '',
  item.finding.verify ? `- 建议验证: ${item.finding.verify}` : '',
  '',
  'reason 用 1-3 句中文说明你实际核对过的证据与结论依据。',
].filter(Boolean).join('\n')

// ============ 主流程 ============

const main = async () => {
  const isReReview = typeof args !== 'undefined' && args && args.mode === 're-review'
  const previousFindings = typeof args !== 'undefined' && args && Array.isArray(args.previousFindings)
    ? args.previousFindings.slice(0, 20)
    : []
  // 审查强度分层：light（小改动，1 个综合审查 agent、无验证）/ standard（默认，4 维度 + 每发现 1 个验证者）/ heavy（大改动/敏感业务，4 维度 + 每发现 3 个验证者投票）
  const level = typeof args !== 'undefined' && args && ['light', 'standard', 'heavy'].includes(args.level) ? args.level : 'standard'
  log(`[Workflow] review-loop level=${level}${isReReview ? '（re-review）' : ''}`)

  // ---- 阶段 Review ----
  phase('Review')
  const dimensions = isReReview ? reReviewDimensions(previousFindings) : (level === 'light' ? [lightDimension] : reviewDimensions)
  log(`[Review] ${isReReview ? 're-review' : level + ' 模式'}：启动 ${dimensions.length} 个只读审查 agent（维度：${dimensions.map((d) => d.name).join('、')}）`)

  const raw = await parallel(
    dimensions.map((d) => () => agent(d.prompt, { label: d.label, phase: 'Review', schema: reviewSchema, effort: 'high' }))
  )
  const results = dimensions.map((d, i) => ({ dim: d, data: raw[i] })).filter((x) => x.data)
  if (results.length < dimensions.length) {
    log(`[Review] 警告：${dimensions.length - results.length} 个审查 agent 失败（无输出），已跳过其维度`)
  }

  const allFindings = []
  results.forEach(({ dim, data }) => {
    const list = Array.isArray(data.findings) ? data.findings : []
    list.forEach((finding) => allFindings.push({ dimension: dim.name, finding }))
    log(`[Review] 维度「${dim.name}」：${list.length} 条发现 | ${data.summary || ''}`)
  })
  log(`[Review] 合计 ${allFindings.length} 条发现`)

  // ---- 阶段 Verify ----
  phase('Verify')
  let confirmed = []
  let refuted = []
  let uncertain = []
  if (isReReview) {
    log('[Verify] re-review 模式跳过对抗验证（发现直接交主代理处理）')
  } else if (level === 'light') {
    log('[Verify] light 模式跳过对抗验证（发现直接交主代理处理）')
    uncertain = allFindings.map((item) => ({ ...item, verdict: 'pending', reason: 'light 模式未做对抗验证' }))
  } else if (allFindings.length === 0) {
    log('[Verify] 无发现，跳过对抗验证')
  } else {
    const verifierCount = level === 'heavy' ? 3 : 1
    log(`[Verify] 对 ${allFindings.length} 条发现逐条启动 ${verifierCount} 个独立对抗验证 agent（默认怀疑、不确定即 refuted${level === 'heavy' ? '；real/refuted 取多数票' : ''}）`)
    const verdicts = await parallel(
      allFindings.map((item, i) => () =>
        parallel(
          Array.from({ length: verifierCount }, (_, k) => () =>
            agent(verifyPrompt(item, i), { label: `verify-${i}-${k}`, phase: 'Verify', schema: verdictSchema, effort: 'medium' })
          )
        ).then((vs) => ({ i, vs }))
      )
    )
    const resolved = allFindings.map((item, i) => {
      const vs = (verdicts[i] && verdicts[i].vs) || []
      const votes = vs.filter((v) => v && ['real', 'refuted', 'uncertain'].indexOf(v.verdict) >= 0)
      const realVotes = votes.filter((v) => v.verdict === 'real').length
      const refutedVotes = votes.filter((v) => v.verdict === 'refuted').length
      let verdict
      if (votes.length === 0) {
        verdict = 'uncertain'
      } else if (level === 'heavy') {
        verdict = realVotes >= 2 ? 'real' : (refutedVotes >= 2 ? 'refuted' : 'uncertain')
      } else {
        verdict = votes[0].verdict
      }
      const reason = votes.map((v) => v.reason).filter(Boolean).join(' | ')
      return { ...item, verdict, reason: reason || '（验证 agent 失败，按不确定处理）' }
    })
    confirmed = resolved.filter((x) => x.verdict === 'real')
    refuted = resolved.filter((x) => x.verdict === 'refuted')
    uncertain = resolved.filter((x) => x.verdict === 'uncertain')
    log(`[Verify] 验证完成：real=${confirmed.length} refuted=${refuted.length} uncertain=${uncertain.length}`)
    if (refuted.length > 0) {
      log(`[Verify] 被反驳（误报）示例：${refuted.slice(0, 3).map((x) => `${x.finding.file} ${x.finding.issue}`).join('；')}`)
    }
  }

  // ---- 阶段 Re-review ----
  phase('Re-review')
  log('[Re-review] 主代理修复确认发现后，以 { mode: "re-review" } 重新运行本脚本做复审（仅 Review，3 个维度：修复正确性/遗留/新引入）')

  const pack = (items) => items.map((x) => ({
    file: x.finding.file,
    severity: x.finding.severity,
    issue: x.finding.issue,
    evidence: x.finding.evidence || '',
    fix: x.finding.fix || '',
    verify: x.finding.verify || '',
    dimension: x.dimension,
    verdict: x.verdict || 'pending',
    reason: x.reason || '',
  }))

  const summary = isReReview
    ? `复审完成：${allFindings.length} 条剩余/新发现（维度：修复正确性/遗留/新引入），请主代理逐条处理`
    : level === 'light'
      ? `审查完成（light，未做对抗验证）：${allFindings.length} 条发现，请主代理逐条判断后修复`
      : `审查完成：${allFindings.length} 条发现 -> ${confirmed.length} 条确认、${refuted.length} 条反驳、${uncertain.length} 条不确定；修复确认项后以 mode=re-review 重跑复审`

  return {
    phase: isReReview ? 'Review' : 'Verify',
    mode: isReReview ? 're-review' : level,
    findings: isReReview ? pack(allFindings) : (level === 'light' ? pack(uncertain) : pack(confirmed)),
    refuted: pack(refuted),
    uncertain: pack(uncertain),
    summary,
    next: '主代理按 findings（severity 降序）修复后，以 args { mode: "re-review", previousFindings: <本次确认发现> } 重新运行本脚本复审',
  }
}

const result = await main()
return result
