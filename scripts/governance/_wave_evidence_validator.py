#!/usr/bin/env python3
"""Shared helpers for Wave runtime evidence validator scripts."""
from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Callable


PayloadValidator = Callable[..., tuple[bool, str]]


def read_json(path: Path) -> tuple[object | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as error:
        return None, f"invalid JSON: {path}: {error}"


def bad_ref(
    value: str,
    *,
    allow_example_refs: bool,
    blocked_ref_tokens: tuple[str, ...],
) -> bool:
    lowered = value.lower()
    blocked = blocked_ref_tokens if not allow_example_refs else blocked_ref_tokens[:-1]
    return any(token in lowered for token in blocked)


def has_environment_token(value: str, environment: str) -> bool:
    return re.search(
        rf"(^|[^0-9a-z]){re.escape(environment)}([^0-9a-z]|$)",
        value.lower(),
    ) is not None


def positive_int(payload: dict[str, object], key: str) -> bool:
    value = payload.get(key)
    return isinstance(value, int) and value >= 1


def placeholder_fields(
    payload: dict[str, object],
    keys: tuple[str, ...],
    *,
    placeholder_tokens: tuple[str, ...],
) -> list[str]:
    fields = []
    for key in keys:
        value = payload.get(key)
        if not isinstance(value, str):
            continue
        lowered = value.lower()
        if any(token in lowered for token in placeholder_tokens):
            fields.append(key)
    return fields


def validate_one(
    path: Path,
    *,
    allow_example_refs: bool,
    validate: PayloadValidator,
) -> tuple[bool, str]:
    payload, error = read_json(path)
    if error:
        return False, error
    ok, message = validate(payload, allow_example_refs=allow_example_refs)
    return ok, f"{path}: {message}"


def blocked_ref_fields(
    payload: dict[str, object],
    keys: tuple[str, ...],
    *,
    is_bad_ref,
    allow_example_refs: bool,
) -> list[str]:
    return [
        key
        for key in keys
        if is_bad_ref(
            str(payload.get(key, "")),
            allow_example_refs=allow_example_refs,
        )
    ]


def blocked_ref_message(boundary: str, fields: list[str]) -> str:
    return f"证据引用不能指向 {boundary} 边界: {', '.join(fields)}"
