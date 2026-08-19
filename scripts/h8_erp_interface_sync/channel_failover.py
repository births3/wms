"""US-H8-001 主备降级：REST 主用 → 接口表备用（非双写）。

channel_mode:
  rest                         → 仅 HTTP
  interface_table              → 仅接口表
  rest_primary_table_fallback  → 先 HTTP，失败后同一 Idempotency-Key 转接口表
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Literal

from circuit_breaker import CircuitBreaker

ChannelMode = Literal["rest", "interface_table", "rest_primary_table_fallback"]
Transport = Literal["http", "table", "both", "failover"]
PublishChannel = Literal["http", "table", "table_fallback"]


def map_channel_mode_to_transport(channel_mode: str) -> Transport:
    mode = (channel_mode or "").strip().lower()
    if mode == "rest":
        return "http"
    if mode == "interface_table":
        return "table"
    if mode == "rest_primary_table_fallback":
        return "failover"
    return "table"


def production_allows_simultaneous_dual_write(channel_mode: str) -> bool:
    """生产 channel_mode 永不同时双写；both 仅本地联调。"""
    _ = channel_mode
    return False


@dataclass
class PublishResult:
    channel: PublishChannel
    attempts_http: int
    fallback_used: bool
    error: str | None = None


def publish_with_failover(
    *,
    transport: str,
    publish_http: Callable[[], None],
    publish_table: Callable[[], None],
    http_max_attempts: int = 2,
    circuit: CircuitBreaker | None = None,
) -> PublishResult:
    """
    按 transport 投递；failover 时先 REST 再 table，禁止双成功双写。
    AC10：可选 circuit 实现熔断 open / 半开探测后回主。
    """
    t = (transport or "table").strip().lower()
    if t == "table":
        publish_table()
        return PublishResult(channel="table", attempts_http=0, fallback_used=False)
    if t == "http":
        publish_http()
        return PublishResult(channel="http", attempts_http=1, fallback_used=False)
    if t == "both":
        # 本地双写联调：顺序执行，任一失败抛错；成功表示 table+http 均已写（error 保持 None）
        publish_table()
        publish_http()
        return PublishResult(channel="http", attempts_http=1, fallback_used=False)
    if t == "failover":
        errors: list[str] = []
        attempts = 0
        allow_http = True if circuit is None else circuit.allow_http()
        if allow_http:
            for _ in range(max(1, http_max_attempts)):
                attempts += 1
                try:
                    publish_http()
                    if circuit is not None:
                        circuit.on_http_success()
                    return PublishResult(
                        channel="http",
                        attempts_http=attempts,
                        fallback_used=False,
                    )
                except Exception as exc:  # noqa: BLE001
                    errors.append(str(exc))
                    if circuit is not None:
                        circuit.on_http_failure()
        else:
            errors.append("circuit_open:skip_http")
        # REST 失败或熔断 open → 接口表备用，保持业务幂等键由调用方携带
        try:
            publish_table()
        except Exception as exc:  # noqa: BLE001
            raise RuntimeError(
                f"failover: http failed ({'; '.join(errors)}); "
                f"table also failed: {exc}"
            ) from exc
        return PublishResult(
            channel="table_fallback",
            attempts_http=attempts,
            fallback_used=True,
            error="; ".join(errors),
        )
    raise ValueError(f"unknown transport: {transport}")
