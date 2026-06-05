#!/usr/bin/env python3
"""report_wave1_completion.py — Wave 1 完成度证据报告

类别：4. 流程治理（报告型，默认不阻塞）
Tier：手动 / Wave 出口检查
输入：ROADMAP.md Wave 1 完成标准 + 当前仓库文件
输出：人类可读 + --json
退出码：
  默认：0（只报告当前证据）
  --strict：无阻塞缺口返回 0；任一阻塞缺口返回 1

本脚本只把 ROADMAP.md 已有完成标准转成可复跑证据检查，不新增业务语义。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent

PROVED_BY_STATIC_FILES = "PROVED_BY_STATIC_FILES"
MISSING_OR_NEEDS_CONFIRMATION = "MISSING_OR_NEEDS_CONFIRMATION"
NOT_SCRIPT_JUDGEABLE = "NOT_SCRIPT_JUDGEABLE"


@dataclass
class EvidenceItem:
    item_id: str
    requirement: str
    status: str  # PROVED_BY_STATIC_FILES | MISSING_OR_NEEDS_CONFIRMATION | NOT_SCRIPT_JUDGEABLE
    evidence: list[str] = field(default_factory=list)
    gaps: list[str] = field(default_factory=list)
    strict_blocking: bool = True

    @property
    def complete(self) -> bool:
        return self.status == PROVED_BY_STATIC_FILES

    @property
    def blocks_strict(self) -> bool:
        return self.strict_blocking and not self.complete


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8") if path.exists() else ""


def file_exists(path: str) -> bool:
    return (REPO_ROOT / path).exists()


def file_contains(path: str, needle: str) -> bool:
    return needle in read_text(REPO_ROOT / path)


def any_file_contains(root: str, pattern: str) -> bool:
    base = REPO_ROOT / root
    if not base.exists():
        return False
    regex = re.compile(pattern)
    for path in base.rglob("*"):
        if "target" in path.parts:
            continue
        if not path.is_file():
            continue
        if path.suffix in {".png", ".lock"}:
            continue
        try:
            if regex.search(path.read_text(encoding="utf-8")):
                return True
        except UnicodeDecodeError:
            continue
    return False


def accepted_adr(path: str) -> bool:
    text = read_text(REPO_ROOT / path)
    return any(line.strip().startswith("- 状态：") and "Accepted" in line for line in text.splitlines())


def contains_environment_token(value: str, environment: str) -> bool:
    return re.search(rf"(^|[^0-9a-z]){re.escape(environment.lower())}([^0-9a-z]|$)", value.lower()) is not None


def contains_forbidden_runtime_boundary(value: str, *, allow_example_refs: bool = False) -> bool:
    forbidden = r"prod|production|prodution|localhost|127\.0\.0\.1|0\.0\.0\.0|stub|mock|fake"
    if not allow_example_refs:
        forbidden = f"{forbidden}|example"
    return re.search(rf"(^|[^0-9a-z])({forbidden})([^0-9a-z]|$)", value.lower()) is not None


def coerce_int(value: object) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def coerce_float(value: object) -> float | None:
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def validate_w1d_runtime_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "W1.D runtime evidence 必须是 JSON object"

    environment = str(payload.get("environment", ""))
    if environment not in {"dev", "staging"}:
        return False, "W1.D runtime evidence environment 必须是 dev 或 staging"

    signal_url = str(payload.get("signal_url", ""))
    rollback_log_ref = str(payload.get("rollback_log_ref", ""))
    external_log_ref = str(payload.get("external_log_ref", ""))
    if not signal_url or not rollback_log_ref or not external_log_ref:
        return False, "W1.D runtime evidence 必须包含 signal_url / rollback_log_ref / external_log_ref"
    if any(
        contains_forbidden_runtime_boundary(value, allow_example_refs=allow_example_refs)
        for value in [signal_url, rollback_log_ref, external_log_ref]
    ):
        return False, "W1.D runtime evidence 不能指向 localhost/127.0.0.1/0.0.0.0/prod/stub/mock/fake/example 边界"
    missing_environment_refs = [
        name
        for name, value in {
            "signal_url": signal_url,
            "rollback_log_ref": rollback_log_ref,
            "external_log_ref": external_log_ref,
        }.items()
        if not contains_environment_token(value, environment)
    ]
    if missing_environment_refs:
        return False, (
            "W1.D runtime evidence 必须在每个 signal/log 引用中包含 "
            f"{environment} 环境标记: {', '.join(missing_environment_refs)}"
        )
    if payload.get("rollback_triggered") is not True or payload.get("rollback_exit_code") != 0:
        return False, "W1.D runtime evidence 必须证明失败信号触发 rollback 且退出码为 0"
    if payload.get("signal_type") not in {"http", "prometheus"}:
        return False, "W1.D runtime evidence signal_type 必须是 http 或 prometheus"
    if not payload.get("captured_at"):
        return False, "W1.D runtime evidence 必须包含 captured_at"

    return True, "W1.D runtime evidence 内容有效"


def valid_w1d_runtime_evidence() -> tuple[bool, str]:
    path = REPO_ROOT / "docs/retros/wave-1-runtime-evidence.json"
    if not path.exists():
        return False, "缺少 docs/retros/wave-1-runtime-evidence.json 真实 dev/staging 自动回滚证据"

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return False, f"docs/retros/wave-1-runtime-evidence.json JSON 无效：{error}"

    ok, message = validate_w1d_runtime_payload(payload)
    if not ok:
        return False, message
    return True, "docs/retros/wave-1-runtime-evidence.json 记录真实 dev/staging 自动回滚证据"


def valid_h1_auth_story_alignment() -> tuple[bool, str]:
    path = REPO_ROOT / "docs/domain/user-stories-h1-auth-tenant.md"
    if not path.exists():
        return False, "缺少 docs/domain/user-stories-h1-auth-tenant.md"

    text = read_text(path)
    required = [
        ("ADR-0024", "引用 ADR-0024"),
        ("Access Token 默认 1 小时", "Access Token 默认 1 小时"),
        ("Refresh Token 默认 24 小时", "Refresh Token 默认 24 小时"),
        ("AuthContext.owner_id", "使用 AuthContext.owner_id 作 Wave 1 隔离锚点"),
        ("PostgreSQL RLS 延后", "声明 PostgreSQL RLS 延后评估"),
        ("AUTH-009", "权限变更使用 AUTH-009 失效链路"),
    ]
    missing = [message for needle, message in required if needle not in text]
    if missing:
        return False, "H1 用户故事未对齐 ADR-0024：" + "；".join(missing)

    forbidden = [
        "有效期默认 8 小时",
        "Refresh Token 默认 7 天",
        "PostgreSQL Row-Level Security (RLS)",
        "强制刷新 token",
        "Redis 不可用 → 降级到 PG 黑名单表",
    ]
    stale = [needle for needle in forbidden if needle in text]
    if stale:
        return False, "H1 用户故事仍含 ADR-0024 旧口径：" + "；".join(stale)

    return True, "H1 用户故事 token/RLS/owner_id 口径已对齐 ADR-0024"


def validate_h2_runtime_payload(
    payload: object,
    *,
    allow_example_refs: bool = False,
) -> tuple[bool, str]:
    if not isinstance(payload, dict):
        return False, "H2 runtime evidence 必须是 JSON object"
    environment = str(payload.get("environment", ""))
    if environment != "dev":
        return False, "H2 runtime evidence environment 必须是 dev"
    if not payload.get("captured_at"):
        return False, "H2 runtime evidence 必须包含 captured_at"

    performance = payload.get("performance")
    if not isinstance(performance, dict):
        return False, "H2 runtime evidence 必须包含 performance 对象"
    if performance.get("tool") != "wrk":
        return False, "H2 runtime evidence performance.tool 必须是 wrk"
    baseline_rows = coerce_int(performance.get("baseline_rows", 0))
    target_qps = coerce_int(performance.get("target_qps", 0))
    observed_qps = coerce_float(performance.get("observed_qps", 0.0))
    duration_seconds = coerce_int(performance.get("duration_seconds", 0))
    p99_ms = coerce_float(performance.get("p99_ms", 999999.0))
    if baseline_rows is None:
        return False, "H2 runtime evidence baseline_rows 必须是整数"
    if target_qps is None:
        return False, "H2 runtime evidence target_qps 必须是整数"
    if observed_qps is None:
        return False, "H2 runtime evidence observed_qps 必须是数字"
    if duration_seconds is None:
        return False, "H2 runtime evidence duration_seconds 必须是整数"
    if p99_ms is None:
        return False, "H2 runtime evidence p99_ms 必须是数字"
    if baseline_rows < 60_000_000:
        return False, "H2 runtime evidence baseline_rows 必须 >= 60000000"
    if target_qps < 1000:
        return False, "H2 runtime evidence target_qps 必须 >= 1000"
    if observed_qps < 1000.0:
        return False, "H2 runtime evidence observed_qps 必须 >= 1000"
    if duration_seconds < 3600:
        return False, "H2 runtime evidence duration_seconds 必须 >= 3600"
    if p99_ms >= 200.0:
        return False, "H2 runtime evidence p99_ms 必须 < 200"
    benchmark_log_ref = str(performance.get("benchmark_log_ref", ""))
    if not benchmark_log_ref:
        return False, "H2 runtime evidence 必须包含 performance.benchmark_log_ref"
    if (
        contains_forbidden_runtime_boundary(benchmark_log_ref, allow_example_refs=allow_example_refs)
        or not contains_environment_token(benchmark_log_ref, environment)
    ):
        return False, "H2 runtime evidence benchmark_log_ref 必须指向非本机 dev 证据"

    seal_cron = payload.get("seal_cron")
    if not isinstance(seal_cron, dict):
        return False, "H2 runtime evidence 必须包含 seal_cron 对象"
    consecutive_success_days = coerce_int(seal_cron.get("consecutive_success_days", 0))
    failure_count = coerce_int(seal_cron.get("failure_count", 1))
    if consecutive_success_days is None:
        return False, "H2 runtime evidence seal_cron.consecutive_success_days 必须是整数"
    if failure_count is None:
        return False, "H2 runtime evidence seal_cron.failure_count 必须是整数"
    if consecutive_success_days < 7:
        return False, "H2 runtime evidence seal_cron.consecutive_success_days 必须 >= 7"
    if failure_count != 0:
        return False, "H2 runtime evidence seal_cron.failure_count 必须为 0"
    if seal_cron.get("last_seal_verified") is not True:
        return False, "H2 runtime evidence seal_cron.last_seal_verified 必须为 true"
    cron_log_ref = str(seal_cron.get("cron_log_ref", ""))
    if not cron_log_ref:
        return False, "H2 runtime evidence 必须包含 seal_cron.cron_log_ref"
    if (
        contains_forbidden_runtime_boundary(cron_log_ref, allow_example_refs=allow_example_refs)
        or not contains_environment_token(cron_log_ref, environment)
    ):
        return False, "H2 runtime evidence cron_log_ref 必须指向非本机 dev 证据"

    return True, "H2 runtime evidence 内容有效"


def valid_h2_runtime_evidence() -> tuple[bool, str]:
    path = REPO_ROOT / "docs/retros/wave-1-h2-runtime-evidence.json"
    if not path.exists():
        return False, "缺少 docs/retros/wave-1-h2-runtime-evidence.json 真实 PostgreSQL 压测与封档证据"

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return False, f"docs/retros/wave-1-h2-runtime-evidence.json JSON 无效：{error}"

    ok, message = validate_h2_runtime_payload(payload)
    if not ok:
        return False, message
    return True, "docs/retros/wave-1-h2-runtime-evidence.json 记录真实 PostgreSQL 压测与封档证据"


def valid_h2_runtime_collection_assets() -> tuple[bool, str]:
    checks = [
        (
            file_exists("scripts/governance/collect_wave1_h2_runtime_evidence.py"),
            "缺少 H2 runtime evidence collector",
        ),
        (
            file_contains("justfile", "wave-1-h2-runtime-evidence"),
            "justfile 缺少 wave-1-h2-runtime-evidence 入口",
        ),
        (
            file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-h2-runtime-evidence"),
            "runbook 缺少 H2 runtime evidence 采集命令",
        ),
        (
            file_exists("scripts/governance/check_wave1_runtime_evidence_prereqs.py")
            and file_exists("scripts/governance/check_wave1_h2_runtime_readiness.py")
            and file_exists("scripts/governance/validate_wave1_runtime_evidence.py")
            and file_contains("justfile", "wave-1-runtime-prereq-h2")
            and file_contains("justfile", "wave-1-h2-runtime-readiness")
            and file_contains("justfile", "wave-1-runtime-evidence-validate")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-runtime-prereq-h2")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-h2-runtime-readiness")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-runtime-evidence-validate"),
            "缺少 H2 runtime evidence 前置检查入口",
        ),
    ]
    missing = [message for ok, message in checks if not ok]
    if missing:
        return False, "；".join(missing)
    return True, "H2 runtime 采集器、DB readiness、evidence validator、just 入口与 runbook 已就绪"


def valid_w1d_runtime_collection_assets() -> tuple[bool, str]:
    checks = [
        (
            file_exists("deploy/scripts/wave1_auto_rollback_probe.sh"),
            "缺少 W1.D 自动回滚 probe",
        ),
        (
            file_contains("justfile", "wave-1-rollback-runtime-evidence-k8s")
            and file_contains("justfile", "wave-1-rollback-runtime-evidence-compose"),
            "justfile 缺少 W1.D k8s / docker-compose runtime evidence 入口",
        ),
        (
            file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-rollback-runtime-evidence-k8s")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-rollback-runtime-evidence-compose"),
            "runbook 缺少 W1.D runtime evidence 采集命令",
        ),
        (
            file_exists("scripts/governance/check_wave1_runtime_evidence_prereqs.py")
            and file_exists("scripts/governance/validate_wave1_runtime_evidence.py")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "--check-only")
            and file_contains("justfile", "wave-1-runtime-prereq-rollback-k8s")
            and file_contains("justfile", "wave-1-runtime-prereq-rollback-compose")
            and file_contains("justfile", "wave-1-rollback-runtime-readiness-k8s")
            and file_contains("justfile", "wave-1-rollback-runtime-readiness-compose")
            and file_contains("justfile", "wave-1-runtime-evidence-validate")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-runtime-prereq-rollback-k8s")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-runtime-prereq-rollback-compose")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-rollback-runtime-readiness-k8s")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-rollback-runtime-readiness-compose")
            and file_contains("docs/runbooks/wave-1-runtime-evidence.md", "just wave-1-runtime-evidence-validate"),
            "缺少 W1.D runtime evidence 前置检查入口",
        ),
    ]
    missing = [message for ok, message in checks if not ok]
    if missing:
        return False, "；".join(missing)
    return True, "W1.D runtime probe、readiness、evidence validator、前置检查、just 入口与 runbook 已就绪"


def status_from_checks(checks: list[tuple[bool, str]]) -> tuple[str, list[str], list[str]]:
    evidence = [message for ok, message in checks if ok]
    gaps = [message for ok, message in checks if not ok]
    if not gaps:
        return PROVED_BY_STATIC_FILES, evidence, []
    return MISSING_OR_NEEDS_CONFIRMATION, evidence, gaps


def evaluate_wave1() -> list[EvidenceItem]:
    items: list[EvidenceItem] = []

    h1_story_ok, h1_story_message = valid_h1_auth_story_alignment()
    h1_checks = [
        (accepted_adr("docs/adr/0024-auth-model.md"), "ADR-0024 鉴权模型已 Accepted"),
        (h1_story_ok, h1_story_message),
        (any_file_contains("backend/crates", r"struct\s+AuthContext"), "后端包含 AuthContext"),
        (any_file_contains("backend/crates", r"FromRequestParts"), "AuthContext extractor 可挂 handler"),
        (any_file_contains("backend", r"jsonwebtoken"), "后端依赖/代码包含 jsonwebtoken"),
        (
            any_file_contains("backend/crates/api", r"auth_context_extractor_is_demo_items_handler_compatible")
            and any_file_contains("backend/crates/api", r"/api/v1/demo/items"),
            "非 auth 示例业务 handler 已通过 AuthContext 挂接测试",
        ),
        (
            any_file_contains("backend/crates/api", r"ACCESS_TOKEN_TTL_SECONDS")
            and any_file_contains("backend/crates/api", r"owner_id"),
            "H1 runtime 固化 access 1h 与 owner_id 隔离口径",
        ),
        (
            any_file_contains("backend/crates/api", r"AUTH-004")
            and any_file_contains("backend/crates/api", r"AUTH-009")
            and any_file_contains("backend/crates/api", r"permissions_changed_at")
            and any_file_contains("backend", r"redis"),
            "H1 runtime 包含 Redis blacklist 与 permissions_changed_at 失效链路",
        ),
        (
            any_file_contains("backend/crates/api", r"MissingRuntimePolicy")
            and any_file_contains("backend/crates/api", r"auth_runtime_layer")
            and any_file_contains("backend/crates/api", r"auth_context_extractor_uses_auth_runtime_policy_extension"),
            "H1 extractor 强制注入并执行 AuthRuntimePolicy",
        ),
    ]
    status, evidence, gaps = status_from_checks(h1_checks)
    items.append(EvidenceItem(
        "W1.A",
        "任意业务 handler 可挂 H1（AuthContext/JWT/多租户上下文）",
        status,
        evidence,
        gaps,
    ))

    h2_collection_ok, h2_collection_message = valid_h2_runtime_collection_assets()
    h2_checks = [
        (accepted_adr("docs/adr/0025-audit-storage-model.md"), "ADR-0025 审计存储模型已 Accepted"),
        (
            any_file_contains("backend", r"CREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+audit_event"),
            "存在 audit_event migration/schema",
        ),
        (any_file_contains("backend", r"append_event"), "后端包含 append_event 写入入口"),
        (any_file_contains("backend", r"audit_chain_seal"), "后端包含 audit_chain_seal 封档证据"),
        (any_file_contains("backend/crates/api", r"commit_with_audit"), "后端包含统一写操作审计 helper"),
        (
            any_file_contains("backend/crates/api", r"two_mutation_handlers_reuse_commit_with_audit")
            and any_file_contains("backend/crates/api", r"verify_hash_chain"),
            "至少两个 mutation handler 测试复用审计 helper 并校验 hash chain",
        ),
        (
            any_file_contains("backend/crates/api", r"PgPool")
            and any_file_contains("backend/crates/api", r"SELECT[\s\S]*FOR\s+UPDATE")
            and any_file_contains("backend/crates/api", r"INSERT[\s\S]*INTO\s+audit_event"),
            "H2 append_event 真实写入 PostgreSQL 并使用链头锁",
        ),
        (
            any_file_contains("backend", r"create_next_partition")
            and any_file_contains("backend", r"audit_chain_seal"),
            "H2 包含月分区维护与封档任务资产",
        ),
        (
            (
                file_exists("backend/tests/audit_postgres.rs")
                or file_exists("backend/crates/api/tests/audit_postgres.rs")
            )
            and any_file_contains("backend", r"#\[sqlx::test")
            and any_file_contains("backend", r"append_event\(&pool")
            and any_file_contains("backend", r"seal_audit_chain\(&pool"),
            "H2 包含真实 PostgreSQL append/seal 集成测试",
        ),
        (
            h2_collection_ok,
            h2_collection_message,
        ),
    ]
    status, evidence, gaps = status_from_checks(h2_checks)
    items.append(EvidenceItem(
        "W1.B",
        "任意写操作可经 H2 append-only 审计链路",
        status,
        evidence,
        gaps,
    ))

    h2_runtime_ok, h2_runtime_message = valid_h2_runtime_evidence()
    h2_runtime_status = PROVED_BY_STATIC_FILES if h2_runtime_ok else MISSING_OR_NEEDS_CONFIRMATION
    items.append(EvidenceItem(
        "W1.B-pre-release-runtime",
        "H2 真实 dev PostgreSQL 压测与 7 天封档 runtime evidence（预发布 gate，不阻塞 Wave 1 开发完成）",
        h2_runtime_status,
        [h2_runtime_message] if h2_runtime_ok else [],
        [] if h2_runtime_ok else [h2_runtime_message],
        strict_blocking=False,
    ))

    h3_checks = [
        (file_exists("backend/crates/api/src/lib.rs"), "backend/crates/api/src/lib.rs 存在"),
        (file_exists("backend/crates/openapi-export/src/main.rs"), "openapi-export 存在"),
        (file_exists("shared/openapi/openapi.json"), "shared/openapi/openapi.json 存在"),
        (file_exists("packages/api-client/src/schema.ts"), "api-client schema.ts 存在"),
        (file_contains("packages/api-client/package.json", "openapi-typescript"), "api-client 生成链路包含 openapi-typescript"),
    ]
    status, evidence, gaps = status_from_checks(h3_checks)
    items.append(EvidenceItem(
        "W1.C",
        "后端注解可生成 OpenAPI，前端 @wms/api-client 可消费",
        status,
        evidence,
        gaps,
    ))

    flag_static_checks = [
        (file_exists("deploy/feature_flags.toml"), "deploy/feature_flags.toml 存在"),
        (file_exists("scripts/governance/check_feature_flags.py"), "check_feature_flags.py 存在"),
        (file_contains("scripts/governance/governance_checks.py", '"check_feature_flags.py"'), "check_feature_flags.py 已进 T1"),
    ]
    status, evidence, gaps = status_from_checks(flag_static_checks)
    items.append(EvidenceItem(
        "W1.D-static",
        "check_feature_flags.py 进入 T1，文件版 Feature Flag 元数据可治理",
        status,
        evidence,
        gaps,
    ))

    retro = "docs/retros/wave-1-retro.md"
    w1d_collection_ok, w1d_collection_message = valid_w1d_runtime_collection_assets()
    rollback_checks = [
        (
            any_file_contains("backend", r"FeatureFlagRegistry::from_file")
            or any_file_contains("backend", r"FeatureFlagRegistry::from_toml_str"),
            "后端包含 FeatureFlagRegistry 文件版读取实现与测试证据",
        ),
        (
            file_exists("deploy/scripts/wave1_rollback.sh")
            and file_contains("deploy/scripts/wave1_rollback.sh", "kubectl rollout undo")
            and file_contains("deploy/scripts/wave1_rollback.sh", "docker compose")
            and file_contains("deploy/scripts/wave1_rollback.sh", "--execute")
            and file_contains("deploy/scripts/wave1_rollback.sh", "validate_environment_boundary")
            and file_contains("deploy/scripts/wave1_rollback.sh", 'validate_environment_boundary "--context" "$context"')
            and file_contains("deploy/scripts/wave1_rollback.sh", 'validate_environment_boundary "--namespace" "$namespace"')
            and file_contains("deploy/scripts/wave1_rollback.sh", 'validate_environment_boundary "--compose-file" "$compose_file_abs"')
            and file_contains("deploy/scripts/wave1_rollback.sh", "must include the selected environment token")
            and file_contains("deploy/scripts/wave1_rollback.sh", "must not point to a production boundary"),
            "deploy 下包含 dev/staging 回滚执行资产",
        ),
        (
            file_exists("deploy/scripts/wave1_auto_rollback_probe.sh")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "missing runtime evidence")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "--smoke-url")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "PROMETHEUS_URL")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "wave1_rollback.sh")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "--execute"),
            "deploy 下包含真实信号触发自动回滚入口，缺信号时不伪造证据",
        ),
        (
            file_exists("deploy/scripts/wave1_auto_rollback_probe.sh")
            and file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "PROMETHEUS_URL")
            and not file_contains("deploy/scripts/wave1_auto_rollback_probe.sh", "stub kubectl/docker"),
            "自动回滚 probe 只接受真实 dev/staging smoke gate 或监控信号",
        ),
        (
            w1d_collection_ok,
            w1d_collection_message,
        ),
    ]
    status, evidence, gaps = status_from_checks(rollback_checks)
    items.append(EvidenceItem(
        "W1.D-runtime",
        "文件版灰度链路 + 自动回滚运行资产已就绪",
        status,
        evidence,
        gaps,
    ))

    w1d_runtime_ok, w1d_runtime_message = valid_w1d_runtime_evidence()
    w1d_runtime_status = PROVED_BY_STATIC_FILES if w1d_runtime_ok else MISSING_OR_NEEDS_CONFIRMATION
    items.append(EvidenceItem(
        "W1.D-pre-release-runtime",
        "W1.D 真实 dev/staging 自动回滚 runtime evidence（预发布 gate，不阻塞 Wave 1 开发完成）",
        w1d_runtime_status,
        [w1d_runtime_message] if w1d_runtime_ok else [],
        [] if w1d_runtime_ok else [w1d_runtime_message],
        strict_blocking=False,
    ))

    web_checks = [
        (file_exists("apps/web-admin/src/App.tsx"), "apps/web-admin 壳工程存在"),
        (file_contains("apps/web-admin/src/App.tsx", "@wms/ui"), "web-admin 复用 @wms/ui"),
        (file_contains("apps/web-admin/src/lib/api.ts", "@wms/api-client"), "web-admin 接入 @wms/api-client"),
        (file_contains("apps/web-admin/src/App.tsx", "H1 权限与多租户"), "壳工程呈现 H1"),
        (file_contains("apps/web-admin/src/App.tsx", "H2 审计追踪"), "壳工程呈现 H2"),
        (not file_contains("apps/web-admin/src/App.tsx", 'id: "h1",\n    title: "H1 权限与多租户",\n    description: "AuthContext、JWT、多货主隔离和权限门控的生产入口。",\n    status: "pending"'), "H1 不再是 pending"),
        (not file_contains("apps/web-admin/src/App.tsx", 'id: "h2",\n    title: "H2 审计追踪",\n    description: "写操作接入 append-only 审计事件和审计查询链路。",\n    status: "pending"'), "H2 不再是 pending"),
    ]
    status, evidence, gaps = status_from_checks(web_checks)
    items.append(EvidenceItem(
        "W1.E",
        "apps/web-admin 壳工程复用 @wms/ui，并接入 H1/H2/H3 基础链路",
        status,
        evidence,
        gaps,
    ))

    contract_checks = [
        (accepted_adr("docs/adr/0030-integration-capability.md"), "ADR-0030 H-INT 已 Accepted"),
        (accepted_adr("docs/adr/0031-file-attachment-capability.md"), "ADR-0031 H-FILE 已 Accepted"),
        (accepted_adr("docs/adr/0032-approval-engine.md"), "ADR-0032 H-APV 已 Accepted"),
        (accepted_adr("docs/adr/0033-scheduler-engine.md"), "ADR-0033 H-SCH 已 Accepted"),
        (file_contains("docs/architecture-dependencies.md", "H-INT"), "依赖图登记 H-INT"),
        (file_contains("docs/architecture-dependencies.md", "H-FILE"), "依赖图登记 H-FILE"),
        (file_contains("docs/architecture-dependencies.md", "H-APV"), "依赖图登记 H-APV"),
        (file_contains("docs/architecture-dependencies.md", "H-SCH"), "依赖图登记 H-SCH"),
    ]
    status, evidence, gaps = status_from_checks(contract_checks)
    items.append(EvidenceItem(
        "W1.FGH-contracts",
        "W1.F/G/H + H-FILE 契约段登记一致",
        status,
        evidence,
        gaps,
    ))

    review_checks = [
        (file_exists(retro), "docs/retros/wave-1-retro.md 存在"),
        (file_contains(retro, "H-INT"), "retro 记录 H-INT"),
        (file_contains(retro, "H-FILE"), "retro 记录 H-FILE"),
        (file_contains(retro, "H-APV"), "retro 记录 H-APV"),
        (file_contains(retro, "H-SCH"), "retro 记录 H-SCH"),
        (file_contains(retro, "approval_source"), "retro 核对 approval_source 留痕"),
    ]
    status, evidence, gaps = status_from_checks(review_checks)
    items.append(EvidenceItem(
        "W1-review",
        "四横向契约联合评审结论记入 Wave 1 retro",
        status,
        evidence,
        gaps,
    ))

    items.append(EvidenceItem(
        "W1-external",
        "“码上放心”账号开通为外部并行启动项，需人工确认外部状态",
        NOT_SCRIPT_JUDGEABLE,
        [],
        ["脚本无法从仓库证明外部账号已开通"],
        strict_blocking=False,
    ))

    return items


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--strict", action="store_true", help="Wave 1 出口检查：未完成则返回 1")
    args = parser.parse_args(argv)

    items = evaluate_wave1()
    strict_pass = not any(item.blocks_strict for item in items)
    completed_count = sum(1 for item in items if item.complete)
    strict_items = [item for item in items if item.strict_blocking]
    completed_strict_count = sum(1 for item in strict_items if item.complete)

    if args.json:
        print(json.dumps({
            "report": "wave1_completion",
            "strict": args.strict,
            "strict_pass": strict_pass,
            "strict_proved_items": completed_strict_count,
            "strict_total_items": len(strict_items),
            "static_proved_items": completed_count,
            "total_items": len(items),
            "items": [asdict(item) for item in items],
        }, ensure_ascii=False, indent=2))
    else:
        print("report_wave1_completion (流程治理，静态证据覆盖报告)")
        print(f"  · strict_pass: {strict_pass}")
        print(f"  · strict_proved: {completed_strict_count}/{len(strict_items)}")
        print(f"  · static_proved: {completed_count}/{len(items)}")
        for item in items:
            mark = "✓" if item.complete else ("!" if not item.strict_blocking else "✘")
            print(f"\n  {mark} {item.item_id} [{item.status}] {item.requirement}")
            for evidence in item.evidence:
                print(f"      + {evidence}")
            for gap in item.gaps:
                print(f"      - {gap}")

    return 1 if args.strict and not strict_pass else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
