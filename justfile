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
# 当前已进入 Wave 6；四级入口必须执行真实检查并 fail closed。

set shell := ["bash", "-cu"]
set dotenv-load := true

# 仓库根目录的绝对路径（所有 worktree 内行为一致）
ROOT := justfile_directory()
MAIN_ROOT := "/home/test1/workspace/wms"
DEV_WEB_SESSION := "wms-web-admin-9002"
DEV_WEB_PORT := "9002"
DEV_WEB_CWD := MAIN_ROOT / "apps/web-admin"
DEV_WEB_LOG := "/tmp/wms-web-admin-9002.log"
ISSUE_AGENT_LOG := MAIN_ROOT / ".codex/issue-agent/watch.log"
ISSUE_AGENT_PID := MAIN_ROOT / ".codex/issue-agent/watch.pid"
ISSUE_AGENT_CRON_TAG := "wms-issue-agent-watchdog"
SESSION_CLOSEOUT_LOG := MAIN_ROOT / ".codex/session-closeout/watch.log"
SESSION_CLOSEOUT_PID := MAIN_ROOT / ".codex/session-closeout/watch.pid"
SESSION_CLOSEOUT_CRON_TAG := "wms-session-closeout-watchdog"

# 默认显示帮助
default:
    @just --list

# ============================================================
# Tier 1: quick-check（< 10 秒）
# 写代码时随手跑、pre-commit 自动触发
# 仅做：格式和快速治理，不跑测试
# ============================================================

# Tier 1 quick-check (< 10s): fmt + fast governance
quick-check: _t1-banner _t1-fmt _t1-governance

_t1-banner:
    @echo "▶ Tier 1 quick-check (target < 10s)"

_t1-fmt:
    @echo "  · Rust format check"
    @cargo fmt --manifest-path backend/Cargo.toml --all -- --check

_t1-governance:
    @echo "  · governance T1 checks"
    @python3 scripts/governance/governance_checks.py --tier T1

# ============================================================
# Tier 2: task-check（< 120 秒）
# 任务结束、commit 前
# T1 + diff 触发的最小治理集 + L1 单元测试 + L2 静态契约
# ============================================================

# Tier 2 task-check (< 120s): T1 + diff-driven + L1/L2
task-check: quick-check _t2-banner _t2-diff-checks _t2-lint _t2-unit-tests _t2-contract-static

_t2-banner:
    @echo "▶ Tier 2 task-check (target < 120s)"

_t2-diff-checks:
    @echo "  · diff-driven governance checks"
    @python3 scripts/governance/task_check.py --tier T2 --strict

_t2-lint:
    @echo "  · Rust clippy"
    @cargo clippy --manifest-path backend/Cargo.toml --workspace --all-targets --no-deps -- -D warnings -A clippy::too-many-arguments

_t2-unit-tests:
    @echo "  · L1 unit tests"
    @cargo test --manifest-path backend/Cargo.toml --workspace --lib
    @pnpm --dir apps/web-admin run test:self-checks
    @node packages/ui/tests/dialog-dismiss.test.ts
    @node packages/ui/tests/data-grid-views.test.ts
    @node packages/ui/tests/query-panel-quick-filters.test.ts
    @node apps/web-admin/src/pages/inbound/inbound-document-entry-model.test.ts

_t2-contract-static:
    @echo "  · L2 API contract static"
    @pnpm --dir apps/web-admin exec tsc --noEmit
    @pnpm --dir packages/api-client run typecheck
    @python3 scripts/governance/check_openapi_in_sync.py --strict
    @python3 scripts/governance/validate_openapi_artifacts.py
    @python3 scripts/governance/check_openapi_contract.py

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
    @echo "  · L3-L5/L8/L11 integration tests"
    @cargo test --manifest-path backend/Cargo.toml --workspace
    @python3 -m pytest scripts/governance/tests -q
    @pnpm --dir apps/web-admin run test:e2e:shell-dev
    @pnpm --dir apps/web-admin run test:e2e:h4-dev
    @pnpm --dir apps/web-admin run test:e2e:h9-dev

_t3-governance-l3:
    @echo "  · governance T3 checks"
    @python3 scripts/governance/task_check.py --tier T3 --strict
    @python3 scripts/governance/capture_visual_snapshots.py --port 15173 --start-server
    @python3 scripts/governance/check_visual_regression.py

# ============================================================
# Tier 4: verify（< 30 分钟）
# 合并前、CI、发版前
# T3 + L6 并发 + L7 性能 + L10 可观测 + 完整 E2E + 合规追溯
# ============================================================

# Tier 4 verify (< 30min): T3 + L6/L7/L10 + E2E
verify: preflight _t4-banner _t4-full-tests _t4-e2e _t4-perf-bench _t4-contract-check _t4-governance-l4

_t4-banner:
    @echo "▶ Tier 4 verify (target < 30min)"

_t4-full-tests:
    @echo "  · full test suite incl. release mode"
    @cargo test --manifest-path backend/Cargo.toml --workspace --release

_t4-e2e:
    @echo "  · Matrix E2E screenshots (full, [PROTOTYPE-C])"
    @just matrix-e2e-full

_t4-perf-bench:
    @echo "  · L7 / runtime evidence release gate"
    @python3 scripts/governance/validate_wave1_runtime_evidence.py --kind h2
    @just wave-6-complete-check

_t4-contract-check:
    @echo "  · OpenAPI contract consistency"
    @python3 scripts/governance/check_openapi_in_sync.py --strict
    @python3 scripts/governance/validate_openapi_artifacts.py
    @python3 scripts/governance/check_openapi_contract.py

_t4-governance-l4:
    @echo "  · active-module scope completeness"
    @python3 scripts/governance/check_runtime_route_mounts.py --strict
    @python3 scripts/governance/check_handler_test_coverage.py --strict
    @python3 scripts/governance/check_scope_gap_discovery.py --strict
    @python3 scripts/governance/check_bounded_contexts.py --strict
    @python3 scripts/governance/check_multi_end_consistency.py --strict
    @python3 scripts/governance/check_observability.py --strict

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

# Matrix E2E 截图烟测（默认只跑前 20 个 tab；可传 --tab 精确调试）
matrix-e2e-smoke *args:
    @python3 scripts/governance/check_e2e_matrix_completeness.py
    @python3 scripts/governance/run_matrix_e2e_screenshots.py --limit 20 {{args}}
    @python3 scripts/governance/check_matrix_e2e_report.py --allow-partial

# Matrix E2E 截图全量门禁（C 方案：204 tab，全量 DOM / 交互 / 截图证据）
matrix-e2e-full *args:
    @python3 scripts/governance/check_e2e_matrix_completeness.py
    @python3 scripts/governance/run_matrix_e2e_screenshots.py {{args}}
    @python3 scripts/governance/check_matrix_e2e_report.py

# M1 管理端真实后端数据 E2E；基于 DATABASE_URL / WMS_DB_URL 创建并回收一次性数据库
web-admin-m1-real-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "${DATABASE_URL:-}" && -z "${WMS_DB_URL:-}" && -f .env ]]; then
      set -a
      source .env
      set +a
    fi
    source_url="${DATABASE_URL:-${WMS_DB_URL:-}}"
    if [[ -z "$source_url" ]]; then
      echo "DATABASE_URL or WMS_DB_URL is required for M1 real-data E2E" >&2
      exit 2
    fi
    url_without_query="${source_url%%\?*}"
    query=""
    if [[ "$source_url" == *"?"* ]]; then query="?${source_url#*\?}"; fi
    base_url="${url_without_query%/*}"
    database_name="wms_m1_e2e_${RANDOM}_$$"
    admin_url="${base_url}/postgres${query}"
    test_url="${base_url}/${database_name}${query}"
    cleanup() {
      psql "$admin_url" -q -c "DROP DATABASE IF EXISTS \"${database_name}\" WITH (FORCE)"
    }
    trap cleanup EXIT
    psql "$admin_url" -v ON_ERROR_STOP=1 -q -c "CREATE DATABASE \"${database_name}\""
    DATABASE_URL="$test_url" pnpm --dir prototypes exec playwright test --config=playwright-web-admin-m1-real-config.ts

# M2 管理端真实后端完整入库链路 E2E；需要 DATABASE_URL 或 WMS_DB_URL 指向测试库
web-admin-m2-real-e2e:
    @pnpm --dir apps/web-admin run test:e2e:m2-real

# H8 本地联调：outbox → 容器 ERP（A）/ MSSQL 接口表（B）主备
h8-local-integration:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "${DATABASE_URL:-}" && -z "${WMS_DB_URL:-}" && -f .env ]]; then
      set -a; source .env; set +a
    fi
    export ERP_CALLBACK_BASE="${ERP_CALLBACK_BASE:-http://127.0.0.1:18092}"
    export WMS_DB_URL="${WMS_DB_URL:-${DATABASE_URL:-}}"
    cd deploy
    docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
    docker compose -f docker-compose.h8-erp-if.yml up -d
    bash h8-erp-if/wait-and-init.sh
    cd ..
    python3 scripts/h8_erp_interface_sync/run_local_integration.py

# 容器化外部 ERP 厂商 + S4 风格回执/主备证据
h8-container-erp-s4-evidence:
    #!/usr/bin/env bash
    set -euo pipefail
    cd deploy
    docker compose -f docker-compose.h8-erp-vendor.yml up -d --build
    export ERP_CALLBACK_BASE="${ERP_CALLBACK_BASE:-http://127.0.0.1:18092}"
    for i in $(seq 1 40); do
      if curl -fsS "$ERP_CALLBACK_BASE/healthz" >/dev/null 2>&1; then break; fi
      sleep 0.5
    done
    cd ..
    python3 scripts/h8_erp_interface_sync/run_container_erp_s4_evidence.py

# H8 ERP 连接真实后端 E2E；基于 DATABASE_URL / WMS_DB_URL 创建并回收一次性数据库
web-admin-h8-real-e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "${DATABASE_URL:-}" && -z "${WMS_DB_URL:-}" && -f .env ]]; then
      set -a
      source .env
      set +a
    fi
    source_url="${DATABASE_URL:-${WMS_DB_URL:-}}"
    if [[ -z "$source_url" ]]; then
      echo "DATABASE_URL or WMS_DB_URL is required for H8 real-data E2E" >&2
      exit 2
    fi
    url_without_query="${source_url%%\?*}"
    query=""
    if [[ "$source_url" == *"?"* ]]; then query="?${source_url#*\?}"; fi
    base_url="${url_without_query%/*}"
    database_name="wms_h8_e2e_${RANDOM}_$$"
    admin_url="${base_url}/postgres${query}"
    test_url="${base_url}/${database_name}${query}"
    cleanup() {
      psql "$admin_url" -q -c "DROP DATABASE IF EXISTS \"${database_name}\" WITH (FORCE)" || true
    }
    trap cleanup EXIT
    psql "$admin_url" -v ON_ERROR_STOP=1 -q -c "CREATE DATABASE \"${database_name}\""
    DATABASE_URL="$test_url" pnpm --dir apps/web-admin run test:e2e:h8-real

# 管理端 9002 当前占用状态
dev-web-status:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "tmux:"
    tmux ls 2>/dev/null | grep -E '^{{DEV_WEB_SESSION}}:' || true
    echo "process:"
    pgrep -af 'vite .*--port[[:space:]]+{{DEV_WEB_PORT}}|pnpm -C apps/web-admin dev .*--port[[:space:]]+{{DEV_WEB_PORT}}' || true
    echo "cwd:"
    for pid in $(pgrep -f 'vite .*--port[[:space:]]+{{DEV_WEB_PORT}}' || true); do
      printf '%s ' "$pid"
      readlink "/proc/$pid/cwd" || true
    done

# 从主工作区重启管理端 9002
dev-web-restart:
    #!/usr/bin/env bash
    set -euo pipefail
    tmux kill-session -t "{{DEV_WEB_SESSION}}" 2>/dev/null || true
    for pid in $(pgrep -f 'vite .*--port[[:space:]]+{{DEV_WEB_PORT}}' || true); do
      cwd=$(readlink "/proc/$pid/cwd" 2>/dev/null || true)
      if [[ "$cwd" == "{{DEV_WEB_CWD}}" || "$cwd" == *"/wms-agent-"* ]]; then
        kill "$pid" 2>/dev/null || true
      fi
    done
    if [[ "${WMS_WEB_ADMIN_DEV_MOCK:-1}" == "1" ]]; then
      dev_env='WMS_WEB_ADMIN_DEV_MOCK=1'
    else
      dev_env="WMS_WEB_ADMIN_DEV_MOCK=0 VITE_API_BASE_URL= WMS_WEB_ADMIN_E2E_API_URL=${WMS_WEB_ADMIN_E2E_API_URL:-http://192.168.124.10:18080}"
    fi
    tmux new-session -d -s "{{DEV_WEB_SESSION}}" -c "{{MAIN_ROOT}}" \
      "$dev_env pnpm -C apps/web-admin dev 2>&1 | tee {{DEV_WEB_LOG}}"
    sleep 2
    just dev-web-verify

# 校验 9002 来自主工作区，不是 agent worktree
dev-web-verify:
    #!/usr/bin/env bash
    set -euo pipefail
    ok=0
    for pid in $(pgrep -f 'vite .*--port[[:space:]]+{{DEV_WEB_PORT}}' || true); do
      cwd=$(readlink "/proc/$pid/cwd" 2>/dev/null || true)
      echo "$pid $cwd"
      if [[ "$cwd" == "{{DEV_WEB_CWD}}" ]]; then
        ok=1
      fi
      if [[ "$cwd" == *"/wms-agent-"* ]]; then
        echo "9002 被 agent worktree 占用：$cwd" >&2
        exit 1
      fi
    done
    if [[ "$ok" != 1 ]]; then
      echo "9002 未由主工作区 {{DEV_WEB_CWD}} 提供" >&2
      exit 1
    fi
    curl --noproxy '*' -fsS --max-time 3 "http://127.0.0.1:{{DEV_WEB_PORT}}/" >/dev/null
    lan_ip="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1;i<=NF;i++) if ($i=="src") {print $(i+1); exit}}')"
    if [[ -n "$lan_ip" && "$lan_ip" != 127.* ]]; then
      curl --noproxy '*' -fsS --max-time 3 "http://$lan_ip:{{DEV_WEB_PORT}}/" >/dev/null
      echo "LAN URL: http://$lan_ip:{{DEV_WEB_PORT}}/"
    fi

# 从指定 worktree 启动管理端预览；9002 保留给主工作区，worktree 默认 9003
dev-web-worktree-restart path port="9003":
    #!/usr/bin/env bash
    set -euo pipefail
    worktree="$(realpath "{{path}}")"
    port="{{port}}"
    if [[ "$port" == "{{DEV_WEB_PORT}}" || ! "$port" =~ ^9[0-9]{3}$ || "$port" -lt 9003 || "$port" -gt 9099 ]]; then
      echo "worktree 前端端口必须是 9003-9099，9002 保留给主工作区" >&2
      exit 1
    fi
    [[ -d "$worktree/apps/web-admin" ]] || { echo "不是 WMS worktree：$worktree" >&2; exit 1; }
    slug="$(basename "$worktree" | tr -c '[:alnum:]_-' '-')"
    session="wms-web-admin-${port}-${slug}"
    log="/tmp/${session}.log"
    tmux kill-session -t "$session" 2>/dev/null || true
    for pid in $(pgrep -f "vite .*--port[[:space:]]+$port" || true); do
      cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
      if [[ "$cwd" == "$worktree" || "$cwd" == "$worktree/apps/web-admin" ]]; then
        kill "$pid" 2>/dev/null || true
      else
        echo "端口 $port 已被其他进程占用：$pid $cwd" >&2
        exit 1
      fi
    done
    if [[ "${WMS_WEB_ADMIN_DEV_MOCK:-1}" == "1" ]]; then
      dev_env='WMS_WEB_ADMIN_DEV_MOCK=1'
    else
      dev_env="WMS_WEB_ADMIN_DEV_MOCK=0 VITE_API_BASE_URL= WMS_WEB_ADMIN_E2E_API_URL=${WMS_WEB_ADMIN_E2E_API_URL:-http://192.168.124.10:18080}"
    fi
    tmux new-session -d -s "$session" -c "$worktree" \
      "$dev_env pnpm -C apps/web-admin exec vite --host 0.0.0.0 --port $port --strictPort 2>&1 | tee $log"
    sleep 2
    just dev-web-worktree-verify "$worktree" "$port"

# 校验指定 worktree 的管理端预览端口和 LAN 地址
dev-web-worktree-verify path port="9003":
    #!/usr/bin/env bash
    set -euo pipefail
    worktree="$(realpath "{{path}}")"
    port="{{port}}"
    if [[ "$port" == "{{DEV_WEB_PORT}}" || ! "$port" =~ ^9[0-9]{3}$ || "$port" -lt 9003 || "$port" -gt 9099 ]]; then
      echo "worktree 前端端口必须是 9003-9099，9002 保留给主工作区" >&2
      exit 1
    fi
    ok=0
    for pid in $(pgrep -f "vite .*--port[[:space:]]+$port" || true); do
      cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
      echo "$pid $cwd"
      if [[ "$cwd" == "$worktree" || "$cwd" == "$worktree/apps/web-admin" ]]; then
        ok=1
      fi
    done
    if [[ "$ok" != 1 ]]; then
      echo "$port 未由 worktree $worktree 提供" >&2
      exit 1
    fi
    lan_ip="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1;i<=NF;i++) if ($i=="src") {print $(i+1); exit}}')"
    curl --noproxy '*' -fsS --max-time 3 "http://127.0.0.1:$port/" >/dev/null
    if [[ -n "$lan_ip" && "$lan_ip" != 127.* ]]; then
      curl --noproxy '*' -fsS --max-time 3 "http://$lan_ip:$port/" >/dev/null
      echo "LAN URL: http://$lan_ip:$port/"
    fi

# 从指定 worktree 启动后端 API；18080 保留给主工作区，worktree 默认 18081
dev-api-worktree-restart path port="18081":
    #!/usr/bin/env bash
    set -euo pipefail
    worktree="$(realpath "{{path}}")"
    port="{{port}}"
    if [[ "$port" == "18080" || ! "$port" =~ ^18[0-9]{3}$ || "$port" -lt 18081 || "$port" -gt 18099 ]]; then
      echo "worktree 后端端口必须是 18081-18099，18080 保留给主工作区" >&2
      exit 1
    fi
    [[ -d "$worktree/backend" ]] || { echo "不是 WMS worktree：$worktree" >&2; exit 1; }
    slug="$(basename "$worktree" | tr -c '[:alnum:]_-' '-')"
    session="wms-api-${port}-${slug}"
    log="/tmp/${session}.log"
    tmux kill-session -t "$session" 2>/dev/null || true
    mapfile -t pids < <(ss -ltnp "sport = :$port" 2>/dev/null | sed -n 's/.*pid=\([0-9]\+\).*/\1/p' | sort -u)
    for pid in "${pids[@]}"; do
      cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
      if [[ "$cwd" == "$worktree" || "$cwd" == "$worktree/backend" ]]; then
        kill "$pid" 2>/dev/null || true
      else
        echo "端口 $port 已被其他进程占用：$pid $cwd" >&2
        exit 1
      fi
    done
    tmux new-session -d -s "$session" -c "$worktree" \
      "WMS_BIND_ADDR=0.0.0.0:$port WMS_JWT_SECRET=\${WMS_JWT_SECRET:-dev-jwt-secret-change-me} WMS_REDIS_URL=\${WMS_REDIS_URL:-redis://127.0.0.1:6379} cargo run --manifest-path backend/Cargo.toml -p wms-api --bin wms-api 2>&1 | tee $log"
    just dev-api-worktree-verify "$worktree" "$port"

# 校验指定 worktree 的后端 API 端口、/healthz 和 LAN 地址
dev-api-worktree-verify path port="18081":
    #!/usr/bin/env bash
    set -euo pipefail
    worktree="$(realpath "{{path}}")"
    port="{{port}}"
    if [[ "$port" == "18080" || ! "$port" =~ ^18[0-9]{3}$ || "$port" -lt 18081 || "$port" -gt 18099 ]]; then
      echo "worktree 后端端口必须是 18081-18099，18080 保留给主工作区" >&2
      exit 1
    fi
    for _ in {1..30}; do
      if curl --noproxy '*' -fsS --max-time 2 "http://127.0.0.1:$port/healthz" >/dev/null; then
        break
      fi
      sleep 1
    done
    curl --noproxy '*' -fsS --max-time 3 "http://127.0.0.1:$port/healthz" >/dev/null
    ok=0
    mapfile -t pids < <(ss -ltnp "sport = :$port" 2>/dev/null | sed -n 's/.*pid=\([0-9]\+\).*/\1/p' | sort -u)
    for pid in "${pids[@]}"; do
      cwd="$(readlink "/proc/$pid/cwd" 2>/dev/null || true)"
      echo "$pid $cwd"
      if [[ "$cwd" == "$worktree" || "$cwd" == "$worktree/backend" ]]; then
        ok=1
      fi
    done
    if [[ "$ok" != 1 ]]; then
      echo "$port 未由 worktree $worktree 提供" >&2
      exit 1
    fi
    lan_ip="$(ip -4 route get 1.1.1.1 2>/dev/null | awk '{for (i=1;i<=NF;i++) if ($i=="src") {print $(i+1); exit}}')"
    if [[ -n "$lan_ip" && "$lan_ip" != 127.* ]]; then
      curl --noproxy '*' -fsS --max-time 3 "http://$lan_ip:$port/healthz" >/dev/null
      echo "LAN URL: http://$lan_ip:$port/"
    fi

# 生成主仓 OpenAPI JSON 并刷新 @wms/api-client 类型
openapi-sync:
    @cd backend && cargo run --quiet --bin openapi-export > ../shared/openapi/openapi.json
    @test -s shared/openapi/openapi.json
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

# Gitea issue agent：执行一轮扫描；默认 dry-run，传 --apply 才评论或运行 codex exec
issue-agent-once *args:
    @python3 -u scripts/agents/issue_runner.py once {{args}}

# Gitea issue agent：循环扫描；默认 dry-run，传 --apply 才评论或运行 codex exec
issue-agent-watch *args:
    @python3 -u scripts/agents/issue_runner.py watch {{args}}

# Gitea issue agent：扫描 closed issue 的本地分支合并队列；默认 dry-run，传 --apply 才本地合并
issue-agent-local-merge *args:
    @python3 -u scripts/agents/issue_runner.py local-merge-closed {{args}}

# Gitea issue agent：验证后台环境能否真正启动 codex exec
issue-agent-codex-smoke *args:
    @python3 -u scripts/agents/issue_runner.py codex-smoke {{args}}

# Gitea issue agent：查看长期 watcher 状态
issue-agent-status:
    #!/usr/bin/env bash
    set -euo pipefail
    pid_file="{{ISSUE_AGENT_PID}}"
    echo "pid:"
    if [[ -f "$pid_file" ]]; then
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      echo "${pid:-empty}"
      if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        ps -fp "$pid" || true
      else
        echo "not-running"
      fi
    else
      echo "no pid file"
    fi
    echo "process:"
    pgrep -af 'scripts/agents/issue_runner.py watch' || true
    echo "watchdog:"
    crontab -l 2>/dev/null | grep -F "{{ISSUE_AGENT_CRON_TAG}}" || true
    echo "recent-log:"
    tail -20 "{{ISSUE_AGENT_LOG}}" 2>/dev/null || true

# Gitea issue agent：校验长期 watcher 真的在跑
issue-agent-verify:
    #!/usr/bin/env bash
    set -euo pipefail
    pid_file="{{ISSUE_AGENT_PID}}"
    [[ -f "$pid_file" ]] || { echo "issue-agent pid 文件不存在：$pid_file" >&2; exit 1; }
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    [[ "$pid" =~ ^[0-9]+$ ]] || { echo "issue-agent pid 无效：${pid:-empty}" >&2; exit 1; }
    kill -0 "$pid" 2>/dev/null || { echo "issue-agent 进程不存在：$pid" >&2; exit 1; }
    cmd="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    grep -F 'scripts/agents/issue_runner.py watch' <<<"$cmd" >/dev/null || {
      echo "issue-agent pid 不是 watcher：$cmd" >&2
      exit 1
    }

# Gitea issue agent：重启长期 watcher
issue-agent-restart:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$(dirname "{{ISSUE_AGENT_LOG}}")"
    just_bin="${JUST_BIN:-$(command -v just)}"
    pid_file="{{ISSUE_AGENT_PID}}"
    stop_pid() {
      local pid="$1"
      [[ "$pid" =~ ^[0-9]+$ ]] || return 0
      [[ "$pid" != "$$" && "$pid" != "$BASHPID" ]] || return 0
      kill -0 "$pid" 2>/dev/null || return 0
      kill "$pid" 2>/dev/null || true
      for _ in {1..20}; do
        kill -0 "$pid" 2>/dev/null || return 0
        sleep 0.2
      done
      kill -KILL "$pid" 2>/dev/null || true
    }
    if [[ -f "$pid_file" ]]; then
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      stop_pid "$pid"
    fi
    mapfile -t legacy_pids < <(ps -eo pid=,args= | awk '/[p]ython3 -u scripts\/agents\/issue_runner\.py watch/ {print $1}')
    for pid in "${legacy_pids[@]}"; do
      stop_pid "$pid"
    done
    rm -f "$pid_file"
    cd "{{MAIN_ROOT}}"
    nohup setsid python3 -u scripts/agents/issue_runner.py watch --interval "${WMS_ISSUE_AGENT_INTERVAL:-60}" --apply --local-merge-closed >> "{{ISSUE_AGENT_LOG}}" 2>&1 < /dev/null &
    echo "$!" > "$pid_file"
    sleep 2
    "$just_bin" issue-agent-verify

# Gitea issue agent：保活检查；进程不在则重启
issue-agent-ensure:
    #!/usr/bin/env bash
    set -euo pipefail
    just_bin="${JUST_BIN:-$(command -v just)}"
    if "$just_bin" issue-agent-verify >/dev/null 2>&1; then
      echo "issue-agent running"
      exit 0
    fi
    echo "issue-agent not running; restarting"
    "$just_bin" issue-agent-restart

# Gitea issue agent：安装 cron watchdog，每分钟执行一次保活检查
issue-agent-install-watchdog:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$(dirname "{{ISSUE_AGENT_LOG}}")"
    just_bin="$(command -v just)"
    path_value="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    line="* * * * * cd {{MAIN_ROOT}} && PATH=$path_value JUST_BIN=$just_bin $just_bin issue-agent-ensure >> {{ISSUE_AGENT_LOG}} 2>&1 # {{ISSUE_AGENT_CRON_TAG}}"
    (crontab -l 2>/dev/null | grep -v -F "{{ISSUE_AGENT_CRON_TAG}}" || true; echo "$line") | crontab -
    "$just_bin" issue-agent-ensure

# Gitea issue agent：卸载 cron watchdog；不停止当前 watcher
issue-agent-uninstall-watchdog:
    #!/usr/bin/env bash
    set -euo pipefail
    { crontab -l 2>/dev/null || true; } | { grep -v -F "{{ISSUE_AGENT_CRON_TAG}}" || true; } | crontab -

# 会话收口：执行一轮空闲检查；默认 dry-run，传 --apply 才运行 codex exec
session-closeout-once *args:
    @python3 -u scripts/agents/session_closeout_runner.py once {{args}}

# 会话收口：循环空闲检查；默认 dry-run，传 --apply 才运行 codex exec
session-closeout-watch *args:
    @python3 -u scripts/agents/session_closeout_runner.py watch {{args}}

# 会话收口：查看最近触发状态和 watcher 状态
session-closeout-status:
    #!/usr/bin/env bash
    set -euo pipefail
    pid_file="{{SESSION_CLOSEOUT_PID}}"
    echo "state:"
    python3 -u scripts/agents/session_closeout_runner.py status || true
    echo "pid:"
    if [[ -f "$pid_file" ]]; then
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      echo "${pid:-empty}"
      if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        ps -fp "$pid" || true
      else
        echo "not-running"
      fi
    else
      echo "no pid file"
    fi
    echo "process:"
    pgrep -af 'scripts/agents/session_closeout_runner.py watch' || true
    echo "watchdog:"
    crontab -l 2>/dev/null | grep -F "{{SESSION_CLOSEOUT_CRON_TAG}}" || true
    echo "recent-log:"
    tail -20 "{{SESSION_CLOSEOUT_LOG}}" 2>/dev/null || true

# 会话收口：校验长期 watcher 真的在跑
session-closeout-verify:
    #!/usr/bin/env bash
    set -euo pipefail
    pid_file="{{SESSION_CLOSEOUT_PID}}"
    [[ -f "$pid_file" ]] || { echo "session-closeout pid 文件不存在：$pid_file" >&2; exit 1; }
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    [[ "$pid" =~ ^[0-9]+$ ]] || { echo "session-closeout pid 无效：${pid:-empty}" >&2; exit 1; }
    kill -0 "$pid" 2>/dev/null || { echo "session-closeout 进程不存在：$pid" >&2; exit 1; }
    cmd="$(ps -p "$pid" -o args= 2>/dev/null || true)"
    grep -F 'scripts/agents/session_closeout_runner.py watch' <<<"$cmd" >/dev/null || {
      echo "session-closeout pid 不是 watcher：$cmd" >&2
      exit 1
    }

# 会话收口：重启长期 watcher
session-closeout-restart:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$(dirname "{{SESSION_CLOSEOUT_LOG}}")"
    just_bin="${JUST_BIN:-$(command -v just)}"
    pid_file="{{SESSION_CLOSEOUT_PID}}"
    stop_pid() {
      local pid="$1"
      [[ "$pid" =~ ^[0-9]+$ ]] || return 0
      [[ "$pid" != "$$" && "$pid" != "$BASHPID" ]] || return 0
      kill -0 "$pid" 2>/dev/null || return 0
      kill "$pid" 2>/dev/null || true
      for _ in {1..20}; do
        kill -0 "$pid" 2>/dev/null || return 0
        sleep 0.2
      done
      kill -KILL "$pid" 2>/dev/null || true
    }
    if [[ -f "$pid_file" ]]; then
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      stop_pid "$pid"
    fi
    mapfile -t legacy_pids < <(ps -eo pid=,args= | awk '/[p]ython3 -u scripts\/agents\/session_closeout_runner\.py watch/ {print $1}')
    for pid in "${legacy_pids[@]}"; do
      stop_pid "$pid"
    done
    rm -f "$pid_file"
    cd "{{MAIN_ROOT}}"
    nohup setsid python3 -u scripts/agents/session_closeout_runner.py watch --interval "${WMS_SESSION_CLOSEOUT_INTERVAL:-60}" --idle-seconds "${WMS_SESSION_CLOSEOUT_IDLE_SECONDS:-1800}" --apply >> "{{SESSION_CLOSEOUT_LOG}}" 2>&1 < /dev/null &
    echo "$!" > "$pid_file"
    sleep 2
    "$just_bin" session-closeout-verify

# 会话收口：保活检查；进程不在则重启
session-closeout-ensure:
    #!/usr/bin/env bash
    set -euo pipefail
    just_bin="${JUST_BIN:-$(command -v just)}"
    if "$just_bin" session-closeout-verify >/dev/null 2>&1; then
      echo "session-closeout running"
      exit 0
    fi
    echo "session-closeout not running; restarting"
    "$just_bin" session-closeout-restart

# 会话收口：安装 cron watchdog，每分钟保活检查
session-closeout-install-watchdog:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$(dirname "{{SESSION_CLOSEOUT_LOG}}")"
    just_bin="$(command -v just)"
    path_value="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    line="* * * * * cd {{MAIN_ROOT}} && PATH=$path_value JUST_BIN=$just_bin $just_bin session-closeout-ensure >> {{SESSION_CLOSEOUT_LOG}} 2>&1 # {{SESSION_CLOSEOUT_CRON_TAG}}"
    (crontab -l 2>/dev/null | grep -v -F "{{SESSION_CLOSEOUT_CRON_TAG}}" || true; echo "$line") | crontab -
    "$just_bin" session-closeout-ensure

# 会话收口：卸载 cron watchdog；不停止当前 watcher
session-closeout-uninstall-watchdog:
    #!/usr/bin/env bash
    set -euo pipefail
    { crontab -l 2>/dev/null || true; } | { grep -v -F "{{SESSION_CLOSEOUT_CRON_TAG}}" || true; } | crontab -

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

# 报告 Wave 3 完成度（默认不阻塞；出口检查用 --strict）
wave-3-status:
    @python3 scripts/governance/report_wave3_completion.py

# Wave 3 开发完成出口检查
wave-3-complete-check:
    @python3 scripts/governance/report_wave3_completion.py --strict

# Wave 3 真 PDA + L7 runtime evidence 预发布验证
wave-3-pda-runtime-evidence-validate:
    @python3 scripts/governance/validate_wave3_pda_runtime_evidence.py

# Wave 3 真 PDA + L7 runtime evidence readiness；不写 evidence
wave-3-pda-runtime-readiness *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py {{args}}

# Wave 3 无 PDA 阶段服务前置检查；只探测 dev/staging health 与 Wave3 鉴权边界，不写 evidence
wave-3-pda-service-precheck *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --service-precheck-only {{args}}

# Wave 3 无 PDA 阶段追溯码 OpenAPI 前置检查；只读验证合约和 X-API-Key，不写 evidence
wave-3-pda-trace-code-openapi-precheck *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --trace-code-openapi-precheck {{args}}

# Wave 3 无 PDA 阶段现场材料清单；只输出字段分工，不探测服务、不写 evidence
wave-3-pda-materials-checklist *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --materials-checklist {{args}}

# Wave 3 无 PDA 阶段预审包；汇总可推进项、真机阻塞项和禁止事项，不探测服务、不写 evidence
wave-3-pda-preaudit-kit *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --preaudit-kit {{args}}

# Wave 3 现场资源申请包；只输出可转发 Markdown/JSON，不探测服务、不写 evidence
wave-3-pda-field-work-request *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --field-work-request {{args}}

# Wave 3 现场执行摘要；只汇总当前变量缺口和下一步命令，不探测服务、不写 evidence
wave-3-pda-field-execution-summary *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --field-execution-summary {{args}}

# Wave 3 现场前置一键预检；组合服务、追溯码 OpenAPI 和字段摘要，只读不写 evidence
wave-3-pda-field-precheck-summary *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --field-precheck-summary {{args}}

# Wave 3 现场 owner 缺口动作单；按负责人聚合缺口，只读不写 evidence
wave-3-pda-field-owner-gap-actions *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --field-owner-gap-actions {{args}}

# Wave 3 现场交接总包；聚合预审、材料、owner 缺口、证据包模板和可选 from-env/attachment 预检，可用 --field-handoff-output 归档，只读不写 evidence
wave-3-pda-field-handoff-bundle *args:
    @python3 scripts/governance/check_wave3_pda_runtime_readiness.py --field-handoff-bundle {{args}}

# Wave 3 现场证据包 Markdown/JSON 模板；只输出模板，不写 runtime evidence
wave-3-pda-evidence-package-template *args:
    @python3 scripts/governance/record_wave3_pda_runtime_evidence.py --export-package-template {{args}}

# Wave 3 现场 JSON intake 模板；可输出或用 --intake-template-output 落盘，不写 runtime evidence
wave-3-pda-intake-template *args:
    @python3 scripts/governance/record_wave3_pda_runtime_evidence.py --export-intake-template {{args}}

# Wave 3 现场 JSON intake 只读校验；从 WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE 读取路径，不写 runtime evidence
wave-3-pda-intake-check *args:
    @test -n "$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" || (echo "WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE is required" >&2; exit 2)
    @python3 scripts/governance/record_wave3_pda_runtime_evidence.py --from-intake-file "$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" --check-only {{args}}

# Wave 3 现场 JSON intake 正式记录；从同一份 intake 文件读取真实材料，写 runtime evidence
wave-3-pda-intake-record *args:
    @test -n "$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" || (echo "WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE is required" >&2; exit 2)
    @python3 scripts/governance/record_wave3_pda_runtime_evidence.py --from-intake-file "$WAVE_3_PDA_EVIDENCE_PACKAGE_TEMPLATE_FROM_INTAKE_FILE" {{args}}

# 记录 Wave 3 真 PDA + L7 runtime evidence；参数透传给 record_wave3_pda_runtime_evidence.py
wave-3-pda-runtime-evidence-record *args:
    @python3 scripts/governance/record_wave3_pda_runtime_evidence.py {{args}}

# 报告 Wave 4 完成度（默认不阻塞；出口检查用 --strict）
wave-4-status:
    @python3 scripts/governance/report_wave4_completion.py

# Wave 4 开发完成出口检查
wave-4-complete-check:
    @python3 scripts/governance/report_wave4_completion.py --strict

# Wave 4 外部依赖真实证据校验（当前主要覆盖 M-TC 码上放心）
wave-4-external-dependencies-validate:
    @python3 scripts/governance/validate_wave4_external_dependencies.py

# Wave 4 外部依赖 readiness；只读检查真实材料引用，不写 evidence
wave-4-external-dependencies-readiness *args:
    @python3 scripts/governance/check_wave4_external_dependencies_readiness.py {{args}}

# 记录 Wave 4 外部依赖真实证据；参数透传给 record_wave4_external_dependencies.py
wave-4-external-dependencies-record *args:
    @python3 scripts/governance/record_wave4_external_dependencies.py {{args}}

# Wave 4 完成后通知：未通过 wave-4-complete-check 时不会发送 webhook
wave-4-notify-if-complete:
    @python3 scripts/governance/notify_wave4_completion.py

# Wave 4 最终关闭流程：完成门禁 + 成功后通知
# 注：W4.D 码上放心真实外部 evidence 已按 clarifications #50 延期，
#     后续仍需单独运行 wave-4-external-dependencies-validate 关闭外部证据。
wave-4-closeout:
    @just wave-4-complete-check
    @just wave-4-notify-if-complete

# 报告 Wave 5 完成度（默认不阻塞；出口检查用 --strict）
wave-5-status:
    @python3 scripts/governance/report_wave5_completion.py

# Wave 5 开发完成出口检查
wave-5-complete-check:
    @python3 scripts/governance/report_wave5_completion.py --strict

# Wave 5 M-PK 真实硬件 evidence 预发布验证
wave-5-hardware-evidence-validate:
    @python3 scripts/governance/validate_wave5_hardware_evidence.py

# Wave 5 M-PK 真实硬件 evidence 材料预检；只校验字段和引用边界，不写 evidence
wave-5-hardware-materials *args:
    @python3 scripts/governance/record_wave5_hardware_evidence.py --check-only {{args}}

# Wave 5 M-PK 真实硬件 evidence readiness；只读预检，不连接硬件，不写 evidence
wave-5-hardware-readiness *args:
    @python3 scripts/governance/record_wave5_hardware_evidence.py --check-only {{args}}

# 记录 Wave 5 M-PK 真实硬件 evidence；参数透传给 record_wave5_hardware_evidence.py
wave-5-hardware-evidence-record *args:
    @python3 scripts/governance/record_wave5_hardware_evidence.py {{args}}

# Wave 5 M10 TMS+ 真实 dev/staging evidence 预发布验证
wave-5-tms-evidence-validate:
    @python3 scripts/governance/validate_wave5_tms_evidence.py

# Wave 5 M10 TMS+ 材料检查；只读检查真实 dev/staging refs，不写 evidence
wave-5-tms-materials *args:
    @python3 scripts/governance/record_wave5_tms_evidence.py --check-only {{args}}

# Wave 5 M10 TMS+ readiness；只读检查真实 dev/staging refs，不写 evidence
wave-5-tms-readiness *args:
    @python3 scripts/governance/record_wave5_tms_evidence.py --check-only {{args}}

# 记录 Wave 5 M10 TMS+ evidence；参数透传给 record_wave5_tms_evidence.py
wave-5-tms-evidence-record *args:
    @python3 scripts/governance/record_wave5_tms_evidence.py {{args}}

# 报告 Wave 6 预发布证据收口状态（默认不阻塞；出口检查用 --strict）
wave-6-status:
    @python3 scripts/governance/report_wave6_pre_release.py

# Wave 6 预发布证据收口出口检查
wave-6-complete-check:
    @python3 scripts/governance/report_wave6_pre_release.py --strict

# Wave 6 写 retro 前检查：8 个真实 evidence gate 必须全过，但暂不要求 wave-6-retro.md
wave-6-evidence-check:
    @python3 scripts/governance/report_wave6_pre_release.py --strict --evidence-only

# Wave 6 缺失 evidence gate 的人工采集命令清单（只读，不写 evidence）
wave-6-missing-evidence-commands:
    @python3 scripts/governance/report_wave6_pre_release.py --commands-only --strict --evidence-only

# Wave 6 evidence preflight：只检查 runbook / just 入口 / validator 链路，不写真实 evidence
wave-6-evidence-preflight:
    @python3 scripts/governance/check_wave6_evidence_preflight.py

# Wave 6 灰度发布 evidence 预发布验证
wave-6-deploy-evidence-validate:
    @python3 scripts/governance/validate_wave6_deploy_evidence.py

# Wave 6 灰度发布 readiness：只读检查 staging / payload 前置条件，不写 evidence
wave-6-deploy-readiness *args:
    @python3 scripts/governance/check_wave6_deploy_readiness.py {{args}}

# Wave 6 灰度发布材料 worksheet：只读检查外部 ref 环境变量，不写 evidence
wave-6-deploy-materials *args:
    @python3 scripts/governance/report_wave6_deploy_materials.py {{args}}

# Wave 6 灰度发布审计写入：正式写 audit_event，输出 audit_event_query_ref
wave-6-deploy-audit *args:
    @cargo run --manifest-path backend/Cargo.toml -p wms-api --bin wms-deploy-audit -- {{args}}

# 记录 Wave 6 灰度发布 evidence；参数透传给 record_wave6_deploy_evidence.py
wave-6-deploy-evidence-record *args:
    @python3 scripts/governance/record_wave6_deploy_evidence.py {{args}}

# Wave 2 配置中心 Feature Flag runtime evidence 预发布验证
wave-2-runtime-evidence-validate:
    @python3 scripts/governance/report_wave2_completion.py --strict --require-runtime-evidence

# 记录 Wave 2 配置中心 Feature Flag runtime evidence；参数透传给 record_wave2_runtime_evidence.py
wave-2-runtime-evidence-record *args:
    @python3 scripts/governance/record_wave2_runtime_evidence.py {{args}}

# Wave 2 配置中心 Feature Flag runtime evidence readiness；不写 evidence
wave-2-runtime-evidence-readiness *args:
    @python3 scripts/governance/collect_wave2_runtime_evidence.py --check-only {{args}}

# 执行 Wave 2 配置中心 Feature Flag 真实 smoke 并记录 runtime evidence
wave-2-runtime-evidence-smoke *args:
    @python3 scripts/governance/collect_wave2_runtime_evidence.py {{args}}

# 生成 Wave 2 staging H1 token；输出 export WAVE_2_H1_TOKEN=...
wave-2-h1-token *args:
    @python3 scripts/governance/generate_wave2_h1_token.py {{args}}

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

# Wave 1 H2 本机 dev-h2 dry-run 状态报告：只读，不写 evidence，不能关闭 W6.A gate
wave-1-h2-runtime-readiness-dry-run:
    @python3 scripts/governance/check_wave1_h2_runtime_readiness.py \
      --database-url "$WAVE1_H2_DATABASE_URL" \
      --dry-run-alias-ok \
      --json

# Wave 1 H2 dev 基线材料状态：只读查看行数、分布、容量和 loader 进程，不写 runtime evidence
wave-1-h2-baseline-status-container:
    @sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -c "select count(*) as audit_event_rows from audit_event; select count(*) as audit_chain_seal_rows from audit_chain_seal; select occurred_at::date as day, count(*) as rows from audit_event group by 1 order by 1; with audit_event_relations as (select 'audit_event'::regclass as relid union select inhrelid from pg_inherits where inhparent = 'audit_event'::regclass) select pg_size_pretty(pg_database_size(current_database())) as db_size, pg_size_pretty(sum(pg_total_relation_size(relid))) as audit_event_total_size from audit_event_relations;"
    @sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 df -h /var/lib/postgresql/data
    @ps -eo pid,ppid,stat,pcpu,pmem,etime,cmd | rg '[w]ms-audit-baseline-load|[d]ocker run --rm --pull=never --network wms-dev-h2_default' || true

# Wave 1 H2 dev 60M 基线材料 preflight：只读检查封档/混入/并发加载，然后 dry-run，不写 runtime evidence
wave-1-h2-baseline-preflight-60m-container:
    @SEALED_COUNT="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from audit_chain_seal where seal_date >= current_date - 7 and seal_date < current_date")"; if [ "$SEALED_COUNT" != "0" ]; then echo "target date range already contains audit_chain_seal rows: $SEALED_COUNT" >&2; exit 2; fi
    @MIXED_COUNT="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from audit_event where occurred_at >= current_date - 7 and occurred_at < current_date and (actor_name <> 'wave1-h2-baseline-loader' or action <> 'baseline.synthetic_event.prepared')")"; if [ "$MIXED_COUNT" != "0" ]; then echo "target date range contains non-baseline audit_event rows: $MIXED_COUNT" >&2; exit 2; fi
    @if ps -eo cmd | rg -q '[w]ms-audit-baseline-load|[d]ocker run --rm --pull=never --network wms-dev-h2_default'; then echo "baseline loader already running; refusing preflight" >&2; exit 2; fi
    @just wave-1-h2-baseline-plan-60m-container

# Wave 1 H2 dev 60M 基线材料规划：固定参数 dry-run，不写 runtime evidence
wave-1-h2-baseline-plan-60m-container:
    @RUN_ID="PLAN60M-$(date -u +%Y%m%dT%H%M%SZ)"; just wave-1-h2-baseline-dry-run-container \
      --target-total-rows 60000000 \
      --start-date "$(date -u -d '7 days ago' +%F)" \
      --days 7 \
      --batch-size 4000 \
      --run-id "$RUN_ID" \
      --summary-output "artifacts/dev/wave1/h2/baseline-loader-$RUN_ID.json"

# Wave 1 H2 dev 60M 基线材料加载：固定参数真实写 dev-h2，不写 runtime evidence
wave-1-h2-baseline-load-60m-container:
    @RUN_ID="BASELINE60M-$(date -u +%Y%m%dT%H%M%SZ)"; just wave-1-h2-baseline-load-container \
      --target-total-rows 60000000 \
      --start-date "$(date -u -d '7 days ago' +%F)" \
      --days 7 \
      --batch-size 4000 \
      --run-id "$RUN_ID" \
      --summary-output "artifacts/dev/wave1/h2/baseline-loader-$RUN_ID.json" \
      --execute \
      --i-understand-this-is-not-evidence

# Wave 1 H2 dev 基线材料 dry-run：只规划 audit_event 补数，不写 runtime evidence
wave-1-h2-baseline-dry-run *args:
    @cargo run --manifest-path backend/Cargo.toml -p wms-api --bin wms-audit-baseline-load -- {{args}}

# Wave 1 H2 dev 基线材料容器网络 dry-run：复用 dev-h2 compose 网络，不写 runtime evidence
wave-1-h2-baseline-dry-run-container *args:
    @case " {{args}} " in *" --execute "*) echo "dry-run-container refuses --execute; use wave-1-h2-baseline-load-container" >&2; exit 2;; esac
    @mkdir -p artifacts/dev/wave1/h2
    @cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin wms-audit-baseline-load
    @sudo -n docker run --rm --pull=never --network wms-dev-h2_default \
      --env-file deploy/env/dev-h2.env \
      --workdir /tmp \
      --user "$(id -u):$(id -g)" \
      -v "$PWD/artifacts/dev/wave1/h2:/tmp/artifacts/dev/wave1/h2" \
      -v "$PWD/backend/target/release/wms-audit-baseline-load:/tmp/wms-audit-baseline-load:ro" \
      --entrypoint /bin/sh \
      wms-api-dev-h2:${WMS_VERSION:-latest} \
      -c 'export WMS_DB_URL="postgres://wms_dev_h2:${WMS_DEV_H2_DB_PASSWORD}@postgres-dev-h2:5432/wms_dev_h2"; exec /tmp/wms-audit-baseline-load {{args}}'

# Wave 1 H2 dev 基线材料容器网络加载：需要显式 --execute 和 --i-understand-this-is-not-evidence，不写 runtime evidence
wave-1-h2-baseline-load-container *args:
    @mkdir -p artifacts/dev/wave1/h2
    @cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin wms-audit-baseline-load
    @sudo -n docker run --rm --pull=never --network wms-dev-h2_default \
      --env-file deploy/env/dev-h2.env \
      --workdir /tmp \
      --user "$(id -u):$(id -g)" \
      -v "$PWD/artifacts/dev/wave1/h2:/tmp/artifacts/dev/wave1/h2" \
      -v "$PWD/backend/target/release/wms-audit-baseline-load:/tmp/wms-audit-baseline-load:ro" \
      -e WMS_DEV_DB_HOST_ALLOWLIST=postgres-dev-h2 \
      --entrypoint /bin/sh \
      wms-api-dev-h2:${WMS_VERSION:-latest} \
      -c 'export WMS_DB_URL="postgres://wms_dev_h2:${WMS_DEV_H2_DB_PASSWORD}@postgres-dev-h2:5432/wms_dev_h2"; exec /tmp/wms-audit-baseline-load {{args}}'

# Wave 1 H2 dev 基线材料加载：需要显式 --execute 和 --i-understand-this-is-not-evidence，不写 runtime evidence
wave-1-h2-baseline-load *args:
    @cargo run --manifest-path backend/Cargo.toml -p wms-api --bin wms-audit-baseline-load -- {{args}}

# Wave 1 H2 dev 7 天封档状态：只读查看 audit_chain_seal，不写 runtime evidence
wave-1-h2-seal-status-container:
    @sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -c "select seal_date, last_id, sealed_at from audit_chain_seal where seal_date >= current_date - 7 and seal_date < current_date order by seal_date; select count(*) as recent_seal_days from audit_chain_seal where seal_date >= current_date - 7 and seal_date < current_date;"

# Wave 1 H2 dev 7 天封档 dry-run：构建维护 binary 并展示目标日期，不写 audit_chain_seal / runtime evidence
wave-1-h2-seal-dry-run-7d-container:
    @cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin audit-maintenance
    @echo "writes_audit_chain_seal=false"
    @echo "writes_runtime_evidence=false"
    @echo "target seal dates:"
    @for offset in 7 6 5 4 3 2 1; do date -u -d "$offset days ago" +%F; done
    @just wave-1-h2-seal-status-container

# Wave 1 H2 dev 7 天封档 preflight：只读检查 60M 基线、7 日覆盖、封档冲突和并发加载
wave-1-h2-seal-preflight-7d-container:
    @TOTAL_ROWS="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from audit_event")"; if [ "$TOTAL_ROWS" -lt 60000000 ]; then echo "audit_event rows must be >= 60000000 before seal run: $TOTAL_ROWS" >&2; exit 2; fi
    @DAYS_WITH_EVENTS="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from (select occurred_at::date from audit_event where occurred_at >= current_date - 7 and occurred_at < current_date group by 1 having count(*) > 0) d")"; if [ "$DAYS_WITH_EVENTS" -ne 7 ]; then echo "target window must have audit_event rows on 7 days before seal run: $DAYS_WITH_EVENTS" >&2; exit 2; fi
    @EVENT_DAYS_WITHOUT_SEAL="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from (select current_date - offs as seal_date from generate_series(1,7) as offs) d where exists (select 1 from audit_event e where e.occurred_at >= d.seal_date and e.occurred_at < d.seal_date + interval '1 day') and not exists (select 1 from audit_chain_seal s where s.seal_date = d.seal_date)")"; if [ "$EVENT_DAYS_WITHOUT_SEAL" -ne 7 ]; then echo "target window must have 7 unsealed event days before seal run: $EVENT_DAYS_WITHOUT_SEAL" >&2; exit 2; fi
    @SEALED_COUNT="$(sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select count(*) from audit_chain_seal where seal_date >= current_date - 7 and seal_date < current_date")"; if [ "$SEALED_COUNT" -ne 0 ]; then echo "target date range already contains audit_chain_seal rows: $SEALED_COUNT" >&2; exit 2; fi
    @if ps -eo cmd | rg -q '[w]ms-audit-baseline-load|[d]ocker run --rm --pull=never --network wms-dev-h2_default'; then echo "baseline loader already running; refusing seal preflight" >&2; exit 2; fi
    @echo "writes_runtime_evidence=false"
    @sudo -n docker exec wms-dev-h2-postgres-dev-h2-1 psql -U wms_dev_h2 -d wms_dev_h2 -v ON_ERROR_STOP=1 -Atc "select 'seal preflight ok: audit_event_rows=' || (select count(*) from audit_event) || ' event_days=' || (select count(*) from (select occurred_at::date from audit_event where occurred_at >= current_date - 7 and occurred_at < current_date group by 1 having count(*) > 0) d) || ' unsealed_event_days=' || (select count(*) from (select current_date - offs as seal_date from generate_series(1,7) as offs) d where exists (select 1 from audit_event e where e.occurred_at >= d.seal_date and e.occurred_at < d.seal_date + interval '1 day') and not exists (select 1 from audit_chain_seal s where s.seal_date = d.seal_date))"

# Wave 1 H2 dev 7 天封档执行：真实写 dev-h2 audit_chain_seal，不写 runtime evidence
wave-1-h2-seal-run-7d-container:
    @just wave-1-h2-seal-preflight-7d-container
    @cargo build --manifest-path backend/Cargo.toml -p wms-api --release --bin audit-maintenance
    @sudo -n docker run --rm --pull=never --network wms-dev-h2_default \
      --env-file deploy/env/dev-h2.env \
      --workdir /tmp \
      --user "$(id -u):$(id -g)" \
      -v "$PWD/backend/target/release/audit-maintenance:/tmp/audit-maintenance:ro" \
      -v "$PWD/deploy/scripts/audit_maintenance.sh:/tmp/audit_maintenance.sh:ro" \
      -e AUDIT_MAINTENANCE_BIN=/tmp/audit-maintenance \
      --entrypoint /bin/sh \
      wms-api-dev-h2:${WMS_VERSION:-latest} \
      -c 'export DATABASE_URL="postgres://wms_dev_h2:${WMS_DEV_H2_DB_PASSWORD}@postgres-dev-h2:5432/wms_dev_h2"; for offset in 7 6 5 4 3 2 1; do seal_date="$(date -u -d "$offset days ago" +%F)"; /tmp/audit_maintenance.sh --seal-date "$seal_date"; done'

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

# 采集 Wave 1 H2 runtime 证据：必须在真实 dev PostgreSQL + wrk 压测完成后运行，拒绝本机 dev-h2 readiness alias
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
