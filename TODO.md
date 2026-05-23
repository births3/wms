# wms TODO（当前 Wave）

> 任务追踪粒度：当前 Wave 内的具体任务。
> Wave 切换时归档当前 TODO 并启动新 TODO。
> 长期路线见 [ROADMAP.md](ROADMAP.md)。

---

## 当前 Wave：Wave 0.5 — 原型 + 技术 Spike + 组件库抽离

**目标**：组件库骨架 + P0 原型 + 技术 Spike 验证 + Wave 1 复用准备。

### 已完成

#### 治理 / 文档（Wave 0 续）
- [x] T1 治理脚本扩到 24 个（含 component-* 4 个 + visual-* 3 个 + prototype-* 5 个）
- [x] `governance/gate-rules.toml` 路径触发表（含 prototypes/* 与 packages/ui/src/business/**）
- [x] `governance/baselines/` baseline 机制
- [x] ADR-0021..0023, 0028 起 7 份新 ADR（含 ADR-0008 Odoo 借鉴 / ADR-0019 货主自定义）

#### 高保真原型（ADR-0021）
- [x] `prototypes/` Vite + React + TS + shadcn/ui 工程骨架
- [x] **38 个高保真页面**（M1/M2/M3/M4/M5/M6/M8/M10 + H1/H2/H3，远超 ROADMAP P0 9 页）
- [x] **16 个 Layer 2 业务复合组件**（StatusBadge / ScanInput / DualSignPanel / AuditTimeline / ApprovalFlow / KanbanBoard / PrintPreview / RuleEditor / TempChart / OfflineIndicator / FieldTable / StepFlow / DiffPanel / PageHeader / DataTable / EmptyState）
- [x] **9 个 shadcn primitive**（Button / Input / Label / Card / Tabs / Select / Checkbox / Table / Dialog）
- [x] **16 个组件 Stories.tsx**（与组件 1:1）+ Storybook 8.6 接入（build 通过 7.65s）
- [x] 视觉基线治理：`accept_baseline.py` + `manifest.toml` + 18 个 baseline 全签字
- [x] OCR 关键字校验（`check_visual_keywords.py`）

#### 跨包共享（ADR-0028 / commit e3ce5a0）
- [x] **packages/ui 抽离**：16 业务组件 + 9 shadcn primitive + lib/utils + globals.css → `@wms/ui`
- [x] 仓库根 `pnpm-workspace.yaml`（packages/* + prototypes + apps/*）
- [x] Tailwind preset 化（`packages/ui/tailwind-preset.cjs`）
- [x] 153 处 import 改为 `@wms/ui`，治理脚本路径同步

#### 技术 Spike 计划（commit b2e84eb）
- [x] `docs/spikes/README.md` + 流程规范（accept/reject/defer 三态、时间盒、与 ADR 的关系）
- [x] SPIKE-001 Axum + JWT（2 天，关联 W1.A）
- [x] SPIKE-002 H2 append-only（2 天，关联 W1.B）
- [x] SPIKE-003 utoipa → OpenAPI → TS（1.5 天，关联 W1.C）
- [x] SPIKE-004 SQLx offline（1 天）
- [x] SPIKE-005 RN 扫枪（2 天，关联 PDA）

### 进行中 / 待做

- [ ] **跑 5 个 Spike 验证**（合计上限 8.5 天；建议顺序 003 → 001 → 002 → 004 → 005）
  - [ ] SPIKE-003 utoipa→OpenAPI→TS（最轻量，产出会被 001 / 005 复用）
  - [ ] SPIKE-001 Axum + JWT（鉴权契约 → ADR-0024）
  - [ ] SPIKE-002 H2 append-only（审计存储 → ADR-0025）
  - [ ] SPIKE-004 SQLx offline（编译期 SQL → ADR-0001 附录）
  - [ ] SPIKE-005 RN 扫枪（PDA 离线 → ADR-0027）
- [ ] 升级 ADR-0024 / 0025 / 0026 / 0027（Spike accept 后写）
- [x] **Wave 0.5 retro**（`docs/retros/wave-0.5-retro.md`，246 行 / 10 节）
- [x] 重新 capture visual snapshot 验证 e3ce5a0 后渲染（commit 62bf9eb：37 baseline 全部完美 mean_diff=0.00）

---

## Wave 0.5 退出条件（Wave 1 准入）

- [x] Storybook 可运行（commit 8cce777，build 通过）
- [x] P0 原型 ≥1 次走查 approved（manifest 18 tab 全签字）
- [x] packages/ui 抽离（commit e3ce5a0，ADR-0028 备案）
- [x] Spike 计划落盘（commit b2e84eb，5 项 docs/spikes/*.md）
- [ ] **5 项 Spike 至少进入 accept / reject / defer 三态之一**（不允许停在"起草"）
- [ ] 任一 accept 的 Spike 都有对应 ADR（0024-0027）
- [x] Wave 0.5 retro 写完（`docs/retros/wave-0.5-retro.md`，246 行 / 10 节）
- [ ] T1 治理 24/24 全绿（持续条件）

---

## 后续 Wave 预告（不在当前 TODO，仅参考）

- **Wave 1**：H1 权限/多租户、H2 审计追踪、H3 OpenAPI 工具链；`apps/web-admin/` 启动复用 `@wms/ui`；`packages/api-client/` 自动生成
- **Wave 2**：M1.a 基础档案 + M2 入库 schema + M6 报表骨架
- **Wave 3**：M2/M3 业务规则 + M5 冷链 schema + M9 计费账户

详见 [ROADMAP.md](ROADMAP.md)。
