# wms TODO（当前 Wave）

> 任务追踪粒度：当前 Wave 内的具体任务。
> Wave 切换时归档当前 TODO 并启动新 TODO。
> 长期路线见 [ROADMAP.md](ROADMAP.md)。

---

## 当前 Wave：Wave 0 — 治理骨架

**目标**：治理体系、文档、配置、脚本骨架就位。

### 已完成

- [x] 目录骨架（docs/{adr,domain,compliance}, scripts/governance, governance/baselines, apps/{web-admin,pda-mobile}, backend/crates, packages, shared/openapi）
- [x] Git 仓库初始化（main 分支、禁用 GPG 签名）
- [x] `.gitignore` `.editorconfig` `.gitattributes`
- [x] `docs/governance.md` v0.2
- [x] ADR-0001 技术栈
- [x] ADR-0002 仓库结构
- [x] ADR-0003 治理模型（含 L1-L4 → T1-T4 重命名 + TDD 集成小节）
- [x] ADR-0004 v0.2 波次路线（已由 ADR-0007 取代）
- [x] ADR-0007 v0.3 路线边界对齐
- [x] ADR-0006 TDD + 11 层测试维度
- [x] `docs/architecture-dependencies.md` 依赖图
- [x] `justfile`（T1-T4 入口）
- [x] `lefthook.yml`（pre-commit / commit-msg / pre-push 三钩子）
- [x] 治理脚本：`_baseline.py`、`_diff.py`、4 个起步脚本（环境 / 链接 / ADR / 提交）、2 个调度脚本（governance_checks / task_check）
- [x] `governance/gate-rules.toml` + `governance/baselines/README.md`
- [x] `README.md` / `ROADMAP.md` / 本 `TODO.md`
- [x] `docs/adr/README.md` 索引

### 进行中 / 待做

- [ ] `CHANGELOG.md` 初始版本
- [ ] 工作区根 README 登记 wms 项目
- [ ] 本地验证：跑 `validate_environment.py` / `check_doc_links.py` / `validate_adr_index.py` / `check_commit_convention.py`
- [ ] 首次 commit（feat(governance): wave 0 scaffolding）
- [ ] 哲学自检（task 12）

---

## Wave 0 退出条件（Wave 1 准入）

- [ ] 所有 Wave 0 ADR 状态 = Accepted
- [ ] `python3 scripts/governance/governance_checks.py --tier T1` 全绿
- [ ] `validate_environment.py` 报告必需工具就绪
- [ ] 首次 commit 通过 lefthook 钩子
- [ ] 完成 Wave 0 retro（写入 `docs/retros/wave-0-retro.md`，目前可暂缓到首次 commit 后）

---

## 后续 Wave 预告（不在当前 TODO，仅参考）

- **Wave 1**：H1 权限/多租户、H2 审计追踪、H3 OpenAPI 工具链 + 外部资质并行启动
- **Wave 2**：M1.a 基础档案 + M2 入库 schema + M6 报表骨架
- **Wave 3**：M2/M3 业务规则 + M5 冷链 schema + M9 计费账户

详见 [ROADMAP.md](ROADMAP.md)。
