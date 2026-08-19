import importlib.util
from pathlib import Path
import sys


SCRIPT = Path(__file__).parents[1] / "check_redis_usage_inventory.py"
SPEC = importlib.util.spec_from_file_location("check_redis_usage_inventory", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules["check_redis_usage_inventory"] = MODULE
SPEC.loader.exec_module(MODULE)


def test_repository_redis_inventory_is_registered():
    issues, stats = MODULE.check_inventory()

    assert issues == []
    assert stats["manifest_entries"] == stats["discovered_paths"]
    assert stats["registered_paths"] == stats["discovered_paths"]


def test_missing_required_anchor_is_reported(tmp_path):
    manifest = tmp_path / "inventory.toml"
    source = tmp_path / "source.rs"
    source.write_text("let value = redis;\n", encoding="utf-8")
    manifest.write_text(
        '[[entry]]\npath = "source.rs"\ncategory = "test"\nrequired_terms = ["WMS_REDIS_URL"]\n',
        encoding="utf-8",
    )

    issues, _ = MODULE.check_inventory(tmp_path, manifest)

    assert any(issue.kind == "missing_term" for issue in issues)
