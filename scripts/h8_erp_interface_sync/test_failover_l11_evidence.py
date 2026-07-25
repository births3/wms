import unittest
from types import SimpleNamespace

import run_failover_l11_evidence
from outbound_publish import OUTBOX_SOURCES


class TestFailoverL11Evidence(unittest.TestCase):
    def test_catalog_messages_each_keep_one_interface_row_after_failover(self) -> None:
        row_counts: dict[str, int] = {}

        def fake_sqlcmd(_settings, sql):
            for source in OUTBOX_SOURCES:
                table = source["table"]
                if f"source_outbox_table = N'{table}'" not in sql:
                    continue
                if "IF NOT EXISTS" in sql:
                    row_counts[table] = 1
                if "SELECT COUNT(1)" in sql:
                    return str(row_counts.get(table, 0))
            return ""

        result = run_failover_l11_evidence.collect_catalog(
            settings=SimpleNamespace(),
            sqlcmd_fn=fake_sqlcmd,
        )

        self.assertTrue(result["ok"])
        self.assertEqual(
            [check["message_type"] for check in result["message_checks"]],
            [source["message_type"] for source in OUTBOX_SOURCES],
        )
        self.assertTrue(
            all(check["interface_row_count"] == 1 for check in result["message_checks"])
        )

    def test_same_key_repeated_failover_keeps_one_interface_row(self) -> None:
        row_count = 0

        def fake_sqlcmd(_settings, sql):
            nonlocal row_count
            if "IF NOT EXISTS" in sql:
                row_count = 1
            if "SELECT COUNT(1)" in sql:
                return str(row_count)
            return ""

        result = run_failover_l11_evidence.collect(
            settings=SimpleNamespace(),
            sqlcmd_fn=fake_sqlcmd,
            source_id="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        )

        self.assertTrue(result["ok"])
        self.assertEqual(result["interface_row_count"], 1)
        self.assertEqual(result["rest_attempts"], 4)
        self.assertEqual(result["table_attempts"], 2)
        self.assertEqual(result["channels"], ["table_fallback", "table_fallback"])

    def test_duplicate_interface_rows_fail_the_check(self) -> None:
        def fake_sqlcmd(_settings, sql):
            return "2" if "SELECT COUNT(1)" in sql else ""

        result = run_failover_l11_evidence.collect(
            settings=SimpleNamespace(),
            sqlcmd_fn=fake_sqlcmd,
            source_id="bbbbbbbb-bbbb-cccc-dddd-eeeeeeeeeeee",
        )

        self.assertFalse(result["ok"])
        self.assertEqual(result["interface_row_count"], 2)


if __name__ == "__main__":
    unittest.main()
