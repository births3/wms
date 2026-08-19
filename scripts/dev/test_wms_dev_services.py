from __future__ import annotations

import importlib.util
import signal
import sys
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("wms_dev_services.py")
SPEC = importlib.util.spec_from_file_location("wms_dev_services", MODULE_PATH)
assert SPEC and SPEC.loader
wms_dev_services = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = wms_dev_services
SPEC.loader.exec_module(wms_dev_services)


def test_fingerprint_detects_source_changes_but_ignores_generated_files(tmp_path: Path) -> None:
    backend = tmp_path / "backend"
    source = backend / "crates" / "api" / "src" / "main.rs"
    generated = backend / "target" / "debug" / "wms-api"
    source.parent.mkdir(parents=True)
    generated.parent.mkdir(parents=True)
    source.write_text("fn main() {}\n", encoding="utf-8")
    generated.write_text("old\n", encoding="utf-8")

    before = wms_dev_services.fingerprint((backend,))
    generated.write_text("new generated output\n", encoding="utf-8")
    assert wms_dev_services.fingerprint((backend,)) == before

    source.write_text("fn main() { println!(\"changed\"); }\n", encoding="utf-8")
    assert wms_dev_services.fingerprint((backend,)) != before


def test_service_specs_use_host_commands_and_watch_their_sources(tmp_path: Path) -> None:
    specs = wms_dev_services.service_specs(tmp_path)

    assert set(specs) == {"api", "render", "h8"}
    assert specs["api"].command[:3] == ("cargo", "run", "--manifest-path")
    assert specs["render"].command[:3] == ("pnpm", "--dir", "apps/h9-render-worker")
    assert specs["h8"].command[:3] == ("cargo", "run", "--manifest-path")
    assert all("docker" not in part for spec in specs.values() for part in spec.command)
    assert tmp_path / "backend/crates" in specs["api"].watch_paths
    assert tmp_path / "backend/migrations" not in specs["api"].watch_paths
    assert tmp_path / "apps/h9-render-worker" in specs["render"].watch_paths


def test_dev_environment_quotes_database_password_and_sets_local_endpoints() -> None:
    env = wms_dev_services.dev_environment(
        {
            "WMS_DEV_H2_DB_PASSWORD": "p@ss word",
            "WMS_DEV_H2_DB_PORT": "15432",
            "WMS_DEV_H2_REDIS_PORT": "16379",
            "WMS_DEV_HFILE_API_PORT": "19000",
            "WMS_DEV_H9_RENDER_PORT": "18090",
        }
    )

    assert env["WMS_DB_URL"].startswith("postgres://wms_dev_h2:p%40ss%20word@")
    assert env["WMS_REDIS_URL"] == "redis://127.0.0.1:16379"
    assert env["WMS_H9_RENDER_WORKER_URL"] == "http://127.0.0.1:18090/render"


def test_select_services_rejects_unknown_names() -> None:
    try:
        wms_dev_services.select_services("api,unknown")
    except ValueError as error:
        assert "unknown" in str(error)
    else:
        raise AssertionError("unknown service must be rejected")

    try:
        wms_dev_services.select_services("all,unknown")
    except ValueError as error:
        assert "unknown" in str(error)
    else:
        raise AssertionError("unknown service must be rejected with all")


def test_supervisor_handles_hangup_for_clean_shutdown(tmp_path: Path, monkeypatch) -> None:
    registered_signals: list[int] = []
    supervisor = wms_dev_services.ServiceSupervisor(tmp_path, (), {}, interval=0.01)
    monkeypatch.setattr(
        wms_dev_services.signal,
        "signal",
        lambda signum, _handler: registered_signals.append(signum),
    )
    monkeypatch.setattr(
        wms_dev_services.time,
        "sleep",
        lambda _interval: supervisor.request_stop(signal.SIGHUP, None),
    )

    assert supervisor.run() == 0
    assert signal.SIGHUP in registered_signals
