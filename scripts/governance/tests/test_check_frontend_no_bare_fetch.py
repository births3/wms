"""前端裸 fetch 门禁测试。"""
import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "check_frontend_no_bare_fetch.py"
SPEC = importlib.util.spec_from_file_location("check_frontend_no_bare_fetch", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def test_scan_accepts_typed_client_source(tmp_path):
    source = tmp_path / "client.ts"
    source.write_text("export const request = () => api.GET('/orders');\n", encoding="utf-8")

    assert MODULE.scan([source]) == []


def test_scan_rejects_bare_fetch(tmp_path):
    source = tmp_path / "page.tsx"
    source.write_text("await fetch('/api/v1/orders');\n", encoding="utf-8")

    violations = MODULE.scan([source])

    assert violations == [{"path": str(source), "line": 1, "text": "await fetch('/api/v1/orders');"}]


def test_scan_honors_shared_client_allowlist(tmp_path):
    source = tmp_path / "client.ts"
    source.write_text("await fetch('/api/v1/orders');\n", encoding="utf-8")

    assert MODULE.scan([source], allowlist={source}) == []
