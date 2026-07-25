"""M-RC 自研 ERP 库存快照主动拉取。"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request
import uuid
from typing import Any, Callable

from worker_route import WorkerHttpError, resolve_bearer_token, resolve_outbound_route

CLAIM_LEASE_SECONDS = 120


def pull_due_reconciliation_snapshots(
    settings: Any,
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]],
    erp_get_fn: Callable[..., dict[str, Any]] | None = None,
) -> list[str]:
    """由 H-SCH 单次触发，先认领当前令牌货主的到期窗口再主动拉取。"""
    status, parsed, raw = http_json_fn(
        settings,
        "POST",
        "/api/v1/reconciliation/claims",
        {
            "worker_id": settings.worker_id,
            "lease_seconds": CLAIM_LEASE_SECONDS,
        },
        f"rc:claim:{settings.worker_id}:{uuid.uuid4()}",
    )
    if status != 200 or not isinstance(parsed, dict):
        raise WorkerHttpError(status, "WMS reconciliation claim", raw)
    claim = parsed.get("claim")
    if claim is None:
        return []
    if not isinstance(claim, dict):
        raise WorkerHttpError(502, "WMS reconciliation claim", "claim invalid")
    return [
        pull_reconciliation_snapshot(
            settings,
            _required_text(claim, "owner_id"),
            _required_text(claim, "window_key"),
            claim=claim,
            http_json_fn=http_json_fn,
            erp_get_fn=erp_get_fn,
        )
    ]


def pull_reconciliation_snapshot(
    settings: Any,
    owner_id: str,
    window_key: str,
    *,
    claim: dict[str, Any] | None = None,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]],
    erp_get_fn: Callable[..., dict[str, Any]] | None = None,
) -> str:
    """持有有限租约分页拉取快照，再原子提交 WMS 对账事务与 claim。"""
    if claim is None:
        raise WorkerHttpError(422, "WMS reconciliation claim", "claim required")
    claim_id = _required_text(claim, "id")
    claim_token = _required_text(claim, "claim_token")
    if _required_text(claim, "owner_id") != owner_id:
        raise WorkerHttpError(422, "WMS reconciliation claim", "owner mismatch")
    if _required_text(claim, "window_key") != window_key:
        raise WorkerHttpError(422, "WMS reconciliation claim", "window mismatch")
    idempotency_key = f"rc:{owner_id}:{window_key}"
    try:
        binding = resolve_outbound_route(
            settings,
            "inventory_snapshot",
            owner_id,
            None,
            idempotency_key,
            http_json_fn=http_json_fn,
            require_owner_wide=True,
        )
        if not binding.api_base_url:
            raise WorkerHttpError(422, "ERP inventory query", "api_base_url missing")
        erp_get_fn = erp_get_fn or erp_get_json
        bearer_token = resolve_bearer_token(binding.bearer_secret_alias)
        cursor: str | None = None
        snapshot_at: str | None = None
        items: list[dict[str, Any]] = []
        for page_no in range(1000):
            _renew_claim(
                settings,
                claim_id,
                claim_token,
                page_no,
                http_json_fn=http_json_fn,
            )
            page = erp_get_fn(binding.api_base_url, owner_id, cursor, bearer_token)
            page_snapshot_at = _required_text(page, "snapshot_at")
            if snapshot_at is not None and snapshot_at != page_snapshot_at:
                raise WorkerHttpError(422, "ERP inventory query", "snapshot_at changed")
            snapshot_at = page_snapshot_at
            page_items = page.get("items")
            if not isinstance(page_items, list):
                raise WorkerHttpError(422, "ERP inventory query", "items missing")
            items.extend(_canonical_item(item) for item in page_items)
            next_cursor = page.get("next_cursor")
            if next_cursor in (None, ""):
                break
            cursor = str(next_cursor)
        else:
            raise WorkerHttpError(
                422, "ERP inventory query", "pagination exceeds 1000 pages"
            )
        _renew_claim(
            settings,
            claim_id,
            claim_token,
            page_no + 1,
            http_json_fn=http_json_fn,
        )
    except Exception as exc:
        _try_report_claim_failure(
            settings,
            claim_id,
            claim_token,
            "pull",
            "erp_pull_failed",
            http_json_fn=http_json_fn,
        )
        raise
    body = {
        "claim_id": claim_id,
        "claim_token": claim_token,
        "window_key": window_key,
        "snapshot_at": snapshot_at,
        "items": items,
    }
    try:
        status, parsed, raw = http_json_fn(
            settings,
            "POST",
            "/api/v1/reconciliation/runs",
            body,
            idempotency_key,
        )
        if status != 200 or not isinstance(parsed, dict) or not parsed.get("id"):
            raise WorkerHttpError(status, "WMS reconciliation", raw)
    except Exception as exc:
        _try_report_claim_failure(
            settings,
            claim_id,
            claim_token,
            "submit",
            "snapshot_submit_failed",
            http_json_fn=http_json_fn,
        )
        raise
    return str(parsed["id"])


def _renew_claim(
    settings: Any,
    claim_id: str,
    claim_token: str,
    page_no: int,
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]],
) -> None:
    status, parsed, raw = http_json_fn(
        settings,
        "POST",
        f"/api/v1/reconciliation/claims/{claim_id}/renew",
        {
            "claim_token": claim_token,
            "worker_id": settings.worker_id,
            "lease_seconds": CLAIM_LEASE_SECONDS,
        },
        f"rc:renew:{claim_id}:{page_no}",
    )
    if (
        status != 200
        or not isinstance(parsed, dict)
        or parsed.get("status") != "active"
    ):
        raise WorkerHttpError(status, "WMS reconciliation claim renewal", raw)


def _report_claim_failure(
    settings: Any,
    claim_id: str,
    claim_token: str,
    stage: str,
    error_code: str,
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]],
) -> None:
    status, parsed, raw = http_json_fn(
        settings,
        "POST",
        f"/api/v1/reconciliation/claims/{claim_id}/failed",
        {
            "claim_token": claim_token,
            "stage": stage,
            "error_code": error_code,
        },
        f"rc:failed:{claim_id}:{stage}:{error_code}",
    )
    if (
        status != 200
        or not isinstance(parsed, dict)
        or parsed.get("status") != "failed"
    ):
        raise WorkerHttpError(status, "WMS reconciliation failure report", raw)


def _try_report_claim_failure(
    settings: Any,
    claim_id: str,
    claim_token: str,
    stage: str,
    error_code: str,
    *,
    http_json_fn: Callable[..., tuple[int, dict[str, Any] | None, str]],
) -> None:
    try:
        _report_claim_failure(
            settings,
            claim_id,
            claim_token,
            stage,
            error_code,
            http_json_fn=http_json_fn,
        )
    except WorkerHttpError:
        # 原始拉取/提交异常是调度器需要重试的根因；失败上报不可掩盖根因。
        return


def erp_get_json(
    base_url: str,
    owner_id: str,
    cursor: str | None,
    bearer_token: str | None = None,
) -> dict[str, Any]:
    query = {"owner_id": owner_id}
    if cursor:
        query["cursor"] = cursor
    url = (
        base_url.rstrip("/")
        + "/inventory-snapshots?"
        + urllib.parse.urlencode(query)
    )
    headers = {"Accept": "application/json"}
    if bearer_token:
        headers["Authorization"] = f"Bearer {bearer_token}"
    try:
        with urllib.request.urlopen(
            urllib.request.Request(url, headers=headers), timeout=60
        ) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise WorkerHttpError(
            exc.code, "ERP inventory query", exc.read().decode("utf-8", "replace")
        ) from exc
    except urllib.error.URLError as exc:
        raise WorkerHttpError(0, "ERP inventory query", str(exc.reason)) from exc
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise WorkerHttpError(502, "ERP inventory query", "invalid JSON") from exc
    if not isinstance(value, dict):
        raise WorkerHttpError(502, "ERP inventory query", "object required")
    return value


def _canonical_item(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise WorkerHttpError(422, "ERP inventory query", "invalid item")
    product_code = _required_text(value, "product_code")
    batch_no = _required_text(value, "batch_no")
    qty = value.get("qty_on_hand")
    if not isinstance(qty, int) or isinstance(qty, bool) or qty < 0:
        raise WorkerHttpError(422, "ERP inventory query", "invalid qty_on_hand")
    return {
        "product_code": product_code,
        "batch_no": batch_no,
        "qty_on_hand": qty,
    }


def _required_text(value: dict[str, Any], field: str) -> str:
    text = value.get(field)
    if not isinstance(text, str) or not text.strip():
        raise WorkerHttpError(422, "ERP inventory query", f"{field} required")
    return text.strip()
