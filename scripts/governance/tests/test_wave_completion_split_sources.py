from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave2_static_items_follow_split_production_sources() -> None:
    import report_wave2_completion as report

    items = {item.item_id: item for item in report.collect_items()}

    assert items["W2.A"].complete, items["W2.A"].gaps
    assert items["W2.B"].complete, items["W2.B"].gaps


def test_wave3_backend_and_key_path_follow_split_production_sources() -> None:
    import report_wave3_completion as report

    items = {item.item_id: item for item in report.collect_items()}
    layers = {layer.layer_id: layer for layer in report.collect_key_path_layers()}

    assert items["W3.A-backend"].complete, items["W3.A-backend"].gaps
    assert items["W3.B-backend"].complete, items["W3.B-backend"].gaps
    for layer_id in ("L3", "L4", "L5", "L8", "L10", "L11"):
        assert layers[layer_id].complete, layers[layer_id].gaps


def test_wave3_reader_follows_nested_test_fragments(monkeypatch, tmp_path: Path) -> None:
    import report_wave3_completion as report

    tests = tmp_path / "backend/crates/api/tests"
    (tests / "wave3_postgres").mkdir(parents=True)
    (tests / "wave3_postgres.rs").write_text("include fragment", encoding="utf-8")
    (tests / "wave3_postgres/wave3_postgres_part2.rs").write_text(
        "concurrent_same_idempotency_key_replays_first_receipt tokio::join!",
        encoding="utf-8",
    )
    monkeypatch.setattr(report, "REPO_ROOT", tmp_path)

    assert report.file_contains(
        "backend/crates/api/tests/wave3_postgres.rs",
        "concurrent_same_idempotency_key_replays_first_receipt",
        "tokio::join!",
    )
