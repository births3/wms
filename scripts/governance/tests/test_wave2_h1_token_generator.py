"""Wave 2 staging H1 token generator tests."""
import sys
from pathlib import Path
import json

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_wave2_h1_token_generator_outputs_export_without_leaking_to_stderr(
    capsys,
):
    """Token helper should emit a shell export for W6.C smoke and keep stderr clean."""
    import generate_wave2_h1_token as generator

    assert generator.main([
        "--jwt-secret",
        "staging-secret-at-least-32-characters",
        "--user-id",
        "11111111-1111-4111-8111-111111111111",
        "--owner-id",
        "22222222-2222-4222-8222-222222222222",
        "--user-name",
        "wave2-staging-operator",
        "--jti",
        "wave2-staging-jti",
        "--issued-at",
        "2026-06-07T00:00:00Z",
    ]) == 0

    out = capsys.readouterr()
    assert out.err == ""
    assert out.out.startswith("export WAVE_2_H1_TOKEN=")
    assert "staging-secret" not in out.out

    token = out.out.strip().split("=", maxsplit=1)[1]
    claims = generator.decode_unverified_claims(token)
    assert claims["sub"] == "11111111-1111-4111-8111-111111111111"
    assert claims["owner_id"] == "22222222-2222-4222-8222-222222222222"
    assert claims["user_name"] == "wave2-staging-operator"
    assert claims["jti"] == "wave2-staging-jti"
    assert claims["permissions"] == ["m1.config.write", "m3.read"]
    assert claims["exp"] - claims["iat"] == 3600


def test_wave2_h1_token_generator_rejects_short_secret(capsys):
    """Staging token helper must not normalize weak JWT secrets."""
    import generate_wave2_h1_token as generator

    assert generator.main([
        "--jwt-secret",
        "short",
        "--user-id",
        "11111111-1111-4111-8111-111111111111",
        "--owner-id",
        "22222222-2222-4222-8222-222222222222",
    ]) == 2

    out = capsys.readouterr()
    assert "jwt secret" in out.err.lower()
    assert "short" not in out.err


def test_wave2_h1_token_generator_can_read_secret_from_env(monkeypatch, capsys):
    """Runbook can use WMS_JWT_SECRET without putting the secret on the command line."""
    import generate_wave2_h1_token as generator

    monkeypatch.setenv("WMS_JWT_SECRET", "staging-secret-from-env-at-least-32")

    assert generator.main([
        "--user-id",
        "11111111-1111-4111-8111-111111111111",
        "--owner-id",
        "22222222-2222-4222-8222-222222222222",
    ]) == 0

    out = capsys.readouterr()
    assert out.out.startswith("export WAVE_2_H1_TOKEN=")
    assert "staging-secret-from-env" not in out.out


def test_wave2_h1_token_just_entry_calls_generator():
    """The runbook just entry must stay bound to the token helper."""
    justfile = Path("justfile").read_text(encoding="utf-8")

    assert "wave-2-h1-token *args:" in justfile
    assert "scripts/governance/generate_wave2_h1_token.py {{args}}" in justfile


def test_wave2_h1_token_json_mode_does_not_emit_token(capsys):
    """Smoke JSON mode reports metadata only and must not emit WAVE_2_H1_TOKEN."""
    import generate_wave2_h1_token as generator

    assert generator.main(["--json"]) == 0

    payload = json.loads(capsys.readouterr().out)
    assert payload["ok"] is True
    assert payload["emits_token"] is False
    assert payload["required_permissions"] == ["m1.config.write", "m3.read"]
    assert "WAVE_2_H1_TOKEN" not in json.dumps(payload)
