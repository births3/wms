#!/usr/bin/env python3
"""Shared helpers for Wave runtime evidence recorder scripts."""
from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence


Validator = Callable[[dict[str, Any]], tuple[bool, str]]


def check_only_result(ok: bool, message: str, evidence_file: Path) -> dict[str, object]:
    return {
        "ok": ok,
        "check_only": True,
        "writes_runtime_evidence": False,
        "closes_gate": False,
        "evidence_file": str(evidence_file),
        "message": message,
    }


def write_payload(
    path: Path,
    payload: dict[str, Any],
    *,
    force: bool,
    validate: Validator,
) -> tuple[bool, str]:
    ok, message = validate(payload)
    if not ok:
        return False, message
    if path.exists() and not force:
        return False, f"{path} already exists; pass --force to overwrite"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return True, f"wrote {path}"


def check_payload(payload: dict[str, Any], *, validate: Validator) -> tuple[bool, str]:
    ok, message = validate(payload)
    if ok:
        return True, f"check-only passed: {message}"
    return False, message


def missing_required_args(
    args: object,
    *,
    string_args: Sequence[str],
    count_args: Sequence[str],
) -> list[str]:
    missing: list[str] = []
    for field in string_args:
        if not str(getattr(args, field, "") or "").strip():
            missing.append(f"--{field.replace('_', '-')}")
    for field in count_args:
        if getattr(args, field, None) is None:
            missing.append(f"--{field.replace('_', '-')}")
    return missing


def missing_env_var_owners(
    missing_env_vars: Sequence[str],
    env_var_owners: Mapping[str, tuple[str, str]],
) -> list[dict[str, str]]:
    return [
        {
            "env_var": env_var,
            "source_owner": env_var_owners[env_var][0],
            "evidence_requirement": env_var_owners[env_var][1],
        }
        for env_var in missing_env_vars
    ]


def display_evidence_file(path: Path, *, repo_root: Path) -> Path:
    try:
        return path.resolve().relative_to(repo_root)
    except ValueError:
        return path


def bool_from_env(value: str) -> bool:
    return value.strip().lower() in {"1", "true", "yes", "y", "on"}


def apply_from_env(
    args: object,
    *,
    env_vars: Mapping[str, str],
    count_args: Sequence[str],
    bool_args: Sequence[str],
) -> list[str]:
    missing: list[str] = []
    for field, env_var in env_vars.items():
        raw_value = os.environ.get(env_var)
        if raw_value is None or raw_value.strip() == "":
            missing.append(env_var)
            continue
        if field in count_args:
            try:
                setattr(args, field, int(raw_value))
            except ValueError:
                setattr(args, field, None)
            continue
        if field in bool_args:
            setattr(args, field, bool_from_env(raw_value))
            continue
        setattr(args, field, raw_value)
    return missing


def missing_from_env_result(
    *,
    args: object,
    missing_env_vars: list[str],
    message: str,
    repo_root: Path,
    owner_map: Mapping[str, tuple[str, str]] | None = None,
) -> dict[str, object]:
    result = {
        **check_only_result(
            False,
            message,
            display_evidence_file(getattr(args, "output"), repo_root=repo_root),
        ),
        "missing_env_vars": missing_env_vars,
    }
    if owner_map is not None:
        result["missing_env_var_owners"] = missing_env_var_owners(missing_env_vars, owner_map)
    return result
