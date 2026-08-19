"""OpenAPI 同步脚本治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _patch_openapi_sync_paths(tmp_path, monkeypatch, check, *, document=None):
    import json

    backend = tmp_path / "backend"
    backend.mkdir()
    openapi = tmp_path / "openapi.json"
    openapi_text = (
        '{"openapi":"3.1.0","paths":{},"components":{"schemas":{}}}'
        if document is None
        else json.dumps(document)
    )
    openapi.write_text(openapi_text, encoding="utf-8")
    schema = tmp_path / "schema.ts"
    schema.write_text("export type paths = {};\n", encoding="utf-8")
    portal_openapi = tmp_path / "customer-portal-openapi.yaml"
    portal_openapi.write_text("openapi: 3.0.3\npaths: {}\n", encoding="utf-8")
    portal_schema = tmp_path / "customer-portal-schema.ts"
    portal_schema.write_text("export interface paths {}\n", encoding="utf-8")

    monkeypatch.setattr(check, "BACKEND_DIR", backend)
    monkeypatch.setattr(check, "SHARED_OPENAPI", openapi)
    monkeypatch.setattr(check, "API_CLIENT_SCHEMA", schema)
    monkeypatch.setattr(check, "PORTAL_OPENAPI", portal_openapi, raising=False)
    monkeypatch.setattr(check, "PORTAL_SCHEMA", portal_schema, raising=False)

    return backend, openapi, schema, portal_openapi, portal_schema


def test_openapi_in_sync_strict_cargo_timeout_fails(tmp_path, monkeypatch, capsys):
    """严格模式下 cargo 超时不能被当作同步通过。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    _patch_openapi_sync_paths(tmp_path, monkeypatch, check)

    def fake_run(cmd, **kwargs):
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--strict", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 2
    assert payload["ok"] is False
    assert payload["status"] == "cargo_unavailable"


def test_openapi_in_sync_non_strict_timeout_is_explicitly_degraded(tmp_path, monkeypatch, capsys):
    """非严格模式仍保留本地降级语义，但 JSON 状态必须明确标注未验证。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    _patch_openapi_sync_paths(tmp_path, monkeypatch, check)

    def fake_run(cmd, **kwargs):
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 0
    assert payload["ok"] is True
    assert payload["status"] == "cargo_unavailable"


def test_openapi_in_sync_strict_schema_timeout_fails(tmp_path, monkeypatch, capsys):
    """严格模式下 schema.ts 生成超时必须阻塞。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    document = {"openapi": "3.1.0", "paths": {}, "components": {"schemas": {}}}
    _patch_openapi_sync_paths(tmp_path, monkeypatch, check, document=document)

    def fake_run(cmd, **kwargs):
        if cmd[0] == "cargo":
            return subprocess.CompletedProcess(cmd, 0, stdout=json.dumps(document), stderr="")
        raise subprocess.TimeoutExpired(cmd=cmd, timeout=kwargs["timeout"])

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--strict", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 2
    assert payload["ok"] is False
    assert payload["status"] == "schema_generator_unavailable"


def test_openapi_in_sync_checks_customer_portal_schema(tmp_path, monkeypatch, capsys):
    """主 WMS 契约通过时，客户门户 schema 也必须参与同步校验。"""
    import json
    import subprocess
    import check_openapi_in_sync as check

    document = {"openapi": "3.1.0", "paths": {}, "components": {"schemas": {}}}
    _, _, api_schema, portal_openapi, portal_schema = _patch_openapi_sync_paths(
        tmp_path, monkeypatch, check, document=document
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    api_schema.write_text("generated main schema\n", encoding="utf-8")
    portal_schema.write_text("generated portal schema\n", encoding="utf-8")

    def fake_run(cmd, **kwargs):
        if cmd[0] == "cargo":
            return subprocess.CompletedProcess(cmd, 0, stdout=json.dumps(document), stderr="")
        output = Path(cmd[cmd.index("--output") + 1])
        output.write_text(
            "generated portal schema\n"
            if any("customer-portal-openapi.yaml" in str(arg) for arg in cmd)
            else "generated main schema\n",
            encoding="utf-8",
        )
        return subprocess.CompletedProcess(cmd, 0, stdout="", stderr="")

    monkeypatch.setattr(check.subprocess, "run", fake_run)

    exit_code = check.main(["--strict", "--json"])
    payload = json.loads(capsys.readouterr().out)

    assert exit_code == 0
    assert payload["ok"] is True
    assert payload["portal_openapi_path"] == str(portal_openapi.relative_to(check.REPO_ROOT))
    assert payload["portal_schema_path"] == str(portal_schema.relative_to(check.REPO_ROOT))
