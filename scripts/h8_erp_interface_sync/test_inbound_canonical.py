"""H8 入站接口表 DTO 到 WMS canonical 命令的边界测试。"""

from __future__ import annotations

import unittest
from types import SimpleNamespace
from unittest.mock import patch

from inbound_canonical import (
    CanonicalMappingError,
    H8CanonicalInboundCommand,
    build_inbound_canonical,
)
from sync_worker import HANDLERS
from test_h8_sync_worker import settings


class TestInboundCanonical(unittest.TestCase):
    def test_asn_adapter_row_becomes_typed_canonical_command(self) -> None:
        row = {
            "id": "message-1",
            "owner_id": "owner-1",
            "warehouse_id": "warehouse-1",
            "external_doc_no": "ASN-1",
            "external_ref": "ERP-ASN-1",
            "supplier_id": "supplier-1",
            "product_code": "P-1",
            "expected_qty": "2",
            "expected_arrival_at": "2026-07-23T00:00:00",
            "document_type": "purchase_inbound",
            "receipt_no": "R-1",
            "schema_version": "1",
            "idempotency_key": "idem-1",
            "retry_count": "0",
            "created_at": "2026-07-22T23:59:00",
        }
        command = build_inbound_canonical(
            "asn",
            row,
            SimpleNamespace(
                connector_id="connector-1",
                config_version=3,
                channel="interface_table",
            ),
        )

        self.assertIsInstance(command, H8CanonicalInboundCommand)
        self.assertEqual(command.external_ref, "ERP-ASN-1")
        self.assertEqual(command.correlation_id, "message-1")
        self.assertEqual(command.connector_id, "connector-1")
        self.assertEqual(command.config_version, 3)
        self.assertEqual(command.occurred_at, "2026-07-22T23:59:00Z")
        self.assertEqual(command.fields["expected_qty"], 2)
        self.assertEqual(
            command.fields["expected_arrival_at"], "2026-07-23T00:00:00Z"
        )
        self.assertNotIn("retry_count", command.fields)
        self.assertNotIn("schema_version", command.fields)

    def test_business_handler_consumes_canonical_not_interface_row(self) -> None:
        command = build_inbound_canonical(
            "asn",
            {
                "id": "message-1",
                "owner_id": "owner-1",
                "warehouse_id": "warehouse-1",
                "external_doc_no": "ASN-1",
                "external_ref": "ERP-ASN-1",
                "supplier_id": "supplier-1",
                "product_code": "P-1",
                "expected_qty": "2",
                "expected_arrival_at": "2026-07-23T00:00:00",
                "document_type": "purchase_inbound",
                "receipt_no": "R-1",
                "idempotency_key": "idem-1",
                "created_at": "2026-07-22T23:59:00",
            },
            SimpleNamespace(
                connector_id="connector-1",
                config_version=3,
                channel="interface_table",
            ),
        )

        with patch(
            "sync_worker.http_json",
            return_value=(201, {"id": "receiving-1"}, ""),
        ) as business_api:
            result = HANDLERS["asn"][1](settings(), command)

        self.assertEqual(result, "receiving-1")
        body = business_api.call_args.args[3]
        self.assertEqual(body["external_ref"], "ERP-ASN-1")
        self.assertEqual(body["lines"][0]["expected_qty"], 2)
        self.assertNotIn("retry_count", body)

    def test_unmapped_product_value_is_rejected_during_conversion(self) -> None:
        with self.assertRaises(CanonicalMappingError) as caught:
            build_inbound_canonical(
                "product_master",
                {
                    "id": "message-1",
                    "owner_id": "owner-1",
                    "external_doc_no": "PRODUCT-1",
                    "product_code": "P-1",
                    "product_name": "药品一",
                    "storage_condition": "ERP_UNKNOWN",
                    "idempotency_key": "idem-1",
                    "created_at": "2026-07-22T23:59:00",
                },
                SimpleNamespace(
                    connector_id="connector-1",
                    config_version=3,
                    channel="interface_table",
                ),
            )
        self.assertEqual(caught.exception.status, 422)


if __name__ == "__main__":
    unittest.main()
