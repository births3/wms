"""AR-09: staging API startup must not depend on the print renderer."""
from pathlib import Path
import re


COMPOSE = Path("deploy/docker-compose.staging.yml")
SMOKE = Path("scripts/h9_render_worker_compose_smoke.sh")
JUSTFILE = Path("justfile")
RUNBOOK = Path("docs/runbooks/h9-render-worker-compose-smoke.md")


def test_staging_api_does_not_wait_for_render_worker_health():
    compose = COMPOSE.read_text(encoding="utf-8")
    api = re.search(r"(?ms)^  wms-api-staging:\n(.*?)(?=^  [a-z0-9-]+:|\Z)", compose)
    assert api, "wms-api-staging service must remain declared"
    depends_on = re.search(r"(?ms)^    depends_on:\n(.*?)(?=^    [a-z_]+:|\Z)", api.group(1))
    assert depends_on, "API dependencies must remain explicit"
    assert "h9-render-worker-staging" not in depends_on.group(1)
    assert "h9-render-worker-staging:" in compose
    assert "WMS_H9_RENDER_WORKER_URL=http://h9-render-worker-staging:18090/render" in api.group(1)


def test_compose_smoke_isolated_and_fail_closed():
    smoke = SMOKE.read_text(encoding="utf-8")
    for token in (
        "COMPOSE_PROJECT_NAME",
        "docker compose",
        "h9-render-worker-staging",
        "H9_CATEGORY_PDF_RENDER_FAILED",
        "healthz",
        "Authorization",
        "wrong",
        "down -v",
    ):
        assert token in smoke


def test_compose_smoke_checks_persisted_failure_and_recovery_state():
    smoke = SMOKE.read_text(encoding="utf-8")
    runbook = RUNBOOK.read_text(encoding="utf-8")
    for token in ("list_url", "preparation_status", "processing_status", '"failed"', '"completed"'):
        assert token in smoke
    assert "持久化失败状态" in runbook


def test_smoke_entrypoint_and_runbook_are_registered():
    justfile = JUSTFILE.read_text(encoding="utf-8")
    runbook = RUNBOOK.read_text(encoding="utf-8")
    assert "h9-render-worker-compose-smoke:" in justfile
    assert "just h9-render-worker-compose-smoke" in runbook
    assert "非打印核心接口" in runbook
    assert "H9_CATEGORY_PDF_RENDER_FAILED" in runbook
    assert "down -v" in runbook
