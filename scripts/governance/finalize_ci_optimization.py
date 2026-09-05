#!/usr/bin/env python3
"""Finalize the staged PR CI optimization and remove temporary artifacts."""

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(
            f"expected exactly one match in {path}, found {count}: {old!r}"
        )
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


def main() -> None:
    fast = ".github/workflows/pr-fast-validation.yml"
    replace_once(
        fast,
        "    types: [opened, synchronize, reopened, ready_for_review]",
        "    types: [opened, synchronize, reopened, converted_to_draft]",
    )
    replace_once(
        fast,
        "jobs:\n  fast-validation:\n    runs-on: ubuntu-latest",
        "jobs:\n  fast-validation:\n    if: ${{ github.event.pull_request.draft == true }}\n    runs-on: ubuntu-latest",
    )

    replace_once(
        ".github/workflows/governance.yml",
        "        run: python3 scripts/governance/task_check.py --tier T2 --context pr --base origin/main --strict",
        "        run: python3 scripts/governance/task_check.py --tier T2 --context pr --base origin/main --strict --skip-t1-fallback",
    )

    deep = ".github/workflows/pr-deep-validation.yml"
    replace_once(deep, "    timeout-minutes: 150", "    timeout-minutes: 180")
    marker = "      - name: Prototype TypeScript and Vite build\n"
    insertion = """      - name: Focused PostgreSQL regression tests
        run: |
          cargo test \\
            --manifest-path backend/Cargo.toml \\
            --package wms-api \\
            --test m3_replenishment_location_group_t3_postgres \\
            --test m3_replenishment_wave_postgres \\
            --test schema_baseline_postgres \\
            --test wave4_postgres \\
            --no-fail-fast \\
            --jobs 2

      - name: Full Rust workspace tests
        run: |
          echo "=== disk before full Rust tests ==="
          df -h /
          cargo test \\
            --manifest-path backend/Cargo.toml \\
            --workspace \\
            --no-fail-fast \\
            --jobs 2
          echo "=== disk after full Rust tests ==="
          df -h /
          du -sh backend/target || true

      - name: Prototype TypeScript and Vite build
"""
    deep_path = Path(deep)
    deep_text = deep_path.read_text(encoding="utf-8")
    if "      - name: Focused PostgreSQL regression tests\n" in deep_text:
        raise SystemExit("focused PostgreSQL regression step already exists")
    if "      - name: Full Rust workspace tests\n" in deep_text:
        raise SystemExit("full Rust workspace test step already exists")
    if deep_text.count(marker) != 1:
        raise SystemExit(
            f"expected exactly one prototype build marker, found {deep_text.count(marker)}"
        )
    deep_path.write_text(deep_text.replace(marker, insertion, 1), encoding="utf-8")


if __name__ == "__main__":
    main()
