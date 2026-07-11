"""视觉回归 freshness 使用内容摘要，不依赖文件时间。"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from check_visual_regression import _source_digest, visual_source_files
from capture_visual_snapshots import _capture


def test_source_digest_changes_with_content(tmp_path):
    source = tmp_path / "Page.tsx"
    source.write_text("export {};", encoding="utf-8")
    before = _source_digest([source], root=tmp_path)

    source.write_text("export const changed = true;", encoding="utf-8")

    assert _source_digest([source], root=tmp_path) != before


def test_source_digest_ignores_mtime(tmp_path):
    source = tmp_path / "Page.tsx"
    source.write_text("export {};", encoding="utf-8")
    before = _source_digest([source], root=tmp_path)

    os.utime(source, (20, 20))

    assert _source_digest([source], root=tmp_path) == before


def test_visual_source_files_include_build_configuration():
    paths = {path.relative_to(Path.cwd()).as_posix() for path in visual_source_files()}

    assert {
        "prototypes/index.html",
        "prototypes/postcss.config.js",
        "prototypes/tailwind.config.js",
        "packages/ui/tailwind-preset.cjs",
    } <= paths


def test_failed_capture_does_not_reuse_old_png(tmp_path):
    output = tmp_path / "old.png"
    output.write_bytes(b"old")

    ok, _ = _capture("/bin/false", "http://127.0.0.1:1", 1, 1, output)

    assert ok is False
    assert not output.exists()
