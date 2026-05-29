#!/usr/bin/env python3
"""generate_prototype_proof_report.py — 生成原型验收证据报告

用途：
  生成 docs/prototypes/prototype-proof-report.md。

报告逐行列出每个原型 tab 的：
  - 矩阵来源
  - Tab
  - 模型关键词命中
  - 截图 OCR
  - review 签字

依赖：tesseract（用于 OCR）+ tomli/tomllib。
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from check_prototype_fidelity import (  # type: ignore
    SEMANTIC_TERMS,
    TABS_FILE,
    _blueprints,
    _keyword_profiles,
    _model_text_for_spec,
    _specs_from_specs,
    _tabs_legacy_mappings,
)

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
MATRIX_MD = REPO_ROOT / "docs" / "prototypes" / "prototype-matrix-r3.md"
MANIFEST_TOML = REPO_ROOT / "governance" / "visual-baselines" / "manifest.toml"
BASELINE_DIR = REPO_ROOT / "governance" / "visual-baselines"
REPORT_MD = REPO_ROOT / "docs" / "prototypes" / "prototype-proof-report.md"

END_MAP = {
    "PC": "pc",
    "PDA": "pda",
    "PAD": "pad",
    "H5": "h5",
}


@dataclass(frozen=True)
class MatrixSource:
    row_no: str
    story_id: str
    title: str
    end: str
    reason: str
    slug: str


@dataclass(frozen=True)
class OcrResult:
    chars: int
    hits: tuple[str, ...]
    miss: tuple[str, ...]
    total: int
    png_md5: str


def _load_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    try:
        import tomllib

        return tomllib.loads(text)
    except ModuleNotFoundError:
        import tomli

        return tomli.loads(text)


def _md5(path: Path) -> str:
    h = hashlib.md5()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _normalize_ocr(text: str) -> str:
    return re.sub(r"\s+", " ", text).upper()


def _ocr_extract(png: Path) -> str:
    out_prefix = f"/tmp/_proof_ocr_{png.stem}"
    subprocess.run(
        ["tesseract", str(png), out_prefix, "-l", "eng"],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    txt = Path(f"{out_prefix}.txt")
    if not txt.exists():
        return ""
    return txt.read_text(encoding="utf-8", errors="ignore")


def _check_tesseract() -> None:
    result = subprocess.run(["tesseract", "--version"], capture_output=True, text=True, timeout=5, check=False)
    if result.returncode != 0:
        raise RuntimeError("tesseract 未安装或不可用")


def _ocr_result(png: Path, keywords: list[str], *, skip_ocr: bool) -> OcrResult:
    if not png.exists():
        return OcrResult(chars=0, hits=(), miss=tuple(keywords), total=len(keywords), png_md5="missing")
    png_hash = _md5(png)
    if skip_ocr:
        return OcrResult(chars=-1, hits=(), miss=tuple(keywords), total=len(keywords), png_md5=png_hash)

    text = _ocr_extract(png)
    norm = _normalize_ocr(text)
    chars = sum(1 for c in text if c.isalnum() or c in "-_./")
    hits = tuple(kw for kw in keywords if kw.upper() in norm)
    miss = tuple(kw for kw in keywords if kw.upper() not in norm)
    return OcrResult(chars=chars, hits=hits, miss=miss, total=len(keywords), png_md5=png_hash)


def _slug_for(story_id: str, end: str) -> str:
    return f"{end}-{story_id[3:].lower()}"


def _matrix_sources() -> dict[str, MatrixSource]:
    out: dict[str, MatrixSource] = {}
    if not MATRIX_MD.exists():
        return out

    for line in MATRIX_MD.read_text(encoding="utf-8").splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 7 or not cells[0].isdigit() or not cells[1].startswith("US-"):
            continue
        row_no, story_id, title, raw_ends, exempt, reason = cells[:6]
        if "豁免" in exempt:
            continue
        for raw_end in raw_ends.split("+"):
            raw_end = raw_end.strip()
            end = END_MAP.get(raw_end)
            if not end:
                continue
            slug = _slug_for(story_id, end)
            out[slug] = MatrixSource(
                row_no=row_no,
                story_id=story_id,
                title=title,
                end=end,
                reason=re.sub(r"<[^>]+>", "", reason),
                slug=slug,
            )
    return out


def _short(text: str, limit: int = 54) -> str:
    text = re.sub(r"\s+", " ", text).strip()
    if len(text) <= limit:
        return text
    return text[: limit - 1] + "…"


def _md_cell(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", "<br>")


def _semantic_status(
    spec: dict[str, str] | None,
    *,
    reused_slug: str | None,
    blueprints: dict[str, dict[str, list[str]]],
    profiles: list[tuple[re.Pattern[str], list[str]]],
) -> tuple[str, bool]:
    if spec is None:
        return ("手工页：无生成模型", True)
    if reused_slug:
        return (f"复用 `{reused_slug}`：跳过生成模型", True)

    source_text = f'{spec["title"]}{spec["reason"]}{spec["group"]}'
    model_text = _model_text_for_spec(spec, blueprints, profiles)
    source_terms = [term for term in SEMANTIC_TERMS if term.pattern.search(source_text)]
    if not source_terms:
        return ("无规则主题词", True)

    hit_names: list[str] = []
    missing_names: list[str] = []
    for term in source_terms:
        if any(token in model_text for token in term.model_terms):
            hit_names.append(term.name)
        else:
            missing_names.append(term.name)

    ok = not missing_names
    if ok:
        return (f"OK {len(hit_names)}/{len(source_terms)}: {', '.join(hit_names)}", True)
    return (f"缺失 {len(missing_names)}/{len(source_terms)}: {', '.join(missing_names)}", False)


def _review_status(snap: dict) -> tuple[str, bool]:
    reviewed_by = str(snap.get("reviewed_by", "")).strip()
    reviewed_at = str(snap.get("reviewed_at", "")).strip()
    ok = bool(reviewed_by and reviewed_at and reviewed_by.lower() not in {"todo", "tbd", "?", "-"})
    return (f"{reviewed_by or '-'} / {reviewed_at or '-'}", ok)


def _matrix_source_text(tab: str, snap: dict, matrix: dict[str, MatrixSource]) -> str:
    src = matrix.get(tab)
    if src:
        return f"R{src.row_no} `{src.story_id}` {src.end.upper()} · {_short(src.title)}"
    related = str(snap.get("related_story", "")).strip()
    if tab == "gallery":
        return "组件库走查页（非矩阵行）"
    if related:
        return f"手工高保真页 · {_short(related)}"
    return "未找到矩阵来源"


def _ocr_status_text(result: OcrResult) -> tuple[str, bool]:
    if result.png_md5 == "missing":
        return ("PNG 缺失", False)
    if result.chars == -1:
        return (f"跳过 OCR · md5 `{result.png_md5[:8]}`", True)
    if result.total == 0:
        return (f"{result.chars} chars · 未配置关键词 · md5 `{result.png_md5[:8]}`", result.chars >= 100)
    ok = len(result.hits) > 0 and result.chars >= 100
    return (f"{result.chars} chars · kw {len(result.hits)}/{result.total} · md5 `{result.png_md5[:8]}`", ok)


def _render_report(rows: list[dict], summary: dict) -> str:
    generated_at = dt.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    lines = [
        "# Prototype Proof Report",
        "",
        f"> 生成时间：{generated_at}",
        "> 用途：作为高保真原型走查验收依据。报告由脚本生成，不手工维护。",
        "",
        "## 证据口径",
        "",
        "- 原型清单：`governance/visual-baselines/manifest.toml` 的 `[[snapshots]]`。",
        "- 矩阵来源：`docs/prototypes/prototype-matrix-r3.md` 的非豁免 UI 行，按端展开到 tab。",
        "- 模型关键词：`scripts/governance/check_prototype_fidelity.py` 的 `SEMANTIC_TERMS`，对全量矩阵生成页检查标题/原因中的高风险主题词是否进入生成模型。",
        "- 截图 OCR：对 `governance/visual-baselines/*.png` 运行 tesseract `eng`，并按 manifest 的 `expected_keywords` 统计命中。",
        "- Review 签字：manifest 的 `reviewed_by` / `reviewed_at`。",
        "",
        "## 汇总",
        "",
        f"- 原型 tab 总数：{summary['total']}",
        f"- 矩阵原型：{summary['matrix_rows']}；手工/组件页：{summary['hand_built_rows']}",
        f"- 模型关键词通过：{summary['semantic_ok']}/{summary['total']}",
        f"- 截图 OCR 通过：{summary['ocr_ok']}/{summary['total']}",
        f"- Review 签字完整：{summary['review_ok']}/{summary['total']}",
        f"- 全部证据通过：{summary['all_ok']}/{summary['total']}",
        "",
        "## 明细",
        "",
        "| # | 矩阵来源 | Tab | 模型关键词命中 | 截图 OCR | Review 签字 | 状态 |",
        "|---:|---|---|---|---|---|---|",
    ]
    for idx, row in enumerate(rows, start=1):
        status = "OK" if row["ok"] else "CHECK"
        lines.append(
            "| "
            + " | ".join(
                [
                    str(idx),
                    _md_cell(row["matrix_source"]),
                    f"`{row['tab']}`",
                    _md_cell(row["semantic"]),
                    _md_cell(row["ocr"]),
                    _md_cell(row["review"]),
                    status,
                ]
            )
            + " |"
        )
    lines.append("")
    lines.append("## 复跑命令")
    lines.append("")
    lines.append("```bash")
    lines.append("python3 scripts/governance/generate_prototype_proof_report.py")
    lines.append("python3 scripts/governance/governance_checks.py --tier T1")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def generate(*, output: Path, skip_ocr: bool) -> dict:
    if not skip_ocr:
        _check_tesseract()

    manifest = _load_toml(MANIFEST_TOML)
    snapshots = manifest.get("snapshots", [])
    matrix = _matrix_sources()
    specs = {spec["slug"]: spec for spec in _specs_from_specs()}
    tabs_text = TABS_FILE.read_text(encoding="utf-8")
    legacy_mappings = _tabs_legacy_mappings(tabs_text)
    blueprints = _blueprints()
    profiles = _keyword_profiles()

    rows: list[dict] = []
    for snap in snapshots:
        tab = snap["tab"]
        spec = specs.get(tab)
        legacy_key = f'{spec["end"]}-{spec["storyId"].lower()}' if spec else ""
        reused_slug = legacy_mappings.get(legacy_key)
        semantic_text, semantic_ok = _semantic_status(
            spec,
            reused_slug=reused_slug,
            blueprints=blueprints,
            profiles=profiles,
        )
        ocr_result = _ocr_result(BASELINE_DIR / snap["file"], list(snap.get("expected_keywords", [])), skip_ocr=skip_ocr)
        ocr_text, ocr_ok = _ocr_status_text(ocr_result)
        review_text, review_ok = _review_status(snap)
        matrix_source = _matrix_source_text(tab, snap, matrix)
        rows.append(
            {
                "tab": tab,
                "matrix_source": matrix_source,
                "semantic": semantic_text,
                "semantic_ok": semantic_ok,
                "ocr": ocr_text,
                "ocr_ok": ocr_ok,
                "review": review_text,
                "review_ok": review_ok,
                "ok": semantic_ok and ocr_ok and review_ok,
            }
        )

    summary = {
        "total": len(rows),
        "matrix_rows": sum(1 for row in rows if row["tab"] in matrix),
        "hand_built_rows": sum(1 for row in rows if row["tab"] not in matrix),
        "semantic_ok": sum(1 for row in rows if row["semantic_ok"]),
        "ocr_ok": sum(1 for row in rows if row["ocr_ok"]),
        "review_ok": sum(1 for row in rows if row["review_ok"]),
        "all_ok": sum(1 for row in rows if row["ok"]),
    }
    report = _render_report(rows, summary)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(report, encoding="utf-8")
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--output", default=str(REPORT_MD), help="输出 Markdown 路径")
    parser.add_argument("--skip-ocr", action="store_true", help="跳过 OCR，仅生成结构报告")
    parser.add_argument("--json", action="store_true", help="输出 JSON 摘要")
    args = parser.parse_args()

    try:
        summary = generate(output=Path(args.output), skip_ocr=args.skip_ocr)
    except Exception as e:  # noqa: BLE001
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}, ensure_ascii=False))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({"status": "pass", "summary": summary, "output": args.output}, ensure_ascii=False, indent=2))
    else:
        print(f"✓ prototype proof report generated: {Path(args.output).relative_to(REPO_ROOT)}")
        print(
            "  "
            f"tabs={summary['total']} "
            f"semantic={summary['semantic_ok']}/{summary['total']} "
            f"ocr={summary['ocr_ok']}/{summary['total']} "
            f"review={summary['review_ok']}/{summary['total']} "
            f"all={summary['all_ok']}/{summary['total']}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
