# Wave 0.5 Retrospective

- 日期：2026-05-23
- 范围：高保真原型 + 组件库抽离 + 技术 Spike 计划（commit 8704578 → 62bf9eb）
- 周期：2 天（2026-05-22 ~ 2026-05-23）/ 36 个 commit
- 关联 ADR：0021 高保真原型策略 / 0022 原型组件规范 / 0023 业务报表方案 / 0028 组件库抽离

---

## 1. 计划 vs 实际

| 维度 | 计划（ROADMAP v0 原稿） | 实际交付 | 差异 |
|------|-----------|---------|------|
| 周期 | 2 周 | 2 天密集对话 | 远快于预期；不可外推 |
| P0 高保真页面 | 9 个（H1 + H2 + H3） | 38 个（M1-M6/M8/M10 + H1/H2/H3 全覆盖） | +322% |
| Layer 2 业务复合组件 | 12 个 | 16 个（+ DiffPanel / PageHeader / DataTable / EmptyState） | +33% |
| Layer 1 shadcn primitive | 不在计划中 | 9 个（Button / Input / Label / Card / Tabs / Select / Checkbox / Table / Dialog） | 新增 |
| Storybook 接入 | 计划在内但未指明版本 | 8.6.18 + 16 stories.tsx 全覆盖（与组件 1:1） | 完成 |
| 视觉基线治理 | 不在计划中 | accept_baseline.py + manifest.toml + 37 baseline PNG + OCR + imagehash + 截断检测 | 新增重大产出 |
| 组件库抽离至 packages/ui | 不在计划中 | commit e3ce5a0 + ADR-0028 + 117 文件改动 | 新增（用户决策驱动） |
| 技术 Spike 验证 | 5 项（结论入 docs/spikes/） | 5 项**计划**全落盘（状态=起草），验证未跑 | 计划达成 / 验证未启动 |
| ADR 产出 | 计划 ≥ 0021 | 4 份新增（0021 / 0022 / 0023 / 0028）+ 4 份占位（0024-0027 待 Spike accept） | 超出 |
| Wave 0.5 retro | 必交 | 本文档 | 完成 |

**结论**：交付内容远超 ROADMAP 设想，但**未完成 Spike 验证**——这是 Wave 1 启动的硬性前置。

---

## 2. 做对了的

### 2.1 高保真原型先行（ADR-0021 落地）
对 169 个用户故事中 133 个 UI 故事的"操作密度"做了诚实评估：PDA 14 步验收 / 双人签字 / 离线模式 / 14 项核对，不画原型直接写 RN 代码必然返工。
38 个页面让业务方走查能看到完整流程，**不是 Figma 静态图，是真组件 + 真状态切换 + 真扫码动画**。

### 2.2 视觉治理体系自洽（7 个 commit 演进）
从最初"截图 + 像素 diff"到完整治理：
- `bfd5ee4` 11 baseline + 像素对比脚本基线
- `671f59e` 底部截断检测（A3 类）
- `29d2bea` OCR 关键字命中替代人工 review（A4 类）
- `5fa518c` `accept_baseline.py` 工具 + 4 类标准
- `e81318b` baseline 入仓强制规范
- `430fdab` imagehash 集成（phash 比 mean_diff 对结构变化更敏感）
- `62bf9eb` `--accept-resize` 选项处理 viewport 变化

每次都是问题驱动的演进，不是过早抽象。

### 2.3 组件规范化（ADR-0022）
6 个早期组件（DualSignPanel / KanbanBoard 等）style 内联混搭 + 命名不一致，本可以"以后再改"放任。但 `18ced83` 一次性重写 6 组件 + 立刻 4 个治理脚本进 T1，杜绝后续累积。
**关键约束的 4 脚本治理**（doc-header / props-classname / no-inline-style / registry-consistency）让 16 个组件交付质量统一。

### 2.4 ComponentsGallery 替代 Storybook 早期空跑
Storybook 配置占位但未真接入是早期 bug（直到 review 才发现）。同期 `97e950e` 建了 ComponentsGallery 一站式走查页，**实质上替代了 Storybook 走查角色**。
`706a5f2` 扩到 16 组件全展示后，业务方走查体验比 Storybook controls 还直接。Storybook 在 `8cce777` 才真正接入做"开发者文档"用，分工清晰。

### 2.5 ADR-0023 业务报表方案 — 把分歧显式化
原型阶段 M6 报表的"GSP 法定 vs 业务自由"分歧不能用代码躲过。`82c51ea / d0926d2 / e7f101f` 三个 commit 把 ADR-0023 落地为方案 C：
- A. GSP 法定后端实现
- B. 业务报表 Metabase 嵌入（Wave 5）
- C. 业务页快捷入口
- D. 订阅 WMS 自实现

不强行用单一方案统一所有需求。

### 2.6 review-then-fix 的纪律
e3ce5a0 抽 packages/ui 后，主动做了 4 层 review（代码 / 治理 / 风险 / 流程）发现 5 处路径漂移：
- `governance/gate-rules.toml` 路径
- ADR-0022 §1 三层架构图
- `docs/prototypes/component-registry.md` 4 处假路径（`src/tokens/*` / `wms-theme.css` 从未存在）
- ROADMAP / TODO 与现实脱节
- accept_baseline.py 缺 resize 通道

`e45da12 / 95122c0 / 62bf9eb` 三个 commit 一次性清完。**review 不是形式，是发现"现实/文档/治理三方漂移"的唯一手段**。

### 2.7 Spike 计划带时间盒（不直接动手）
用户要求"前端代替原型"时，没有冲动地直接干 apps/web-admin，而是：
1. 区分 4 种路径（A 完全替换 / B 复制 / C 抽组件库 / D 严格 Spike）
2. 列出每条的 trade-off
3. 让用户选 C
4. 实施 + ADR-0028 备案

避免了"跳过 Spike 直接写生产代码"的硬阻塞（鉴权 / API 客户端契约都没定型）。

---

## 3. 做错了的 / 需要改进的

### 3.1 ADR / ROADMAP 与现实持续滞后
ADR-0021 写时设想"12 组件 + 9 P0 页"，实际做到"16 组件 + 38 页"，但 ADR 没修订。
ROADMAP § Wave 0.5 任务清单直到 review (95122c0) 才同步真实状态。
**改进**：每个 Wave 内 ≥ 50% 范围扩大就要更新 ROADMAP，不能积压到 retro 才修。

### 3.2 路径漂移在大重构后必然发生
e3ce5a0 抽 packages/ui 117 文件改动，5 处路径漂移（gate-rules / ADR / component-registry）都是同类型——**硬编码路径 + 大重构 = 漂移**。
**改进**：未来大型重构前先 grep 出所有可能受影响的硬编码路径，列入修改清单。当前的"事后 review"代价虽然 OK 但不可持续。

### 3.3 pnpm 11 严格模式反复踩坑
`onlyBuiltDependencies` 配置位置 / `--config.dangerously-allow-all-builds=true` / 自动改写 `pnpm-workspace.yaml` 的占位符等，前后踩了 4 次。
**改进**：把 pnpm 11 配置规范写入 `docs/infra/`（如 `pnpm-workspace.md`）作为运维知识，不要每次现搜。

### 3.4 Storybook 长期"占位但不真跑"
`.storybook/main.ts` + `preview.ts` 在最早期就建了，但 5 个 stories.tsx 写好却没装 storybook 包；preview.ts CSS import 路径错；package.json 无 storybook script。直到 step 3 review 才发现。
**改进**：占位代码必须立刻被 build 路径覆盖（"先让坏的 build，再优化"）。本次教训是不要让"占位"和"未实现"留作未跟踪的技术债。

### 3.5 视觉 baseline 重复签字成本
viewport 一调整就触发 5 个 tab 的 baseline mismatch，需要重新 capture + accept。本周用了 4 次。
**改进**：未来 viewport 调整需在 commit message 明确说"会触发 baseline 重新接受"作为预期；考虑加 T3 自动化 capture（避免人工再跑 vite dev + chrome）。

### 3.6 TS strict 没有真正的门禁验证
prototypes/tsconfig.json 和 packages/ui/tsconfig.json 都开了 `strict: true`，但 vite build 默认不做严格类型检查（仅转译）。如果有类型错误也能 build 过。
**改进**：T1 加 `tsc --noEmit` 检查（Wave 1 启动时落地，避免此 Wave 范围扩大）。

### 3.7 业务边界澄清做了三次
M-TC 追溯码与药监 EDI 边界（"码上放心"由 ERP 还是 WMS 处理）：
- 第 1 次 ADR-0007 v0.3 决策（M11 移除）
- 第 2 次 prototypes 阶段又出现"码上放心上报"原型 → 6f38a0f 改回"追溯码数据导出"
- 第 3 次 review 时 ADR-0028 后续 backlog 再次确认

**改进**：边界澄清进 `docs/domain/clarifications.md` 后应当作"业务红线"，UI 提示词、API scope、ADR 后续段全部强制对齐。当前是"决策做了但实施时被忘记"。

---

## 4. Spike 状态

| ID | 状态 | 关联 ADR | 阻塞 |
|----|------|---------|------|
| SPIKE-001 Axum + JWT | 起草 | 拟产出 ADR-0024 | Wave 1 W1.A 启动前必跑 |
| SPIKE-002 H2 append-only | 起草 | 拟产出 ADR-0025 | Wave 1 W1.B 启动前必跑 |
| SPIKE-003 utoipa→OpenAPI→TS | 起草 | 拟产出 ADR-0026 | **Wave 1 W1.C / packages/api-client 启动前必跑（最高优先级）** |
| SPIKE-004 SQLx offline | 起草 | ADR-0001 附录 | Wave 1 W1.A/B 编译前必跑 |
| SPIKE-005 RN 扫枪 | 起草 | 拟产出 ADR-0027 | Wave 3 W3.A PDA 业务启动前必跑（可后置） |

5 项合计时间盒 8.5 天。**当前节奏铁律阻塞 Wave 1**——Wave 0.5 退出条件之一是"Spike 进入 accept/reject/defer"，全部停在"起草"。

**建议执行顺序**：
1. SPIKE-003（1.5 天）— 最轻量，产出会被 001 / 005 复用（packages/api-client 模式）
2. SPIKE-001（2 天）— 鉴权契约，决定 Wave 1 W1.A 的 handler 签名
3. SPIKE-002（2 天）— 审计存储，决定 Wave 1 W1.B 表结构
4. SPIKE-004（1 天）— SQLx，决定 Wave 1 后端编译流程
5. SPIKE-005（2 天）— RN 扫枪，可后置到 Wave 3 启动前

---

## 5. ADR 状态

| ADR | 状态 | 备注 |
|-----|------|------|
| 0021 高保真原型策略 | Accepted | 约束生效，约束 1 (`prototypes/`) 因 ADR-0028 路径细化但精神一致 |
| 0022 原型组件规范 | Accepted v0.2 | 加 v0.2 修订记录段（路径迁移 packages/ui） |
| 0023 业务报表方案选型 | Accepted | 混合 4 方案（A/B/C/D），原型阶段验证 |
| 0028 组件库抽离 | Accepted | 本 Wave 大动作，已备案 |
| 0024 鉴权模型（拟） | 占位 | SPIKE-001 accept 后写 |
| 0025 审计存储模型（拟） | 占位 | SPIKE-002 accept 后写 |
| 0026 跨端契约管线（拟） | 占位 | SPIKE-003 accept 后写 |
| 0027 PDA 离线模型（拟） | 占位 | SPIKE-005 accept 后写 |

---

## 6. Baseline 复盘

视觉基线总数：37 个 PNG（manifest.toml 18 个 tab 起步 → 加新页持续扩展）。
最终 reviewed_at 全签字 2026-05-23 / reviewed_by="项目主人"。
本 Wave 累计 baseline 接受动作 ~6 次，对应每个新页面入仓 + 文案打磨 + viewport 调整后的更新。

`governance/baselines/` 仍为空——治理脚本 baseline（debt baseline）未产生，因为 T1 全程 0 违规。
首个 debt baseline 预计在 Wave 1 真业务代码进入时产生（如某个 handler 的 11 层测试覆盖度初次低于 100%）。

---

## 7. Wave 1 准入条件检查

| 条件 | 状态 |
|------|------|
| Wave 0.5 ADR 全部 Accepted | ✅ 0021/0022/0023/0028 |
| Storybook 可运行 | ✅ build 7.65s（commit 8cce777） |
| P0 原型 ≥1 次走查 approved | ✅ manifest 37 个 tab 全签字 |
| packages/ui 抽离 | ✅ commit e3ce5a0 + ADR-0028 |
| Spike 计划落盘 | ✅ commit b2e84eb（5 项 + README） |
| **Spike 进入 accept/reject/defer** | ❌ 全部停在"起草" |
| 任一 accept 的 Spike 都有 ADR | ⏸ 等 Spike 跑后 |
| Wave 0.5 retro 完成 | ✅ 本文档 |
| T1 治理 24/24 全绿（持续条件） | ✅ ~2.24s |

**结论**：**Wave 0.5 不能退出**。唯一阻塞是 5 项 Spike 验证（合计时间盒 8.5 天）。
其他条件全部达成。Wave 1 启动前必须跑完 Spike-001 / 002 / 003 / 004（005 可推迟到 Wave 3）。

---

## 8. 节奏反思

### 8.1 2 天产出 36 个 commit 的代价

Wave 0.5 在 2 天内产出 36 个 commit（含 5 份 ADR、4 个 Spike 计划、~140 个 tsx 文件、~30 个治理脚本演进）。这个密度**主要靠 AI 协作 + 用户当裁判** 实现，**不是真实工程节奏的代表**。
具体来说：
- 没有实测成本（仅有 vite/storybook build 时间，没有 RN 真机扫码 / cargo 编译 / DB 连接）
- 没有 PR 评审（单分支推进）
- 没有真实业务联调（mock 数据）

**警示**：进入 Wave 1 后业务代码 + TDD + 真测试链路联通后，每天 commit 数会下降到 3-8 个量级。不要把 Wave 0.5 节奏当基线。

### 8.2 用户决策的关键时刻

本 Wave 有 4 个关键决策点用户必须拍板：
1. "前端代替原型？"→ 选 C 步骤 1（抽组件库）
2. "Storybook 怎么处理？"→ 选 A（真接入）
3. "review 后修哪些？"→ 选"都做"
4. "下一步 Spike 还是 retro？"→ 选 retro

每次 AI 都给了 2-4 个候选 + trade-off，不擅自代替决策。这是项目治理纪律的体现，**不应该在 Wave 1 业务代码阶段松懈**。

### 8.3 review-then-fix 的价值

第三次"review→修"是本 Wave 最大的工程纪律收获。在没有 review 的情况下：
- gate-rules.toml 路径不会被发现
- component-registry.md 假路径会一直误导新人
- ADR-0022 §1 路径会跟现实脱节
- accept_baseline 缺 resize 通道不会暴露

**review 不是形式步骤**，是 commit 序列中的"自我对账"。建议 Wave 1 起每个大节点（≥ 5 个 commit）后强制做 review，写入 `docs/governance.md` §x 或 ADR-0003 附录。

---

## 9. 下一步

1. **跑 SPIKE-003**（utoipa→OpenAPI→TS，最轻量、产出会被 001/005 复用）
2. SPIKE-003 accept → 写 ADR-0026
3. **跑 SPIKE-001 + SPIKE-002**（鉴权 + 审计；Wave 1 W1.A/W1.B 直接前置）
4. 各自 accept → 写 ADR-0024 / 0025
5. **跑 SPIKE-004**（SQLx offline，1 天）→ ADR-0001 附录
6. SPIKE-005 推迟（PDA 是 Wave 3 才用，先验证 PC 链路）
7. Wave 0.5 退出 → 启动 Wave 1 W1.A
8. 同步启动外部资质（"码上放心"账号开通——这是 Wave 4 阻塞，越早启动越好）

---

## 10. 哲学自检

按 AGENTS.md 风险分级标准与节奏铁律：

- ✅ 没有跨 Wave 私自并行：未启动 apps/web-admin（Wave 1）真业务代码
- ✅ 没有压缩 TDD 节奏：本 Wave 全是原型 + 治理，没有写业务规则
- ✅ 重大决策都有 ADR 备案：0021/0022/0023/0028 + 5 项 Spike 文档
- ✅ 范围扩大但合理：38 vs 9 页 / 16 vs 12 组件，业务方走查覆盖更全面，符合 ADR-0021 精神
- ⚠️ "ROADMAP / ADR 滞后" 是治理债，但 review 时已清；下个 Wave 起改进
- ⚠️ 节奏不可外推：2 天 36 commit 不是 Wave 1 业务代码节奏的预测值

**结论**：Wave 0.5 工程纪律基本到位。可推进 Spike → ADR → Wave 1。


---

## 11. Wave 0.5 持续演进（2026-05-24，retro 写完之后）

> 原 retro 写于 2026-05-24 commit 96bb477（Wave 0.5 retro 任务点）。
> retro 后又有 ~5 个 commit 进入 Wave 0.5 范畴（Spike 验证 + ADR review + 修风险）；
> 仿照 wave-0-retro.md §10 的"持续演进"模式记录补记。

### 11.1 Spike 验证 4 轮（实工 ~10.5h vs 时间盒 8.5d）

| commit | Spike | 实工 | 关键产出 |
|--------|-------|------|---------|
| b3df10d | SPIKE-003 utoipa→OpenAPI→TS | ~4h | ADR-0026 Proposed |
| 288a21c | SPIKE-001 Axum + JWT | ~3h | ADR-0024 Proposed + pda-offline-state.md |
| (合入) | SPIKE-004 SQLx offline | ~1.5h | ADR-0001 §SQLx 附录 v0.2 |
| (合入) | SPIKE-002 H2 append-only | ~2h | ADR-0025 Proposed |
| f2614bb | SPIKE-005 RN 扫枪 deferred | 0h | 推迟到 Wave 3，启动条件入 ROADMAP |

10x 加速比的真实原因：spike 性质（小而集中、不写生产代码、复用现成 PG/cargo/pnpm 工具链）+ AI 协作 + mock 数据。
**节奏不可外推到 Wave 1 业务代码**（已在 §8 反复强调，本节再确认）。

### 11.2 review-then-fix 周期 ×2 轮

**第 1 轮**（commit a781f70）—— 4 份 ADR Proposed 集中 review：
- 标注 3 处风险（permissions 滞后 / Redis 单点 / hash chain 并发未验证）
- 修法：ADR-0024 §2.1.1 混合失效模式 + §2.3.1 故障降级 / ADR-0025 §2.4.1 spike-002b fallback
- 4 份 ADR Proposed → Accepted（0024/0025 v0.2，0026 直接 Accepted）

**第 2 轮**（本 commit）—— Accepted 后再 review：
- 标注 4 处漏项（docs/error-codes.md 缺 AUTH-001..009 / ADR-0024 §2.8 引用不存在的 SPIKE-006 / pda-offline-state.md 没引 iat / retro 没记 review 周期）
- 修法：本 commit 全修

**经验**：Accept 不等于 done。Wave 1 起每个大节点（ADR 落地 / Wave 退出前）都应跑两轮 review：
- 第 1 轮：决策合理性（spike 假设是否覆盖、风险是否识别）
- 第 2 轮：现实一致性（ADR 互引是否真存在、文档暗示的事是否真做了、命名是否笔误）

### 11.3 隐藏漏项的反思

第 2 轮 review 揭示的"我说做了但没做"模式：
- ADR-0024 §2.6 标题"（入 docs/error-codes.md）"是承诺，但仅写了承诺没真做
- ADR-0024 §2.8 引用 SPIKE-006 是笔误，没人审就过 Accepted

**改进**：
- 类似的"暗示已做"必须显式验证（grep / ls / curl）；写在 ADR review 流程里
- 治理脚本 `check_doc_links.py` 是文件链接级别，不能检测"§2.6 标题写了 docs/error-codes.md 但内容没加"。Wave 1 起评估扩展：ADR 承诺类语句（"入 X" / "加到 Y"）必须有对应实证文件改动（diff 触发）

### 11.4 Wave 0.5 真正完整退出 commit 链（按时间序）

```
8704578 文档(原型): ADR-0021 高保真原型策略 + Wave 0.5 + 治理脚本 5 个   <- 起点
... (35 commit 原型 + 治理 + Spike 计划)
e3ce5a0 重构(原型,治理): 抽 packages/ui 共享包，prototypes 改用 @wms/ui
e45da12 文档(治理): 路径漂移修复 + ADR-0028 备案 packages/ui 抽离
95122c0 文档(文档): ROADMAP / TODO 同步 Wave 0.5 实际进度
62bf9eb 构建(治理,原型): 视觉基线全量重新接受 + accept_baseline 加 --accept-resize
96bb477 文档(治理): Wave 0.5 retrospective + retro 命名扩展支持小数 Wave   <- retro 写完
b3df10d 功能(接口): SPIKE-003 utoipa→OpenAPI→TS 全链路验证 accept + ADR-0026
288a21c 功能(接口): SPIKE-001 Axum+JWT+多租户验证 accept + ADR-0024
(SPIKE-004 / SPIKE-002 commit)
f2614bb 文档(治理): SPIKE-005 deferred + Wave 0.5 退出条件全达成
a781f70 文档(治理): ADR-0024/0025/0026 review 修风险 + Proposed → Accepted
本 commit 文档(治理): 修 review 第二轮 4 处漏项                            <- 真正退出
```

**最终 Wave 0.5 退出条件 8/8 全 ✅**（详见 §7 与本节 §11.4）。
