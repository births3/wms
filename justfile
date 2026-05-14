# wms 任务编排：4 个治理 Tier 执行入口
#
# 详细规则见：docs/governance.md §2.2、ADR-0003、ADR-0006
#
# Tier 时间预算（硬上限）：
#   T1 quick-check  < 10s    写代码时随手跑、pre-commit 自动
#   T2 task-check   < 120s   任务结束、commit 前
#   T3 preflight    < 5min   推送前、pre-push、PR 创建前
#   T4 verify       < 30min  合并前、CI、发版前
#
# 用法：
#   just                    显示所有可用任务
#   just quick-check        Tier 1
#   just task-check         Tier 2
#   just preflight          Tier 3
#   just verify             Tier 4
#
# 注意：第 0 周阶段，Rust / 前端工具链尚未引入，许多命令为占位
#       带 [WAVE-N] 标记的命令将在对应波次启动时启用

set shell := ["bash", "-cu"]
set dotenv-load := true

# 仓库根目录的绝对路径（所有 worktree 内行为一致）
ROOT := justfile_directory()

# 默认显示帮助
default:
    @just --list

# ============================================================
# Tier 1: quick-check（< 10 秒）
# ============================================================
# 写代码时随手跑、pre-commit 自动触发
# 仅做：格式、lint、提交规范，不跑测试
# ============================================================
quick-check: _t1-banner _t1-fmt _t1-lint _t1-commit-conv

_t1-banner:
    @echo "▶ Tier 1 quick-check (target < 10s)"

_t1-fmt:
    @echo "  · format check (placeholder, [WAVE-1])"
    # cargo fmt --all -- --check
    # pnpm -r exec prettier --check .

_t1-lint:
    @echo "  · lint check (placeholder, [WAVE-1])"
    # cargo clippy --workspace --all-targets --no-deps -- -D warnings
    # pnpm -r run lint --quiet

_t1-commit-conv:
    @python3 scripts/governance/check_commit_convention.py --staged || true

# ============================================================
# Tier 2: task-check（< 120 秒）
# ============================================================
# 任务结束、commit 前
# T1 + diff 触发的最小治理集 + L1 单元测试 + L2 静态契约
# ============================================================
task-check: quick-check _t2-banner _t2-diff-checks _t2-unit-tests _t2-contract-static

_t2-banner:
    @echo "▶ Tier 2 task-check (target < 120s)"

_t2-diff-checks:
    @echo "  · diff-driven governance checks"
    @python3 scripts/governance/task_check.py --tier T2 || true

_t2-unit-tests:
    @echo "  · L1 unit tests (placeholder, [WAVE-2])"
    # cargo test --workspace --lib
    # pnpm -r run test:unit

_t2-contract-static:
    @echo "  · L2 API contract static (placeholder, [WAVE-2])"
    # python3 scripts/governance/validate_openapi_artifacts.py

# ============================================================
# Tier 3: preflight（< 5 分钟）
# ============================================================
# 推送前、pre-push、PR 创建前
# T2 + L3 业务流程 + L4 错误 + L5 数据一致 + L8 权限 + L11 幂等
# ============================================================
preflight: task-check _t3-banner _t3-integration _t3-governance-l3

_t3-banner:
    @echo "▶ Tier 3 preflight (target < 5min)"

_t3-integration:
    @echo "  · L3-L5/L8/L11 integration tests (placeholder, [WAVE-3])"
    # cargo test --workspace
    # pnpm -r run test
    # pnpm -r run test:integration

_t3-governance-l3:
    @echo "  · governance T3 checks"
    @python3 scripts/governance/governance_checks.py --tier T3 || true

# ============================================================
# Tier 4: verify（< 30 分钟）
# ============================================================
# 合并前、CI、发版前
# T3 + L6 并发 + L7 性能 + L9 兼容 + L10 可观测 + 完整 E2E + 合规追溯
# ============================================================
verify: preflight _t4-banner _t4-full-tests _t4-e2e _t4-perf-bench _t4-compat-check _t4-governance-l4

_t4-banner:
    @echo "▶ Tier 4 verify (target < 30min)"

_t4-full-tests:
    @echo "  · full test suite incl. release mode (placeholder, [WAVE-4])"
    # cargo test --workspace --release

_t4-e2e:
    @echo "  · E2E tests (placeholder, [WAVE-3])"
    # pnpm -r run test:e2e

_t4-perf-bench:
    @echo "  · L7 performance baselines (placeholder, [WAVE-4])"
    # cargo bench --workspace
    # python3 scripts/governance/check_perf_baseline.py

_t4-compat-check:
    @echo "  · L9 OpenAPI compatibility (placeholder, [WAVE-3])"
    # python3 scripts/governance/check_api_compat.py

_t4-governance-l4:
    @echo "  · governance T4 checks (full)"
    @python3 scripts/governance/governance_checks.py --tier T4 || true

# ============================================================
# 治理脚本独立入口
# ============================================================
# 单独跑某个治理脚本（开发治理脚本时调试用）
# ============================================================

# 跑环境检查
gov-env:
    @python3 scripts/governance/validate_environment.py

# 跑文档链接检查
gov-doc-links:
    @python3 scripts/governance/check_doc_links.py

# 跑 ADR 索引检查
gov-adr-index:
    @python3 scripts/governance/validate_adr_index.py

# 跑提交规范检查（最近一次提交）
gov-commit:
    @python3 scripts/governance/check_commit_convention.py --last

# 跑全部 T1 治理脚本
gov-t1:
    @python3 scripts/governance/governance_checks.py --tier T1

# 跑全部 T2 治理脚本
gov-t2:
    @python3 scripts/governance/governance_checks.py --tier T2

# ============================================================
# 工作流辅助
# ============================================================

# 显示当前 Wave / 当前切片状态
status:
    @echo "wms governance status"
    @echo "  ROOT: {{ROOT}}"
    @echo "  branch: $(git branch --show-current 2>/dev/null || echo 'no-git')"
    @echo "  worktree: $(git rev-parse --git-common-dir 2>/dev/null || echo 'no-git')"
    @echo ""
    @if [ -f TODO.md ]; then echo "── TODO.md ──"; head -20 TODO.md; fi

# 列出所有治理脚本
gov-list:
    @ls -1 scripts/governance/*.py 2>/dev/null | grep -v "^_" || echo "(no scripts yet)"

# 检查各 Tier 实际耗时（写入 governance/baselines/tier-runtime.json）
tier-timing:
    @echo "▶ measuring tier runtimes (placeholder, [WAVE-2])"
    # 后续实现：分别计时 quick-check / task-check / preflight / verify

# ============================================================
# Wave 启动检查（占位）
# ============================================================
# 进入新 Wave 前必须通过的前置条件检查
# ============================================================

# 进入 Wave 1 前置检查
wave-1-ready:
    @echo "checking Wave 1 entry conditions (placeholder)"
    @python3 scripts/governance/validate_environment.py
    @python3 scripts/governance/validate_adr_index.py
    @python3 scripts/governance/check_doc_links.py
    @echo "✓ Wave 0 governance scaffold complete"
