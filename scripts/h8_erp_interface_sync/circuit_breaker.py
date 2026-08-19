"""US-H8-001 AC10：REST 通道简易熔断 / 半开恢复（Worker 侧）。

状态：
  closed   — 正常走 REST
  open     — 连续失败达阈值，直接降级接口表
  half_open — 冷却后允许一次 REST 探测；成功回 closed，失败回 open
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

CircuitState = Literal["closed", "open", "half_open"]


@dataclass
class CircuitBreaker:
    failure_threshold: int = 3
    half_open_after_failures: int = 3  # open 后累计调用次数达到后进入 half_open
    state: CircuitState = "closed"
    consecutive_failures: int = 0
    calls_while_open: int = 0

    def allow_http(self) -> bool:
        if self.state == "closed":
            return True
        if self.state == "half_open":
            return True
        # open：累计探测窗口
        self.calls_while_open += 1
        if self.calls_while_open >= self.half_open_after_failures:
            self.state = "half_open"
            self.calls_while_open = 0
            return True
        return False

    def on_http_success(self) -> None:
        self.state = "closed"
        self.consecutive_failures = 0
        self.calls_while_open = 0

    def on_http_failure(self) -> None:
        self.consecutive_failures += 1
        if self.state == "half_open":
            self.state = "open"
            self.calls_while_open = 0
            return
        if self.consecutive_failures >= self.failure_threshold:
            self.state = "open"
            self.calls_while_open = 0
