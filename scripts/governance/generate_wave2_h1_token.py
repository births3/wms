#!/usr/bin/env python3
"""Generate a short-lived H1 token for Wave 2 staging runtime evidence."""
from __future__ import annotations

import argparse
import base64
import hashlib
import hmac
import json
import os
import sys
from datetime import datetime, timezone
from uuid import UUID, uuid4

ACCESS_TOKEN_TTL_SECONDS = 60 * 60
DEFAULT_PERMISSIONS = ("m1.config.write", "m3.read")
JWT_SECRET_ENV = "WMS_JWT_SECRET"


class TokenError(Exception):
    pass


def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).decode("ascii").rstrip("=")


def b64url_decode(data: str) -> bytes:
    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode((data + padding).encode("ascii"))


def parse_uuid(label: str, value: str) -> str:
    try:
        return str(UUID(value))
    except ValueError as error:
        raise TokenError(f"{label} must be a UUID") from error


def parse_issued_at(value: str | None) -> int:
    if not value:
        return int(datetime.now(timezone.utc).timestamp())
    normalized = value.replace("Z", "+00:00")
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError as error:
        raise TokenError("--issued-at must be ISO-8601, for example 2026-06-07T00:00:00Z") from error
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return int(parsed.timestamp())


def jwt_secret(args: argparse.Namespace) -> str:
    secret = (args.jwt_secret or os.environ.get(JWT_SECRET_ENV, "")).strip()
    if len(secret) < 32:
        raise TokenError("jwt secret must be at least 32 characters and must come from staging secrets")
    return secret


def encode_claims(claims: dict[str, object], secret: str) -> str:
    header = {"alg": "HS256", "typ": "JWT"}
    header_b64 = b64url(json.dumps(header, separators=(",", ":")).encode("utf-8"))
    payload_b64 = b64url(json.dumps(claims, separators=(",", ":")).encode("utf-8"))
    signing_input = f"{header_b64}.{payload_b64}".encode("ascii")
    signature = hmac.new(secret.encode("utf-8"), signing_input, hashlib.sha256).digest()
    return f"{header_b64}.{payload_b64}.{b64url(signature)}"


def decode_unverified_claims(token: str) -> dict[str, object]:
    parts = token.split(".")
    if len(parts) != 3:
        raise TokenError("token must have three JWT segments")
    payload = json.loads(b64url_decode(parts[1]).decode("utf-8"))
    if not isinstance(payload, dict):
        raise TokenError("token payload must be object")
    return payload


def build_claims(args: argparse.Namespace) -> dict[str, object]:
    issued_at = parse_issued_at(args.issued_at)
    return {
        "sub": parse_uuid("--user-id", args.user_id),
        "owner_id": parse_uuid("--owner-id", args.owner_id),
        "user_name": args.user_name,
        "permissions": list(DEFAULT_PERMISSIONS),
        "jti": args.jti,
        "iat": issued_at,
        "exp": issued_at + ACCESS_TOKEN_TTL_SECONDS,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="Emit smoke metadata only; does not generate a token")
    parser.add_argument("--jwt-secret", help=f"JWT signing secret; defaults to ${JWT_SECRET_ENV}")
    parser.add_argument("--user-id", default=str(uuid4()))
    parser.add_argument("--owner-id", default=str(uuid4()))
    parser.add_argument("--user-name", default="wave2-staging-operator")
    parser.add_argument("--jti")
    parser.add_argument("--issued-at")
    args = parser.parse_args(argv)
    if args.json:
        print(json.dumps({
            "script": "generate_wave2_h1_token",
            "category": "流程治理",
            "tier": "manual",
            "writes_runtime_evidence": False,
            "emits_token": False,
            "required_permissions": list(DEFAULT_PERMISSIONS),
            "ok": True,
        }, ensure_ascii=False))
        return 0
    if not args.jti:
        args.jti = f"wave2-staging-{uuid4()}"

    try:
        token = encode_claims(build_claims(args), jwt_secret(args))
    except TokenError as error:
        print(f"wave2 h1 token rejected: {error}", file=sys.stderr)
        return 2
    print(f"export WAVE_2_H1_TOKEN={token}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
