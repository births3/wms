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
# 写代码时随手跑、pre-commit 自动触发
# 仅做：格式、lint、提交规范，不跑测试
# ============================================================

# Tier 1 quick-check (< 10s): fmt + lint + commit convention
quick-check: _t1-banner _t1-fmt _t1-lint _t1-commit-conv

_t1-banner:
    @echo "▶ Tier 1 quick-check (target < 10s)"

_t1-fmt:
    @echo "  · format check (placeholder, [WAVE-1])"
    @# cargo fmt --all -- --check
    @# pnpm -r exec prettier --check .

_t1-lint:
    @echo "  · lint check (placeholder, [WAVE-1])"
    @# cargo clippy --workspace --all-targets --no-deps -- -D warnings
    @# pnpm -r run lint --quiet

_t1-commit-conv:
    @python3 scripts/governance/check_commit_convention.py --staged || true

# ============================================================
# Tier 2: task-check（< 120 秒）
# 任务结束、commit 前
# T1 + diff 触发的最小治理集 + L1 单元测试 + L2 静态契约
# ============================================================

# Tier 2 task-check (< 120s): T1 + diff-driven + L1/L2
task-check: quick-check _t2-banner _t2-diff-checks _t2-unit-tests _t2-contract-static

_t2-banner:
    @echo "▶ Tier 2 task-check (target < 120s)"

_t2-diff-checks:
    @echo "  · diff-driven governance checks"
    @python3 scripts/governance/task_check.py --tier T2 --strict

_t2-unit-tests:
    @echo "  · L1 unit tests (placeholder, [WAVE-2])"
    @# cargo test --workspace --lib
    @# pnpm -r run test:unit

_t2-contract-static:
    @echo "  · L2 API contract static (placeholder, [WAVE-2])"
    @# python3 scripts/governance/validate_openapi_artifacts.py

# ============================================================
# Tier 3: preflight（< 5 分钟）
# 推送前、pre-push、PR 创建前
# T2 + L3 业务流程 + L4 错误 + L5 数据一致 + L8 权限 + L11 幂等
# ============================================================

# Tier 3 preflight (< 5min): T2 + L3-L5/L8/L11
preflight: task-check _t3-banner _t3-integration _t3-governance-l3

_t3-banner:
    @echo "▶ Tier 3 preflight (target < 5min)"

_t3-integration:
    @echo "  · L3-L5/L8/L11 integration tests (placeholder, [WAVE-3])"
    @# cargo test --workspace
    @# pnpm -r run test
    @# pnpm -r run test:integration

_t3-governance-l3:
    @echo "  · governance T3 checks"
    @python3 scripts/governance/governance_checks.py --tier T3 || true

# ============================================================
# Tier 4: verify（< 30 分钟）
# 合并前、CI、发版前
# T3 + L6 并发 + L7 性能 + L9 兼容 + L10 可观测 + 完整 E2E + 合规追溯
# ============================================================

# Tier 4 verify (< 30min): T3 + L6/L7/L9/L10 + E2E
verify: preflight _t4-banner _t4-full-tests _t4-e2e _t4-perf-bench _t4-compat-check _t4-governance-l4

_t4-banner:
    @echo "▶ Tier 4 verify (target < 30min)"

_t4-full-tests:
    @echo "  · full test suite incl. release mode (placeholder, [WAVE-4])"
    @# cargo test --workspace --release

_t4-e2e:
    @echo "  · E2E tests (placeholder, [WAVE-3])"
    @# pnpm -r run test:e2e

_t4-perf-bench:
    @echo "  · L7 performance baselines (placeholder, [WAVE-4])"
    @# cargo bench --workspace
    @# python3 scripts/governance/check_perf_baseline.py

_t4-compat-check:
    @echo "  · L9 OpenAPI compatibility (placeholder, [WAVE-3])"
    @# python3 scripts/governance/check_api_compat.py

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

# 生成主仓 OpenAPI JSON 并刷新 @wms/api-client 类型
openapi-sync:
    @cd backend && cargo run --quiet --bin openapi-export > ../shared/openapi/openapi.json
    @pnpm --filter @wms/api-client gen:schema

# 检查主仓 OpenAPI JSON 与后端 utoipa 定义同步
openapi-check:
    @python3 scripts/governance/check_openapi_in_sync.py --strict

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

# 报告 Wave 1 完成度（默认不阻塞；出口检查用 --strict）
wave-1-status:
    @python3 scripts/governance/report_wave1_completion.py

# Wave 1 出口检查（未完成返回非零）
wave-1-complete-check:
    @python3 scripts/governance/report_wave1_completion.py --strict

# 报告 Wave 2 完成度（默认不阻塞；出口检查用 --strict）
wave-2-status:
    @python3 scripts/governance/report_wave2_completion.py

# Wave 2 开发完成出口检查（真实 dev/staging runtime evidence 作为预发布 gate 单独验证）
wave-2-complete-check:
    @python3 scripts/governance/report_wave2_completion.py --strict

# Wave 2 配置中心 Feature Flag runtime evidence 预发布验证
wave-2-runtime-evidence-validate:
    @python3 scripts/governance/report_wave2_completion.py --strict --require-runtime-evidence

# 定向验证两份 Wave 1 runtime evidence JSON（不检查其他静态完成项）
wave-1-runtime-evidence-validate:
    @python3 scripts/governance/validate_wave1_runtime_evidence.py --kind all

# Wave 1 runtime evidence 前置检查：H2 dev PostgreSQL + wrk 输入边界
wave-1-runtime-prereq-h2:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode h2

# Wave 1 H2 DB readiness：跑 1 小时 wrk 前先确认 dev DB 基线与封档已满足
wave-1-h2-runtime-readiness:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode h2 && \
      python3 scripts/governance/check_wave1_h2_runtime_readiness.py \
        --database-url "$WAVE1_H2_DATABASE_URL"

# Wave 1 runtime evidence 前置检查：k8s 自动回滚输入边界
wave-1-runtime-prereq-rollback-k8s:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-k8s

# Wave 1 W1.D readiness：校验 k8s 自动回滚边界，不触发 rollback、不写 evidence
wave-1-rollback-runtime-readiness-k8s:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-k8s && \
      deploy/scripts/wave1_auto_rollback_probe.sh \
        --check-only \
        --environment "$WAVE1_ROLLBACK_ENVIRONMENT" \
        --target k8s \
        --deployment "${WAVE1_K8S_DEPLOYMENT:-wms-api}" \
        --context "$WAVE1_K8S_CONTEXT" \
        --namespace "$WAVE1_K8S_NAMESPACE" \
        --evidence-file docs/retros/wave-1-runtime-evidence.json \
        --rollback-log-ref "$WAVE1_ROLLBACK_LOG_REF" \
        --external-log-ref "$WAVE1_EXTERNAL_LOG_REF"

# Wave 1 runtime evidence 前置检查：docker-compose 自动回滚输入边界
wave-1-runtime-prereq-rollback-compose:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-compose

# Wave 1 W1.D readiness：校验 docker-compose 自动回滚边界，不触发 rollback、不写 evidence
wave-1-rollback-runtime-readiness-compose:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-compose && \
      deploy/scripts/wave1_auto_rollback_probe.sh \
        --check-only \
        --environment "$WAVE1_ROLLBACK_ENVIRONMENT" \
        --target docker-compose \
        --previous-version "$WAVE1_PREVIOUS_VERSION" \
        --compose-file "$WAVE1_COMPOSE_FILE" \
        --evidence-file docs/retros/wave-1-runtime-evidence.json \
        --rollback-log-ref "$WAVE1_ROLLBACK_LOG_REF" \
        --external-log-ref "$WAVE1_EXTERNAL_LOG_REF"

# 采集 Wave 1 H2 runtime 证据：必须在真实 dev PostgreSQL + wrk 压测完成后运行
wave-1-h2-runtime-evidence:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode h2 --require-wrk-output && \
      python3 scripts/governance/collect_wave1_h2_runtime_evidence.py \
        --database-url "$WAVE1_H2_DATABASE_URL" \
        --wrk-output "$WAVE1_H2_WRK_OUTPUT" \
        --benchmark-log-ref "$WAVE1_H2_BENCHMARK_LOG_REF" \
        --cron-log-ref "$WAVE1_H2_CRON_LOG_REF" \
        --duration-seconds "${WAVE1_H2_DURATION_SECONDS:-3600}" \
        --target-qps "${WAVE1_H2_TARGET_QPS:-1000}" \
        --seal-failure-count "${WAVE1_H2_SEAL_FAILURE_COUNT:-0}"

# 采集 Wave 1 k8s 自动回滚 runtime 证据：SMOKE_URL 或 PROMETHEUS_URL + PROMETHEUS_QUERY 必须指向真实 dev/staging
wave-1-rollback-runtime-evidence-k8s:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-k8s && \
      deploy/scripts/wave1_auto_rollback_probe.sh \
        --environment "$WAVE1_ROLLBACK_ENVIRONMENT" \
        --target k8s \
        --deployment "${WAVE1_K8S_DEPLOYMENT:-wms-api}" \
        --context "$WAVE1_K8S_CONTEXT" \
        --namespace "$WAVE1_K8S_NAMESPACE" \
        --evidence-file docs/retros/wave-1-runtime-evidence.json \
        --rollback-log-ref "$WAVE1_ROLLBACK_LOG_REF" \
        --external-log-ref "$WAVE1_EXTERNAL_LOG_REF"

# 采集 Wave 1 docker-compose 自动回滚 runtime 证据：SMOKE_URL 或 PROMETHEUS_URL + PROMETHEUS_QUERY 必须指向真实 dev/staging
wave-1-rollback-runtime-evidence-compose:
    @python3 scripts/governance/check_wave1_runtime_evidence_prereqs.py --mode rollback-compose && \
      deploy/scripts/wave1_auto_rollback_probe.sh \
        --environment "$WAVE1_ROLLBACK_ENVIRONMENT" \
        --target docker-compose \
        --previous-version "$WAVE1_PREVIOUS_VERSION" \
        --compose-file "$WAVE1_COMPOSE_FILE" \
        --evidence-file docs/retros/wave-1-runtime-evidence.json \
        --rollback-log-ref "$WAVE1_ROLLBACK_LOG_REF" \
        --external-log-ref "$WAVE1_EXTERNAL_LOG_REF"

# 列出所有治理脚本
gov-list:
    @ls -1 scripts/governance/*.py 2>/dev/null | grep -v "^_" || echo "(no scripts yet)"

# 检查各 Tier 实际耗时（写入 governance/baselines/tier-runtime.json）
tier-timing:
    @echo "▶ measuring tier runtimes (单位: ms)"
    @python3 scripts/governance/_tier_timing.py

# ============================================================
# Wave 启动检查（占位）
# ============================================================
# 进入新 Wave 前必须通过的前置条件检查
# ============================================================

# 进入 Wave 1 前置检查
wave-1-ready:
    @echo "▶ Wave 1 entry conditions"
    @echo "  ── 治理体系核心 ──"
    @python3 scripts/governance/validate_environment.py
    @python3 scripts/governance/validate_adr_index.py
    @python3 scripts/governance/check_doc_links.py
    @echo "  ── T1 全套 ──"
    @python3 scripts/governance/governance_checks.py --tier T1 > /dev/null && echo "  ✓ T1 24 项全部通过" || (echo "  ✘ T1 失败"; exit 1)
    @echo "  ── pytest 治理脚本测试 ──"
    @python3 -m pytest scripts/governance/tests/ -q 2>&1 | tail -3
    @echo "  ── Wave 1 应当新增的治理脚本 ──"
    @echo "    [ ] check_layer_dependency.py        — Rust 层级依赖（domain ⊥ infra）"
    @echo "    [ ] check_unsafe_and_unwrap.py       — Rust unsafe / 生产路径 unwrap 检查"
    @echo "    [ ] check_handler_test_coverage.py   — handler 测试覆盖（baseline）"
    @echo "  注：以上脚本必须在 Wave 1 第一周补齐；当前为占位（在 task_check.py --strict 下会失败）"
    @echo ""
    @echo "  ── Wave 1 启动后必做 ──"
    @echo "    [1] 初始化 baseline 快照（首次跑前手动执行 1 次）："
    @echo "        python3 scripts/governance/check_baseline_health.py --update-snapshot"
    @echo "    [2] 在 lefthook.yml pre-push 钩子中启用 --strict 模式："
    @echo "        替换 'task_check.py --tier T2' 为 'task_check.py --tier T2 --strict'"
    @echo "    [3] 在 CI workflow 中启用 --strict 模式（同上）"
    @echo ""
    @echo "✓ Wave 0 治理骨架完整 — 可进入 Wave 1"
