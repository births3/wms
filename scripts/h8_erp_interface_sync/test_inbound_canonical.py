"""H8 入站接口表 DTO 到 WMS canonical 命令的边界测试。"""

from __future__ import annotations

import json
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from inbound_canonical import (
    CanonicalMappingError,
    H8CanonicalInboundCommand,
    build_inbound_canonical,
)
from sync_worker import HANDLERS, build_inbound_canonical_with_mpm
from test_h8_sync_worker import settings


class TestInboundCanonical(unittest.TestCase):
    def test_product_master_requires_non_blank_spec_before_business_api(self) -> None:
        base = {
            "id": "product-no-spec",
            "owner_id": "owner-1",
            "external_doc_no": "product-no-spec",
            "product_code": "P-NO-SPEC",
            "product_name": "缺规格药品",
            "special_drug_category": "普通药品",
            "storage_condition": "常温",
            "packaging_json": json.dumps(
                [
                    {
                        "unit": "盒",
                        "ratio_to_base": 1,
                        "is_base": True,
                        "is_default": True,
                        "sort_order": 1,
                    }
                ],
                ensure_ascii=False,
            ),
            "idempotency_key": "product-no-spec",
            "created_at": "2026-07-23T00:00:00",
        }

        for spec in (None, "", "   "):
            with self.subTest(spec=spec):
                row = dict(base)
                if spec is not None:
                    row["spec"] = spec
                with self.assertRaisesRegex(CanonicalMappingError, "invalid spec"):
                    build_inbound_canonical("product_master", row, None)

    def test_product_fields_are_left_raw_for_shared_rest_anticorruption(self) -> None:
        calls: list[dict] = []

        def fake_http(_settings, _method, _path, body, _idempotency_key):
            calls.append(body)
            raise AssertionError("product master mapping belongs to H8 REST boundary")

        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_master",
            {
                "id": "product-raw",
                "owner_id": "owner-1",
                "external_doc_no": "product-raw",
                "product_code": "P-RAW",
                "product_name": "药品",
                "spec": "10mg*30片",
                "dosage_form": "薄膜衣片",
                "special_drug_category": "普通药品",
                "storage_condition": "2-8℃避光保存",
                "packaging_json": json.dumps(
                    [
                        {
                            "unit": "盒",
                            "ratio_to_base": 1,
                            "is_base": True,
                            "is_default": True,
                            "sort_order": 1,
                        }
                    ],
                    ensure_ascii=False,
                ),
                "idempotency_key": "product-raw",
                "created_at": "2026-07-23T00:00:00",
            },
            None,
            http_json_fn=fake_http,
        )
        self.assertEqual(command.fields["dosage_form"], "薄膜衣片")
        self.assertEqual(command.fields["storage_condition"], "2-8℃避光保存")
        self.assertEqual(command.fields["packaging_levels"][0]["unit"], "盒")
        self.assertEqual(calls, [])

    def test_product_change_dosage_form_is_preserved_for_shared_h8_rest(self) -> None:
        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_change",
            {
                "id": "message-dosage-1",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-CHANGE-DOSAGE-1",
                "product_id": "product-1",
                "product_code": "P-1",
                "field_name": "dosage_form",
                "new_value": "普通片",
                "idempotency_key": "idem-dosage-1",
                "created_at": "2026-07-23T00:00:00",
            },
            None,
            http_json_fn=lambda *_args: (
                200,
                {"status": "matched", "target_value": "片剂"},
                "",
            ),
        )

        self.assertEqual(command.fields["new_value"], "普通片")
        with patch(
            "sync_worker.http_json",
            return_value=(200, {"wms_resource_id": "product-1"}, ""),
        ) as business_api:
            HANDLERS["product_change"][1](settings(), command)
        self.assertEqual(
            business_api.call_args.args[2],
            "/api/v1/integration/erp-messages/inbound/product_change",
        )
        self.assertEqual(
            business_api.call_args.args[3]["new_value"],
            "普通片",
        )

    def test_product_change_physical_dimensions_are_sent_as_one_object(self) -> None:
        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_change",
            {
                "id": "message-dimensions-1",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-CHANGE-DIMENSIONS-1",
                "product_id": "product-1",
                "product_code": "P-1",
                "field_name": "physical_dimensions",
                "new_value": json.dumps(
                    {"length_mm": 120.5, "width_mm": 45, "height_mm": 30.25}
                ),
                "idempotency_key": "idem-dimensions-1",
                "created_at": "2026-07-23T00:00:00",
            },
            None,
            http_json_fn=lambda *_args: (200, {}, ""),
        )

        self.assertEqual(
            command.fields["physical_dimensions"],
            {"length_mm": 120.5, "width_mm": 45.0, "height_mm": 30.25},
        )
        self.assertNotIn("new_value", command.fields)
        with patch(
            "sync_worker.http_json",
            return_value=(200, {"wms_resource_id": "product-1"}, ""),
        ) as business_api:
            HANDLERS["product_change"][1](settings(), command)
        body = business_api.call_args.args[3]
        self.assertEqual(body["physical_dimensions"], command.fields["physical_dimensions"])
        self.assertNotIn("new_value", body)

    def test_product_change_rejects_partial_physical_dimensions(self) -> None:
        with self.assertRaisesRegex(
            CanonicalMappingError, "invalid physical_dimensions"
        ):
            build_inbound_canonical_with_mpm(
                settings(),
                "product_change",
                {
                    "id": "message-dimensions-partial",
                    "owner_id": "owner-1",
                    "external_doc_no": "PRODUCT-CHANGE-DIMENSIONS-PARTIAL",
                    "product_code": "P-1",
                    "field_name": "physical_dimensions",
                    "new_value": json.dumps({"length_mm": 120.5, "width_mm": 45}),
                    "idempotency_key": "idem-dimensions-partial",
                    "created_at": "2026-07-23T00:00:00",
                },
                None,
                http_json_fn=lambda *_args: (200, {}, ""),
            )

    def test_product_change_special_drug_category_is_preserved_before_shared_rest(
        self,
    ) -> None:
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return (
                200,
                {
                    "status": "matched",
                    "target_value": "narcotic",
                    "rule_id": "rule-special-drug-category",
                },
                "",
            )

        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_change",
            {
                "id": "message-special-1",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-CHANGE-SPECIAL-1",
                "product_id": "product-1",
                "product_code": "P-1",
                "field_name": "special_drug_category",
                "new_value": "麻醉药品",
                "idempotency_key": "idem-special-1",
                "created_at": "2026-07-23T00:00:00",
            },
            SimpleNamespace(
                connector_id="connector-1",
                connector_code="SELF-ERP",
                config_version=3,
                channel="interface_table",
            ),
            http_json_fn=fake_http,
        )

        self.assertEqual(command.fields["new_value"], "麻醉药品")
        self.assertEqual(calls, [])

    def test_product_change_storage_condition_is_preserved_before_shared_rest(self) -> None:
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return (
                200,
                {
                    "status": "matched",
                    "target_value": "cold",
                    "rule_id": "rule-storage-condition",
                    "confidence": 100,
                    "fallback_used": False,
                    "queued": False,
                },
                "",
            )

        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_change",
            {
                "id": "message-2",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-CHANGE-1",
                "product_code": "P-1",
                "field_name": "storage_condition",
                "new_value": "2-8℃避光保存",
                "idempotency_key": "idem-2",
                "created_at": "2026-07-22T23:59:00",
            },
            SimpleNamespace(
                connector_id="connector-1",
                connector_code="SELF-ERP",
                config_version=3,
                channel="interface_table",
            ),
            http_json_fn=fake_http,
        )

        self.assertEqual(command.fields["new_value"], "2-8℃避光保存")
        self.assertEqual(calls, [])

    def test_product_change_status_is_preserved_before_shared_rest(self) -> None:
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return (
                200,
                {
                    "status": "matched",
                    "target_value": "disabled",
                    "rule_id": "rule-product-status",
                },
                "",
            )

        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_change",
            {
                "id": "message-status-1",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-CHANGE-STATUS-1",
                "product_id": "product-1",
                "product_code": "P-1",
                "field_name": "status",
                "new_value": "停用",
                "idempotency_key": "idem-status-1",
                "created_at": "2026-07-23T00:00:00",
            },
            None,
            http_json_fn=fake_http,
        )

        self.assertEqual(command.fields["new_value"], "停用")
        self.assertEqual(calls, [])

    def test_asn_document_type_is_resolved_by_persistent_mpm_api(self) -> None:
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            return (
                200,
                {
                    "status": "matched",
                    "target_value": "purchase_inbound",
                    "rule_id": "rule-document-type",
                    "confidence": 100,
                    "fallback_used": False,
                    "queued": False,
                },
                "",
            )

        command = build_inbound_canonical_with_mpm(
            settings(),
            "asn",
            {
                "id": "message-1",
                "owner_id": "owner-1",
                "warehouse_id": "warehouse-1",
                "external_doc_no": "ASN-1",
                "supplier_id": "supplier-1",
                "product_code": "P-1",
                "expected_qty": "2",
                "document_type": "采购入库",
                "idempotency_key": "idem-1",
                "created_at": "2026-07-22T23:59:00",
            },
            SimpleNamespace(
                connector_id="connector-1",
                connector_code="SELF-ERP",
                config_version=3,
                channel="interface_table",
            ),
            http_json_fn=fake_http,
        )

        self.assertEqual(command.fields["document_type"], "purchase_inbound")
        self.assertEqual(calls[0][2]["dict_code"], "document_type")
        self.assertEqual(calls[0][2]["source_value"], "采购入库")
        self.assertEqual(calls[0][3], "idem-1:mpm:document_type")

    def test_product_interface_row_builds_complete_typed_rest_contract(self) -> None:
        calls: list[tuple[str, str, dict, str]] = []

        def fake_http(_settings, method, path, body, idempotency_key):
            calls.append((method, path, body, idempotency_key))
            raise AssertionError("product master mapping belongs to H8 REST boundary")

        command = build_inbound_canonical_with_mpm(
            settings(),
            "product_master",
            {
                "id": "message-1",
                "owner_id": "owner-1",
                "external_doc_no": "PRODUCT-1",
                "product_code": "P-1",
                "product_name": "药品一",
                "spec": "10mg*30片",
                "special_drug_category": "普通药品",
                "storage_condition": "2-8℃避光保存",
                "udi_code": "06912345678901",
                "electronic_regulatory_code": "REG-001",
                "length_mm": "120.0",
                "width_mm": "80.0",
                "height_mm": "50.0",
                "volume_cm3": "",
                "weight_g": "350.5",
                "packaging_json": json.dumps(
                    [
                        {
                            "unit": "支",
                            "ratio_to_base": 1,
                            "is_base": True,
                            "is_default": False,
                            "sort_order": 1,
                        },
                        {
                            "unit": "盒",
                            "ratio_to_base": 12,
                            "is_base": False,
                            "is_default": True,
                            "sort_order": 2,
                        },
                    ],
                    ensure_ascii=False,
                ),
                "idempotency_key": "idem-1",
                "created_at": "2026-07-22T23:59:00",
            },
            SimpleNamespace(
                connector_id="connector-1",
                connector_code="SELF-ERP",
                config_version=3,
                channel="interface_table",
            ),
            http_json_fn=fake_http,
        )

        self.assertEqual(command.fields["storage_condition"], "2-8℃避光保存")
        self.assertEqual(command.fields["special_drug_category"], "普通药品")
        self.assertEqual(command.fields["length_mm"], 120.0)
        self.assertIsNone(command.fields["volume_cm3"])
        self.assertEqual(command.fields["packaging_levels"][1]["ratio_to_base"], 12)
        self.assertEqual(calls, [])

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
