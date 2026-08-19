"""H8 通道 A 两个本地 Mock 的 Bearer 认证回归。"""

from __future__ import annotations

import importlib.util
import json
import unittest
import urllib.error
import urllib.request
from pathlib import Path
from tempfile import TemporaryDirectory
from threading import Thread

import channel_a_callback_mock as CALLBACK
from channel_a_callback_mock import bearer_authorized as callback_authorized

ROOT = Path(__file__).resolve().parents[2]
VENDOR_PATH = ROOT / "deploy" / "h8-erp-vendor" / "server.py"
SPEC = importlib.util.spec_from_file_location("h8_erp_vendor_server", VENDOR_PATH)
assert SPEC and SPEC.loader
VENDOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VENDOR)


class CallbackMockAuthTest(unittest.TestCase):
    def test_unauthenticated_posts_have_no_state_or_file_side_effects(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)

            previous_vendor = (
                VENDOR.BEARER_TOKEN,
                VENDOR.ADMIN_TOKEN,
                VENDOR.DATA_DIR,
                VENDOR.LOG_PATH,
                VENDOR._fail_remaining,
                list(VENDOR._receipts),
            )
            VENDOR.BEARER_TOKEN = "expected"
            VENDOR.ADMIN_TOKEN = ""
            VENDOR.DATA_DIR = root / "vendor"
            VENDOR.LOG_PATH = VENDOR.DATA_DIR / "receipts.jsonl"
            VENDOR._fail_remaining = 7
            VENDOR._receipts[:] = [{"sentinel": True}]
            vendor_server = VENDOR.ThreadingHTTPServer(
                ("127.0.0.1", 0), VENDOR.Handler
            )
            vendor_thread = Thread(target=vendor_server.serve_forever, daemon=True)
            vendor_thread.start()
            try:
                for path in ("/shipment-confirm", "/_admin/fail-count"):
                    request = urllib.request.Request(
                        f"http://127.0.0.1:{vendor_server.server_port}{path}",
                        data=json.dumps({"count": 99}).encode(),
                        method="POST",
                        headers={"Content-Type": "application/json"},
                    )
                    with self.assertRaises(urllib.error.HTTPError) as raised:
                        urllib.request.urlopen(request, timeout=2)
                    self.assertEqual(raised.exception.code, 401)
                self.assertEqual(VENDOR._receipts, [{"sentinel": True}])
                self.assertEqual(VENDOR._fail_remaining, 7)
                self.assertFalse(VENDOR.LOG_PATH.exists())
            finally:
                vendor_server.shutdown()
                vendor_server.server_close()
                vendor_thread.join(timeout=2)
                (
                    VENDOR.BEARER_TOKEN,
                    VENDOR.ADMIN_TOKEN,
                    VENDOR.DATA_DIR,
                    VENDOR.LOG_PATH,
                    VENDOR._fail_remaining,
                    receipts,
                ) = previous_vendor
                VENDOR._receipts[:] = receipts

            previous_callback = (
                CALLBACK._BEARER_TOKEN,
                CALLBACK._LOG_PATH,
                list(CALLBACK.RECEIVED),
            )
            CALLBACK._BEARER_TOKEN = "expected"
            CALLBACK._LOG_PATH = root / "callback.jsonl"
            CALLBACK.RECEIVED[:] = [{"sentinel": True}]
            callback_server = CALLBACK.ThreadingHTTPServer(
                ("127.0.0.1", 0), CALLBACK.Handler
            )
            callback_thread = Thread(
                target=callback_server.serve_forever, daemon=True
            )
            callback_thread.start()
            try:
                request = urllib.request.Request(
                    f"http://127.0.0.1:{callback_server.server_port}/shipment-confirm",
                    data=b"{}",
                    method="POST",
                    headers={"Content-Type": "application/json"},
                )
                with self.assertRaises(urllib.error.HTTPError) as raised:
                    urllib.request.urlopen(request, timeout=2)
                self.assertEqual(raised.exception.code, 401)
                self.assertEqual(CALLBACK.RECEIVED, [{"sentinel": True}])
                self.assertFalse(CALLBACK._LOG_PATH.exists())
            finally:
                callback_server.shutdown()
                callback_server.server_close()
                callback_thread.join(timeout=2)
                CALLBACK._BEARER_TOKEN, CALLBACK._LOG_PATH, received = (
                    previous_callback
                )
                CALLBACK.RECEIVED[:] = received

    def test_channel_a_callback_requires_exact_bearer(self) -> None:
        self.assertTrue(callback_authorized("Bearer expected", "expected"))
        self.assertFalse(callback_authorized(None, "expected"))
        self.assertFalse(callback_authorized("Bearer wrong", "expected"))
        self.assertFalse(callback_authorized("Basic expected", "expected"))

    def test_container_vendor_requires_exact_bearer(self) -> None:
        self.assertTrue(VENDOR.bearer_authorized("Bearer expected", "expected"))
        self.assertFalse(VENDOR.bearer_authorized(None, "expected"))
        self.assertFalse(VENDOR.bearer_authorized("Bearer wrong", "expected"))
        self.assertFalse(VENDOR.bearer_authorized("Basic expected", "expected"))

    def test_both_health_endpoints_reject_missing_bearer(self) -> None:
        for module in (CALLBACK, VENDOR):
            token_name = "_BEARER_TOKEN" if module is CALLBACK else "BEARER_TOKEN"
            previous = getattr(module, token_name)
            setattr(module, token_name, "expected")
            server = module.ThreadingHTTPServer(("127.0.0.1", 0), module.Handler)
            thread = Thread(target=server.serve_forever, daemon=True)
            thread.start()
            url = f"http://127.0.0.1:{server.server_port}/healthz"
            try:
                with self.assertRaises(urllib.error.HTTPError) as raised:
                    urllib.request.urlopen(url, timeout=2)
                self.assertEqual(raised.exception.code, 401)
                request = urllib.request.Request(
                    url,
                    headers={"Authorization": "Bearer expected"},
                )
                with urllib.request.urlopen(request, timeout=2) as response:
                    self.assertEqual(response.status, 200)
            finally:
                server.shutdown()
                server.server_close()
                thread.join(timeout=2)
                setattr(module, token_name, previous)


if __name__ == "__main__":
    unittest.main()
