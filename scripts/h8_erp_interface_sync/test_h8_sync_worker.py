"""H8 worker 纯逻辑单测（无 Docker / 无 DB）。"""

from __future__ import annotations

import json
import unittest

from outbound_publish import (
    OUTBOX_SOURCES,
    OutboxRow,
    insert_if_out_sql,
    sql_escape_mssql,
)


class TestInsertIfOutSql(unittest.TestCase):
    def test_idempotent_insert_contains_source_unique(self) -> None:
        row = OutboxRow(
            table="receiving_putaway_erp_feedback_outbox",
            id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            owner_id="11111111-1111-1111-1111-111111111111",
            event_type="inbound_putaway_completed",
            payload={"qty": 1, "note": "it's ok"},
            external_ref="rcv-1",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inbound-complete",
        )
        sql = insert_if_out_sql(row)
        self.assertIn("if_out_message", sql)
        self.assertIn("source_outbox_table", sql)
        self.assertIn("inbound_putaway_completed", sql)
        self.assertIn("it''s ok", sql)  # MSSQL escape
        self.assertIn("out:receiving_putaway_erp_feedback_outbox:", sql)

    def test_escape_mssql(self) -> None:
        self.assertEqual(sql_escape_mssql("a'b"), "a''b")

    def test_outbox_sources_include_archive_and_recon(self) -> None:
        tables = {s["table"] for s in OUTBOX_SOURCES}
        self.assertIn("archive_revision_erp_feedback_outbox", tables)
        self.assertIn("reconciliation_erp_feedback_outbox", tables)
        self.assertIn("shipment_confirm_erp_feedback_outbox", tables)


class TestPayloadJson(unittest.TestCase):
    def test_roundtrip_payload(self) -> None:
        payload = {"product_code": "P1", "qty": 10, "note": "a|b"}
        raw = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
        self.assertEqual(json.loads(raw)["note"], "a|b")
        self.assertNotIn("\n", raw)

    def test_insert_sql_escapes_pipe_payload(self) -> None:
        row = OutboxRow(
            table="inventory_status_erp_feedback_outbox",
            id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            owner_id="11111111-1111-1111-1111-111111111111",
            event_type="inventory_status_changed",
            payload={"reason": "a|b", "qty": 1},
            external_ref="x",
            attempt_count=1,
            max_attempts=5,
            deadline_at=None,
            callback_path="/inventory-status",
        )
        sql = insert_if_out_sql(row)
        self.assertIn("a|b", sql)
        self.assertNotIn("\n  INSERT", sql.split("IF NOT EXISTS")[0])  # payload line compact


if __name__ == "__main__":
    unittest.main()
