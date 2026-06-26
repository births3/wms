#!/usr/bin/env python3
"""Validate prerequisites before collecting Wave 1 runtime evidence."""
from __future__ import annotations

import argparse
import ipaddress
import os
import re
import shutil
import socket
import sys
from pathlib import Path
from urllib.parse import urlparse


FORBIDDEN_BOUNDARY_RE = re.compile(
    r"(^|[^0-9a-z])(local|prod|production|prodution|localhost|127\.0\.0\.1|0\.0\.0\.0|stub|mock|fake|example)([^0-9a-z]|$)",
    re.IGNORECASE,
)
H2_FORBIDDEN_BOUNDARY_RE = re.compile(
    r"(^|[^0-9a-z])(staging|local|prod|production|prodution|localhost|127\.0\.0\.1|0\.0\.0\.0|stub|mock|fake|example)([^0-9a-z]|$)",
    re.IGNORECASE,
)
H2_DRY_RUN_ALIAS = "dev-h2.wms.internal"
DEV_DB_HOST_ALLOWLIST_ENV = "WMS_DEV_DB_HOST_ALLOWLIST"
DEFAULT_FORMAL_DEV_DB_HOST = "pg-dev.wms.internal"
COMPOSE_ENV_VAR_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?:(:?[-?+]).*?)?\}")


def contains_environment_token(value: str, environment: str) -> bool:
    return (
        re.search(
            rf"(^|[^0-9a-z]){re.escape(environment.lower())}([^0-9a-z]|$)",
            value.lower(),
        )
        is not None
    )


def contains_forbidden_boundary(value: str) -> bool:
    return FORBIDDEN_BOUNDARY_RE.search(value) is not None


def contains_h2_forbidden_boundary(value: str) -> bool:
    return H2_FORBIDDEN_BOUNDARY_RE.search(value) is not None


def database_boundary_value(database_url: str) -> str:
    parsed = urlparse(database_url)
    if not parsed.scheme or not parsed.netloc:
        return database_url
    host = parsed.hostname or ""
    port = f":{parsed.port}" if parsed.port else ""
    return f"{host}{port}{parsed.path}"


def database_host(database_url: str) -> str:
    parsed = urlparse(database_url)
    if parsed.scheme and parsed.netloc:
        return parsed.hostname or ""
    boundary = database_url.rsplit("@", 1)[-1]
    host_port = boundary.split("/", 1)[0]
    if host_port.startswith("[") and "]" in host_port:
        return host_port[1:].split("]", 1)[0]
    return host_port.split(":", 1)[0]


def is_raw_ip(host: str) -> bool:
    try:
        ipaddress.ip_address(host)
    except ValueError:
        return False
    return True


def env_value(name: str, errors: list[str]) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        errors.append(f"{name} is required")
    return value


def require_commands(commands: list[str], errors: list[str]) -> None:
    for command in commands:
        if shutil.which(command) is None:
            errors.append(f"required command not found: {command}")


def validate_external_ref(label: str, value: str, environment: str, errors: list[str]) -> None:
    if not value:
        return
    if contains_forbidden_boundary(value):
        errors.append(f"{label} must not point to localhost/local/prod/production/stub/mock/fake/example boundaries")
    if not contains_environment_token(value, environment):
        errors.append(f"{label} must include environment token: {environment}")


def validate_h2_external_ref(label: str, value: str, errors: list[str]) -> None:
    environment = "dev"
    if not value:
        return
    if contains_h2_forbidden_boundary(value):
        errors.append(
            f"{label} must be a dev boundary and must not point to staging/localhost/local/prod/production/stub/mock/fake/example boundaries"
        )
    if not contains_environment_token(value, environment):
        errors.append(f"{label} must include environment token: {environment}")


def validate_database_url(database_url: str, environment: str, errors: list[str]) -> None:
    if not database_url:
        return
    boundary = database_boundary_value(database_url)
    validate_external_ref("WAVE1_H2_DATABASE_URL", boundary, environment, errors)


def validate_h2_database_url(database_url: str, errors: list[str]) -> None:
    if not database_url:
        return
    host = database_host(database_url)
    if not host:
        errors.append("WAVE1_H2_DATABASE_URL host is required")
        return
    if contains_h2_forbidden_boundary(host):
        errors.append(
            "WAVE1_H2_DATABASE_URL must be a dev boundary and must not point to staging/localhost/local/prod/production/stub/mock/fake/example boundaries"
        )
    if is_raw_ip(host):
        errors.append("WAVE1_H2_DATABASE_URL host must be a dev DNS name, not a raw IP address")
    if not contains_environment_token(host, "dev"):
        errors.append("WAVE1_H2_DATABASE_URL must include environment token: dev")


def validate_h2_formal_database_url(database_url: str, errors: list[str]) -> None:
    if not database_url:
        return
    host = database_host(database_url).lower()
    if H2_DRY_RUN_ALIAS == host:
        errors.append(
            "WAVE1_H2_DATABASE_URL uses dev-h2.wms.internal, which is only allowed for readiness dry-run and cannot be used for formal H2 runtime evidence"
        )
    allowed_hosts = allowed_formal_dev_hosts(errors)
    if allowed_hosts and host not in allowed_hosts:
        errors.append(
            f"WAVE1_H2_DATABASE_URL host {host} is not in {DEV_DB_HOST_ALLOWLIST_ENV}; add the real dev PostgreSQL DNS before formal H2 runtime evidence"
        )
    if allowed_hosts and host in allowed_hosts:
        reject_loopback_resolution(host, errors)


def allowed_formal_dev_hosts(errors: list[str]) -> set[str]:
    raw = os.environ.get(DEV_DB_HOST_ALLOWLIST_ENV, "").strip() or DEFAULT_FORMAL_DEV_DB_HOST
    hosts = {host.strip().lower() for host in raw.split(",") if host.strip()}
    if not hosts:
        errors.append(f"{DEV_DB_HOST_ALLOWLIST_ENV} must contain at least one dev DB host")
        return set()
    for host in sorted(hosts):
        if is_raw_ip(host):
            errors.append(f"{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must be a DNS name, not raw IP")
        if not contains_environment_token(host, "dev"):
            errors.append(f"{DEV_DB_HOST_ALLOWLIST_ENV} host {host} must include dev boundary token")
        if host == H2_DRY_RUN_ALIAS:
            errors.append(f"{DEV_DB_HOST_ALLOWLIST_ENV} must not include dry-run alias {H2_DRY_RUN_ALIAS}")
    return hosts


def resolve_host_ips(host: str) -> list[str]:
    return sorted({info[4][0] for info in socket.getaddrinfo(host, None)})


def reject_loopback_resolution(host: str, errors: list[str]) -> None:
    try:
        ips = resolve_host_ips(host)
    except OSError as error:
        errors.append(f"WAVE1_H2_DATABASE_URL host {host} must resolve before formal H2 runtime evidence: {error}")
        return
    if not ips:
        errors.append(f"WAVE1_H2_DATABASE_URL host {host} did not resolve to any IP address")
        return
    for ip in ips:
        try:
            parsed = ipaddress.ip_address(ip)
        except ValueError:
            errors.append(f"WAVE1_H2_DATABASE_URL host {host} resolved invalid IP {ip}")
            continue
        if parsed.is_loopback:
            errors.append(
                f"WAVE1_H2_DATABASE_URL host {host} resolves to loopback and cannot be used for formal H2 runtime evidence"
            )


def validate_int_floor(name: str, default: int, minimum: int, errors: list[str]) -> None:
    raw = os.environ.get(name, str(default)).strip()
    try:
        value = int(raw)
    except ValueError:
        errors.append(f"{name} must be an integer")
        return
    if value < minimum:
        errors.append(f"{name} must be >= {minimum}")


def validate_int_equals(name: str, default: int, expected: int, errors: list[str]) -> None:
    raw = os.environ.get(name, str(default)).strip()
    try:
        value = int(raw)
    except ValueError:
        errors.append(f"{name} must be an integer")
        return
    if value != expected:
        errors.append(f"{name} must be {expected}")


def validate_planned_file(path_text: str, label: str, require_exists: bool, errors: list[str]) -> None:
    if not path_text:
        return
    path = Path(path_text)
    if require_exists:
        if not path.is_file():
            errors.append(f"{label} must point to an existing file")
        return
    parent = path.parent if path.parent != Path("") else Path(".")
    if not parent.exists():
        errors.append(f"{label} parent directory must exist")


def parse_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            continue
        values[key] = value.strip().strip("'\"")
    return values


def required_compose_env_vars(compose_text: str) -> set[str]:
    required: set[str] = set()
    for match in COMPOSE_ENV_VAR_RE.finditer(compose_text):
        name = match.group(1)
        operator = match.group(2) or ""
        if operator in {":-", "-"}:
            continue
        required.add(name)
    return required


def validate_compose_env_values(compose_file: Path, compose_env_file: str, errors: list[str]) -> None:
    if not compose_file.is_file():
        return

    try:
        required = required_compose_env_vars(compose_file.read_text(encoding="utf-8"))
    except UnicodeDecodeError:
        errors.append("WAVE1_COMPOSE_FILE must be UTF-8 text")
        return

    if not required:
        return

    env_file_values: dict[str, str] = {}
    if compose_env_file:
        env_path = Path(compose_env_file)
        if env_path.is_file():
            env_file_values = parse_env_file(env_path)

    missing = sorted(name for name in required if not os.environ.get(name) and not env_file_values.get(name))
    if missing:
        errors.append("compose env is missing required values: " + ", ".join(missing))


def validate_h2(require_wrk_output: bool) -> list[str]:
    errors: list[str] = []
    environment = "dev"

    database_url = env_value("WAVE1_H2_DATABASE_URL", errors)
    wrk_output = env_value("WAVE1_H2_WRK_OUTPUT", errors)
    benchmark_log_ref = env_value("WAVE1_H2_BENCHMARK_LOG_REF", errors)
    cron_log_ref = env_value("WAVE1_H2_CRON_LOG_REF", errors)

    require_commands(["psql", "wrk"], errors)
    validate_h2_database_url(database_url, errors)
    if require_wrk_output:
        validate_h2_formal_database_url(database_url, errors)
    validate_planned_file(wrk_output, "WAVE1_H2_WRK_OUTPUT", require_wrk_output, errors)
    validate_h2_external_ref("WAVE1_H2_BENCHMARK_LOG_REF", benchmark_log_ref, errors)
    validate_h2_external_ref("WAVE1_H2_CRON_LOG_REF", cron_log_ref, errors)
    validate_int_floor("WAVE1_H2_DURATION_SECONDS", 3600, 3600, errors)
    validate_int_floor("WAVE1_H2_TARGET_QPS", 1000, 1000, errors)
    validate_int_equals("WAVE1_H2_SEAL_FAILURE_COUNT", 0, 0, errors)

    return errors


def rollback_environment(errors: list[str]) -> str:
    environment = env_value("WAVE1_ROLLBACK_ENVIRONMENT", errors)
    if environment and environment not in {"dev", "staging"}:
        errors.append("WAVE1_ROLLBACK_ENVIRONMENT must be dev or staging")
    return environment if environment in {"dev", "staging"} else ""


def validate_signal(environment: str, errors: list[str]) -> None:
    smoke_url = os.environ.get("SMOKE_URL", "").strip()
    prometheus_url = os.environ.get("PROMETHEUS_URL", "").strip()
    prometheus_query = os.environ.get("PROMETHEUS_QUERY", "").strip()

    if smoke_url and (prometheus_url or prometheus_query):
        errors.append("configure either SMOKE_URL or PROMETHEUS_URL + PROMETHEUS_QUERY, not both")
        return

    if smoke_url:
        validate_external_ref("SMOKE_URL", smoke_url, environment, errors)
        return

    if prometheus_url or prometheus_query:
        if not prometheus_url or not prometheus_query:
            errors.append("Prometheus evidence requires both PROMETHEUS_URL and PROMETHEUS_QUERY")
            return
        if contains_forbidden_boundary(prometheus_url) or contains_forbidden_boundary(prometheus_query):
            errors.append("Prometheus boundary must not point to localhost/local/prod/production/stub/mock/fake/example boundaries")
        if not contains_environment_token(prometheus_url, environment):
            errors.append(f"Prometheus URL must include environment token: {environment}")
        if not contains_environment_token(prometheus_query, environment):
            errors.append(f"Prometheus query must include environment token: {environment}")
        return

    errors.append("missing runtime signal: set SMOKE_URL or PROMETHEUS_URL + PROMETHEUS_QUERY")


def validate_rollback_common(errors: list[str]) -> str:
    environment = rollback_environment(errors)
    rollback_log_ref = env_value("WAVE1_ROLLBACK_LOG_REF", errors)
    external_log_ref = env_value("WAVE1_EXTERNAL_LOG_REF", errors)

    require_commands(["curl"], errors)
    if environment:
        validate_external_ref("WAVE1_ROLLBACK_LOG_REF", rollback_log_ref, environment, errors)
        validate_external_ref("WAVE1_EXTERNAL_LOG_REF", external_log_ref, environment, errors)
        validate_signal(environment, errors)
    return environment


def validate_rollback_k8s() -> list[str]:
    errors: list[str] = []
    environment = validate_rollback_common(errors)
    context = env_value("WAVE1_K8S_CONTEXT", errors)
    namespace = env_value("WAVE1_K8S_NAMESPACE", errors)

    require_commands(["kubectl"], errors)
    if environment:
        validate_external_ref("WAVE1_K8S_CONTEXT", context, environment, errors)
        validate_external_ref("WAVE1_K8S_NAMESPACE", namespace, environment, errors)

    return errors


def validate_rollback_compose() -> list[str]:
    errors: list[str] = []
    environment = validate_rollback_common(errors)
    previous_version = env_value("WAVE1_PREVIOUS_VERSION", errors)
    compose_file = env_value("WAVE1_COMPOSE_FILE", errors)
    compose_env_file = os.environ.get("WAVE1_COMPOSE_ENV_FILE", "").strip()

    require_commands(["docker"], errors)
    if previous_version and contains_forbidden_boundary(previous_version):
        errors.append("WAVE1_PREVIOUS_VERSION must not point to localhost/local/prod/production/stub/mock/fake/example boundaries")
    if compose_file:
        path = Path(compose_file)
        if not path.is_file():
            errors.append("WAVE1_COMPOSE_FILE must point to an existing file")
        if environment:
            validate_external_ref("WAVE1_COMPOSE_FILE", str(path), environment, errors)
    if compose_env_file:
        env_path = Path(compose_env_file)
        if not env_path.is_file():
            errors.append("WAVE1_COMPOSE_ENV_FILE must point to an existing file")
    if compose_file:
        validate_compose_env_values(Path(compose_file), compose_env_file, errors)

    return errors


def print_result(mode: str, errors: list[str]) -> int:
    if not errors:
        print(f"wave1 runtime evidence prerequisites ok: mode={mode}")
        return 0

    print(f"wave1 runtime evidence prerequisites failed: mode={mode}", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    return 2


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", required=True, choices=["h2", "rollback-k8s", "rollback-compose"])
    parser.add_argument(
        "--require-wrk-output",
        action="store_true",
        help="Require WAVE1_H2_WRK_OUTPUT to exist; use this immediately before writing H2 evidence.",
    )
    args = parser.parse_args(argv)

    if args.mode == "h2":
        errors = validate_h2(require_wrk_output=args.require_wrk_output)
    elif args.mode == "rollback-k8s":
        errors = validate_rollback_k8s()
    else:
        errors = validate_rollback_compose()

    return print_result(args.mode, errors)


if __name__ == "__main__":
    sys.exit(main())
