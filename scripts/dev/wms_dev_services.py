#!/usr/bin/env python3
"""Run the local WMS application services and restart them on source changes."""

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from types import FrameType
from urllib.parse import quote


IGNORED_DIRECTORIES = {
    ".git",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "artifacts",
    "node_modules",
    "target",
}
SERVICE_NAMES = ("api", "render", "h8")


@dataclass(frozen=True)
class ServiceSpec:
    name: str
    command: tuple[str, ...]
    watch_paths: tuple[Path, ...]


def _iter_files(root: Path):
    root = root.resolve()
    if root.is_file():
        yield root
        return
    if not root.is_dir():
        return
    for directory, directories, files in os.walk(root):
        directories[:] = sorted(
            directory_name
            for directory_name in directories
            if directory_name not in IGNORED_DIRECTORIES
        )
        directory_path = Path(directory)
        for filename in sorted(files):
            path = directory_path / filename
            if not path.is_symlink() and path.is_file():
                yield path


def fingerprint(paths: tuple[Path, ...] | list[Path]) -> tuple[tuple[str, int, int], ...]:
    """Return a cheap source fingerprint that excludes build and dependency output."""

    entries: dict[str, tuple[str, int, int]] = {}
    for root in paths:
        for path in _iter_files(Path(root)):
            try:
                stat = path.stat()
            except FileNotFoundError:
                continue
            resolved = str(path.resolve())
            entries[resolved] = (resolved, stat.st_mtime_ns, stat.st_size)
    return tuple(entries[key] for key in sorted(entries))


def service_specs(root: Path) -> dict[str, ServiceSpec]:
    root = root.resolve()
    backend = root / "backend"
    backend_sources = (
        backend / "crates",
        backend / "Cargo.toml",
        backend / "Cargo.lock",
    )
    return {
        "api": ServiceSpec(
            name="api",
            command=(
                "cargo",
                "run",
                "--manifest-path",
                "backend/Cargo.toml",
                "-p",
                "wms-api",
                "--bin",
                "wms-api",
            ),
            watch_paths=backend_sources + (root / "deploy/feature_flags.toml",),
        ),
        "render": ServiceSpec(
            name="render",
            command=("pnpm", "--dir", "apps/h9-render-worker", "start"),
            watch_paths=(
                root / "apps/h9-render-worker",
                root / "pnpm-lock.yaml",
                root / "pnpm-workspace.yaml",
            ),
        ),
        "h8": ServiceSpec(
            name="h8",
            command=(
                "cargo",
                "run",
                "--manifest-path",
                "backend/Cargo.toml",
                "-p",
                "h8-erp-worker",
                "--bin",
                "h8-erp-worker",
            ),
            watch_paths=backend_sources,
        ),
    }


def select_services(value: str) -> tuple[str, ...]:
    names = tuple(name.strip() for name in value.split(",") if name.strip())
    unknown = tuple(name for name in names if name not in SERVICE_NAMES)
    if unknown:
        raise ValueError(f"unknown service(s): {', '.join(unknown)}")
    if not names:
        raise ValueError("at least one service is required")
    return tuple(dict.fromkeys(names))


def dev_environment(values: dict[str, str] | None = None) -> dict[str, str]:
    env = dict(os.environ if values is None else values)
    db_password = env.get("WMS_DEV_H2_DB_PASSWORD")
    db_port = env.get("WMS_DEV_H2_DB_PORT", "15432")
    redis_port = env.get("WMS_DEV_H2_REDIS_PORT", "16379")
    minio_port = env.get("WMS_DEV_HFILE_API_PORT", "19000")
    render_port = env.get("WMS_DEV_H9_RENDER_PORT", "18090")

    if not env.get("WMS_DB_URL") and db_password:
        env["WMS_DB_URL"] = (
            "postgres://wms_dev_h2:"
            f"{quote(db_password, safe='')}@127.0.0.1:{db_port}/wms_dev_h2"
        )
    env.setdefault("WMS_BIND_ADDR", "0.0.0.0:18080")
    env.setdefault("WMS_REDIS_URL", f"redis://127.0.0.1:{redis_port}")
    env.setdefault("WMS_HFILE_ENDPOINT", f"http://127.0.0.1:{minio_port}")
    env.setdefault("WMS_HFILE_REGION", "us-east-1")
    env.setdefault("WMS_HFILE_BUCKET", "wms-attachments")
    env.setdefault("WMS_HFILE_SSE_MODE", "none")
    env.setdefault("WMS_H9_RENDER_HOST", "127.0.0.1")
    env.setdefault("WMS_H9_RENDER_PORT", render_port)
    env.setdefault("WMS_H9_RENDER_WORKER_URL", f"http://127.0.0.1:{render_port}/render")
    env.setdefault("WMS_API_BASE", "http://127.0.0.1:18080")
    env.setdefault("H8_WORKER_VERSION", "dev")
    return env


def validate_environment(env: dict[str, str], names: tuple[str, ...]) -> None:
    required: list[str] = []
    if "api" in names or "h8" in names:
        if not env.get("WMS_DB_URL"):
            required.append("WMS_DEV_H2_DB_PASSWORD or WMS_DB_URL")
    if "api" in names:
        required.extend(("WMS_JWT_SECRET", "WMS_H9_RENDER_TOKEN"))
    if "render" in names:
        required.append("WMS_H9_RENDER_TOKEN")
    if "h8" in names:
        required.extend(("H8_CONNECTOR_ID", "WMS_H8_WORKER_API_KEY", "WMS_H8_SECRET_ALIASES"))
    missing = list(dict.fromkeys(name for name in required if not env.get(name)))
    if missing:
        raise ValueError("missing development environment: " + ", ".join(missing))


class ServiceSupervisor:
    def __init__(
        self,
        root: Path,
        specs: tuple[ServiceSpec, ...],
        env: dict[str, str],
        interval: float = 1.0,
        restart_delay: float = 2.0,
    ) -> None:
        self.root = root.resolve()
        self.specs = specs
        self.env = env
        self.interval = interval
        self.restart_delay = restart_delay
        self.processes: dict[str, subprocess.Popen[bytes] | None] = {
            spec.name: None for spec in specs
        }
        self.next_start: dict[str, float] = {spec.name: 0.0 for spec in specs}
        self.stop_requested = False

    def request_stop(self, _signum: int, _frame: FrameType | None) -> None:
        self.stop_requested = True

    def run(self) -> int:
        source_state = {
            spec.name: fingerprint(spec.watch_paths) for spec in self.specs
        }
        migration_path = self.root / "backend/migrations"
        migration_state = fingerprint((migration_path,))
        signal.signal(signal.SIGINT, self.request_stop)
        signal.signal(signal.SIGTERM, self.request_stop)
        signal.signal(signal.SIGHUP, self.request_stop)
        try:
            while not self.stop_requested:
                now = time.monotonic()
                for spec in self.specs:
                    current_state = fingerprint(spec.watch_paths)
                    if current_state != source_state[spec.name]:
                        source_state[spec.name] = current_state
                        self._restart(spec, "source changed")
                    self._ensure_running(spec, now)

                current_migration_state = fingerprint((migration_path,))
                if current_migration_state != migration_state:
                    migration_state = current_migration_state
                    print(
                        "[wms-dev] backend/migrations changed; "
                        "run `just dev-migrate` before relying on the new schema",
                        flush=True,
                    )
                time.sleep(self.interval)
        finally:
            self.stop()
        return 0

    def stop(self) -> None:
        for spec in self.specs:
            self._stop_process(spec)

    def _ensure_running(self, spec: ServiceSpec, now: float) -> None:
        process = self.processes[spec.name]
        if process is not None:
            return_code = process.poll()
            if return_code is None:
                return
            print(
                f"[wms-dev] {spec.name} exited with {return_code}; "
                f"retrying in {self.restart_delay:g}s",
                flush=True,
            )
            self.processes[spec.name] = None
            self.next_start[spec.name] = now + self.restart_delay
        if now >= self.next_start[spec.name]:
            self._start_process(spec)

    def _start_process(self, spec: ServiceSpec) -> None:
        print(f"[wms-dev] starting {spec.name}: {' '.join(spec.command)}", flush=True)
        try:
            self.processes[spec.name] = subprocess.Popen(
                spec.command,
                cwd=self.root,
                env=self.env,
                start_new_session=True,
            )
        except OSError as error:
            print(f"[wms-dev] failed to start {spec.name}: {error}", flush=True)
            self.next_start[spec.name] = time.monotonic() + self.restart_delay

    def _restart(self, spec: ServiceSpec, reason: str) -> None:
        print(f"[wms-dev] restarting {spec.name}: {reason}", flush=True)
        self._stop_process(spec)
        self.next_start[spec.name] = 0.0

    def _stop_process(self, spec: ServiceSpec) -> None:
        process = self.processes[spec.name]
        if process is None or process.poll() is not None:
            self.processes[spec.name] = None
            return
        try:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=5)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=1)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                pass
        self.processes[spec.name] = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--services", default="api,render")
    parser.add_argument("--interval", type=float, default=1.0)
    parser.add_argument("--restart-delay", type=float, default=2.0)
    parser.add_argument("command", choices=("run", "describe"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        names = select_services(args.services)
    except ValueError as error:
        print(f"wms-dev-services: {error}", file=sys.stderr)
        return 2

    specs_by_name = service_specs(args.root)
    specs = tuple(specs_by_name[name] for name in names)
    if args.command == "describe":
        for spec in specs:
            print(f"{spec.name}: {' '.join(spec.command)}")
        return 0
    env = dev_environment()
    try:
        validate_environment(env, names)
    except ValueError as error:
        print(f"wms-dev-services: {error}", file=sys.stderr)
        return 2
    if args.interval <= 0 or args.restart_delay < 0:
        print("wms-dev-services: interval must be positive and restart delay non-negative", file=sys.stderr)
        return 2
    return ServiceSupervisor(args.root, specs, env, args.interval, args.restart_delay).run()


if __name__ == "__main__":
    raise SystemExit(main())
