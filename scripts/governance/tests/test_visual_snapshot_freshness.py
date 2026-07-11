"""视觉回归 freshness 使用内容摘要，不依赖文件时间。"""
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from check_visual_regression import _source_digest, visual_source_files
from check_baseline_completeness import (
    _duplicate_baseline_errors,
    _pending_baseline_review_errors,
    _review_metadata_errors,
)
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


def test_changed_baseline_requires_review_metadata_in_same_manifest_hunk():
    stale = '''@@ -1,3 +1,3 @@
 file = "m2.png"
 reviewed_by = "项目主人"
 reviewed_at = "2026-06-27"
'''
    reviewed = '''@@ -1,3 +1,3 @@
 file = "m2.png"
-reviewed_by = "项目主人"
-reviewed_at = "2026-06-27"
+reviewed_by = "zhouliang"
+reviewed_at = "2026-07-11"
'''

    assert _review_metadata_errors(["m2.png"], stale)
    assert _review_metadata_errors(["m2.png"], reviewed) == []


def test_committed_baseline_changes_are_checked(monkeypatch):
    calls = []

    class Result:
        def __init__(self, returncode=0, stdout=""):
            self.returncode = returncode
            self.stdout = stdout

    def run(command, **_kwargs):
        calls.append(command)
        if "rev-parse" in command:
            return Result()
        if "--name-only" in command and "main...HEAD" in command:
            return Result(stdout="governance/visual-baselines/m2.png\n")
        if "--unified=5" in command and "main...HEAD" in command:
            return Result(stdout='@@ -1,3 +1,3 @@\n file = "m2.png"\n reviewed_by = "项目主人"\n reviewed_at = "2026-06-27"\n')
        return Result()

    monkeypatch.setattr("check_baseline_completeness.subprocess.run", run)

    assert _pending_baseline_review_errors("main")
    assert any("main...HEAD" in command for command in calls)


def test_reviewed_staged_baseline_does_not_hide_unreviewed_committed_change(monkeypatch):
    class Result:
        def __init__(self, returncode=0, stdout=""):
            self.returncode = returncode
            self.stdout = stdout

    def run(command, **_kwargs):
        if "rev-parse" in command:
            return Result()
        if "--name-only" in command and "--cached" in command:
            return Result(stdout="governance/visual-baselines/staged.png\n")
        if "--unified=5" in command and "--cached" in command:
            return Result(stdout='@@\n file = "staged.png"\n+reviewed_by = "项目主人"\n+reviewed_at = "2026-07-11"\n')
        if "--name-only" in command and "main...HEAD" in command:
            return Result(stdout="governance/visual-baselines/committed.png\n")
        if "--unified=5" in command and "main...HEAD" in command:
            return Result(stdout='@@\n file = "committed.png"\n reviewed_by = "项目主人"\n reviewed_at = "2026-06-27"\n')
        return Result()

    monkeypatch.setattr("check_baseline_completeness.subprocess.run", run)

    errors = _pending_baseline_review_errors("main")

    assert errors == ["baseline PNG 'committed.png' 已修改，但对应 manifest 区块未同步更新审核人和审核日期"]


def test_baseline_file_must_not_be_shared_between_tabs():
    assert _duplicate_baseline_errors({"m2-pda": "m2.png", "m2-pc": "m2.png"})
    assert _duplicate_baseline_errors({"m2-pda": "pda.png", "m2-pc": "pc.png"}) == []


def test_interaction_capture_closes_each_scene_context():
    source = (
        Path(__file__).resolve().parent.parent / "capture_web_admin_interaction_screenshots.mjs"
    ).read_text(encoding="utf-8")
    scene_loop = source[source.index("for (const scene of scenes)") : source.index("await browser.close()")]

    assert "const context = await browser.newContext" in scene_loop
    assert "finally" in scene_loop
    assert "await context.close()" in scene_loop
