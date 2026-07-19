"""US-H8-002 AC11：入站/出站交换生命周期审计（Worker 真实路径调用）。

阶段与 domain H8_EXCHANGE_AUDIT_STAGES 对齐：
  receive → convert → business_api → send → receipt → final_failure

通过 WMS `POST /api/v1/integration/erp-messages/lifecycle` 写入 H2 脱敏审计；
无 message_id 时由服务端按幂等键 upsert 消息主记录。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable

# 与 backend/crates/domain/src/h8_erp_exchange.rs 保持一致
H8_EXCHANGE_AUDIT_STAGES = (
    "receive",
    "convert",
    "business_api",
    "send",
    "receipt",
    "final_failure",
)


def is_exchange_audit_stage(stage: str) -> bool:
    return stage in H8_EXCHANGE_AUDIT_STAGES


HttpJsonFn = Callable[
    [Any, str, str, dict[str, Any] | None, str],
    tuple[int, dict[str, Any] | None, str],
]


@dataclass
class ExchangeLifecycle:
    """一次消息处理的生命周期审计发射器（绑定单行业务键）。"""

    settings: Any
    message_type: str
    external_ref: str
    idempotency_key: str
    direction: str = "inbound"
    channel: str = "interface_table"
    correlation_id: str = ""
    message_id: str | None = None
    stages_emitted: list[tuple[str, str]] = field(default_factory=list)
    http_json: HttpJsonFn | None = None

    def __post_init__(self) -> None:
        if not self.correlation_id:
            self.correlation_id = f"h8-{self.message_type}-{self.idempotency_key}"

    def stage(self, stage: str, result: str) -> None:
        if not is_exchange_audit_stage(stage):
            raise ValueError(f"invalid exchange stage: {stage}")
        self.stages_emitted.append((stage, result))
        body: dict[str, Any] = {
            "stage": stage,
            "result": result,
            "direction": self.direction,
            "message_type": self.message_type,
            "external_ref": self.external_ref,
            "idempotency_key": self.idempotency_key,
            "correlation_id": self.correlation_id,
            "channel": self.channel,
        }
        if self.message_id:
            body["message_id"] = self.message_id
        http = self.http_json
        if http is None:
            # 延迟导入避免循环；与 sync_worker.http_json 签名一致
            from sync_worker import http_json as default_http

            http = default_http
        status, parsed, raw = http(
            self.settings,
            "POST",
            "/api/v1/integration/erp-messages/lifecycle",
            body,
            f"h8-life-{self.idempotency_key}-{stage}-{len(self.stages_emitted)}",
        )
        if status not in (200, 201):
            # 审计失败不阻断业务，但记录；单测可断言 stages_emitted
            print(
                f"[h8] lifecycle audit {stage} HTTP {status}: {raw[:200]}",
                flush=True,
            )
            return
        if isinstance(parsed, dict) and parsed.get("id"):
            self.message_id = str(parsed["id"])


def run_inbound_pipeline(
    settings: Any,
    message_type: str,
    row: dict[str, str],
    handler: Callable[[Any, dict[str, str]], str],
    *,
    http_json: HttpJsonFn | None = None,
    dry_run: bool = False,
) -> tuple[str | None, ExchangeLifecycle]:
    """真实入站路径：receive→convert→business_api→receipt|final_failure。

    返回 (wms_resource_id|None, lifecycle)。
    """
    external = row.get("external_ref") or row.get("external_doc_no") or row.get("id") or ""
    idem = row.get("idempotency_key") or f"{message_type}-{row.get('id', 'x')}"
    life = ExchangeLifecycle(
        settings=settings,
        message_type=message_type,
        external_ref=str(external),
        idempotency_key=str(idem),
        direction="inbound",
        channel="interface_table",
        http_json=http_json,
    )
    life.stage("receive", "ok")
    life.stage("convert", "ok")
    if dry_run:
        return None, life
    try:
        life.stage("business_api", "started")
        wms_id = handler(settings, row)
        life.stage("business_api", "ok")
        # 入站成功：WMS 已接受即视为回执阶段完成（ERP 业务回执另走 acked）
        life.stage("receipt", "ok")
        return wms_id, life
    except Exception as exc:  # noqa: BLE001
        life.stage("final_failure", "error")
        raise exc


def run_outbound_pipeline(
    settings: Any,
    message_type: str,
    external_ref: str,
    idempotency_key: str,
    send_fn: Callable[[], None],
    *,
    http_json: HttpJsonFn | None = None,
    dry_run: bool = False,
) -> ExchangeLifecycle:
    """真实出站路径：receive→convert→send→receipt|final_failure。"""
    life = ExchangeLifecycle(
        settings=settings,
        message_type=message_type,
        external_ref=external_ref,
        idempotency_key=idempotency_key,
        direction="outbound",
        channel="rest",
        http_json=http_json,
    )
    life.stage("receive", "ok")
    life.stage("convert", "ok")
    if dry_run:
        return life
    try:
        life.stage("send", "started")
        send_fn()
        life.stage("send", "ok")
        life.stage("receipt", "ok")
        return life
    except Exception as exc:  # noqa: BLE001
        life.stage("final_failure", "error")
        raise exc
