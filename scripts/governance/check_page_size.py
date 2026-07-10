#!/usr/bin/env python3
"""check_page_size.py — 页面/源码文件大小约束（600 警告 / 800 门禁）

类别：6. 原型治理
Tier：T1（< 10s）
输入：prototypes / apps 页面、packages/ui/src/business 共享业务组件、生产源码
输出：人类可读 + --json
退出码：0 通过 / 1 违规（≥ 800 行）/ 2 脚本错误

校验项（对照 docs/frontend-coding-standards.md §页面级大小约束）：
- 单页面文件 ≥ 600 行 → warning（提示提取组件）
- 单页面文件 ≥ 800 行 → error（强制提取组件）
- 原型运行时支撑组件同样受约束，防止 UniversalPrototypePage 类模板绕过治理
- 共享业务组件同样受约束，防止 DataGrid 等公共控件堆成单文件
- 生产源码 ≥ 800 行必须被发现；历史遗留文件只允许不增长，新增/增长立即失败
- 禁止把大文件压成 base64 / 超长单行 payload 来绕过行数门禁

豁免方式：文件顶部加 `@governance: skip-page-size` 注释 + 理由

不覆盖：
- 单组件复杂度（应交由 cyclomatic complexity 检查）
- 跨文件累计行数
- 自动生成文件
"""
from __future__ import annotations

import argparse
import ast
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10
    import tomli as tomllib

_THIS = Path(__file__).resolve()
REPO_ROOT = _THIS.parent.parent.parent
PAGE_DIRS = (
    REPO_ROOT / "prototypes" / "src" / "pages",
    REPO_ROOT / "prototypes" / "src" / "prototype-kit",
    REPO_ROOT / "apps" / "web-admin" / "src" / "pages",
    REPO_ROOT / "apps" / "pda-mobile" / "src" / "pages",
    REPO_ROOT / "packages" / "ui" / "src" / "business",
)
CHECK_SUFFIXES = (".tsx",)
SOURCE_DIRS = (
    REPO_ROOT / "backend" / "crates",
    REPO_ROOT / "apps",
    REPO_ROOT / "packages",
    REPO_ROOT / "prototypes",
    REPO_ROOT / "scripts",
)
SOURCE_SUFFIXES = (".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".py")
GENERATED_FILES = {
    "packages/api-client/src/schema.ts",
}
BASELINE_PATH = REPO_ROOT / "governance" / "source-size-baseline.toml"


def _parse_baseline(text: str) -> dict[str, int]:
    values = tomllib.loads(text).get("files", {})
    return {str(path): int(limit) for path, limit in values.items()}


def _load_baseline() -> dict[str, int]:
    return _parse_baseline(BASELINE_PATH.read_text(encoding="utf-8"))


# ponytail: historical debt baseline; remove an entry as soon as that file is split.
LEGACY_OVERSIZED_SOURCE_BASELINE = _load_baseline()

WARN_THRESHOLD = 600
ERROR_THRESHOLD = 800
MAX_PAYLOAD_LINE_LENGTH = 4_000
SKIP_TAG = "@governance: skip-page-size"
FORBIDDEN_PAYLOAD_MARKERS = (
    "_IMPL" + "_SOURCE_B64",
    "Generated implementation " + "loader",
)
PYTHON_DECODER_NAMES = {
    "a2b_base64",
    "b64decode",
    "decodebytes",
    "standard_b64decode",
    "urlsafe_b64decode",
}
PYTHON_EXECUTION_NAMES = {"exec", "eval", "compile"}
JS_PAYLOAD_DECODER_RE = re.compile(
    r"(?:Buffer\s*(?:\.\s*from|\[\s*[\"'`]from[\"'`]\s*\])\s*\([\s\S]{0,500}?[\"'`]base64(?:url)?[\"'`]|\batob\s*\()"
)
JS_PAYLOAD_EXECUTION_RE = re.compile(
    r"(?:\b(?:eval|Function|runInThisContext|runInNewContext|runInContext)|"
    r"\bglobalThis\s*\[\s*[\"'`](?:eval|Function)[\"'`]\s*\])\s*\("
)
JS_IDENTIFIER_RE = r"[A-Za-z_$][A-Za-z0-9_$]*"
JS_DIRECT_PAYLOAD_RE = re.compile(
    JS_PAYLOAD_EXECUTION_RE.pattern + r"\s*" + JS_PAYLOAD_DECODER_RE.pattern
)
JS_DECODER_ASSIGNMENT_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*"
    + JS_PAYLOAD_DECODER_RE.pattern
)
JS_DERIVED_ASSIGNMENT_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*({JS_IDENTIFIER_RE})(?:\s*\.\s*toString\s*\([^;]*\))?"
)
JS_BUFFER_ALIAS_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*"
    r"Buffer\s*(?:\.\s*from|\[\s*[\"'`]from[\"'`]\s*\])\s*(?:;|$)"
)
JS_ENCODING_ASSIGNMENT_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*"
    r"[\"'`]base64(?:url)?[\"'`]"
)
JS_BUFFER_CALL_ASSIGNMENT_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*"
    r"Buffer\s*(?:\.\s*from|\[\s*[\"'`]from[\"'`]\s*\])\s*\(([^;]{0,500})\)"
)
JS_ALIAS_CALL_ASSIGNMENT_RE = re.compile(
    rf"(?<![.\w$])(?:(?:const|let|var)\s+)?({JS_IDENTIFIER_RE})\s*=\s*"
    rf"({JS_IDENTIFIER_RE})\s*\(([^;]{{0,500}})\)"
)


def _count_effective_lines(path: Path) -> int:
    """计算有效代码行（去掉空行 + 纯注释行，但保留 JSDoc 头部计入"""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    return len([l for l in lines if l.strip()])


def _check_file(path: Path) -> tuple[int, str | None, bool]:
    """Returns (line_count, severity, is_error)"""
    text = path.read_text(encoding="utf-8")
    if _has_valid_skip_header(text):
        return (0, None, False)  # 豁免
    lines = _count_effective_lines(path)
    if lines >= ERROR_THRESHOLD:
        return (lines, "error", True)
    if lines >= WARN_THRESHOLD:
        return (lines, "warning", False)
    return (lines, None, False)


def _has_valid_skip_header(text: str) -> bool:
    pattern = re.compile(
        r"^\s*(?://|#|/\*+|\*)\s*@governance: skip-page-size(?:\s*[:=-]\s*|\s+)"
        r"(?P<reason>\S.*?)(?:\s*\*/)?\s*$"
    )
    in_block_comment = False
    for line in text.splitlines()[:10]:
        stripped = line.strip()
        if not stripped:
            continue
        if in_block_comment:
            if pattern.match(line):
                return True
            if "*/" in stripped:
                in_block_comment = False
            continue
        if stripped.startswith(("//", "#")):
            if pattern.match(line):
                return True
            continue
        if stripped.startswith("/*"):
            if pattern.match(line):
                return True
            in_block_comment = "*/" not in stripped[2:]
            continue
        break
    return False


def _payload_errors(path: Path, text: str) -> list[str]:
    rel = path.relative_to(REPO_ROOT).as_posix()
    errors: list[str] = []
    markers = [marker for marker in FORBIDDEN_PAYLOAD_MARKERS if marker in text]
    if path.suffix == ".py" and _python_loads_decoded_payload(text):
        markers.append("Python decoder + dynamic execution")
    if path.suffix in {".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"}:
        if _javascript_loads_decoded_payload(text):
            markers.append("JavaScript decoder + dynamic execution")
    if markers:
        errors.append(f"{rel}: 禁止使用生成实现载荷绕过行数门禁（命中 {', '.join(markers)}）")
    for index, line in enumerate(text.splitlines(), start=1):
        if len(line) > MAX_PAYLOAD_LINE_LENGTH:
            errors.append(
                f"{rel}:{index}: 单行 {len(line)} 字符 > {MAX_PAYLOAD_LINE_LENGTH}，疑似压缩 payload，必须真实拆分"
            )
            break
    return errors


def _javascript_loads_decoded_payload(text: str) -> bool:
    source = _javascript_code_for_scan(text)
    if JS_DIRECT_PAYLOAD_RE.search(source):
        return True

    decoded_names = {match.group(1) for match in JS_DECODER_ASSIGNMENT_RE.finditer(source)}
    decoder_aliases = {match.group(1) for match in JS_BUFFER_ALIAS_RE.finditer(source)}
    encoding_names = {match.group(1) for match in JS_ENCODING_ASSIGNMENT_RE.finditer(source)}
    for match in JS_BUFFER_CALL_ASSIGNMENT_RE.finditer(source):
        target, arguments = match.groups()
        if _javascript_uses_base64_encoding(arguments, encoding_names):
            decoded_names.add(target)
    for match in JS_ALIAS_CALL_ASSIGNMENT_RE.finditer(source):
        target, decoder, arguments = match.groups()
        if decoder in decoder_aliases and _javascript_uses_base64_encoding(arguments, encoding_names):
            decoded_names.add(target)
    changed = True
    while changed:
        changed = False
        for match in JS_DERIVED_ASSIGNMENT_RE.finditer(source):
            target, derived_from = match.groups()
            if derived_from in decoded_names and target not in decoded_names:
                decoded_names.add(target)
                changed = True

    return any(
        re.search(
            JS_PAYLOAD_EXECUTION_RE.pattern
            + rf"\s*{re.escape(name)}(?:\b|\s*\.)",
            source,
        )
        for name in decoded_names
    )


def _javascript_uses_base64_encoding(arguments: str, encoding_names: set[str]) -> bool:
    if re.search(r"[\"'`]base64(?:url)?[\"'`]", arguments):
        return True
    return any(re.search(rf"\b{re.escape(name)}\b", arguments) for name in encoding_names)


def _javascript_code_for_scan(text: str) -> str:
    """Mask comments and non-syntax strings before regex data-flow checks."""
    keep_literals = {"base64", "base64url", "eval", "from", "Function"}
    output: list[str] = []
    index = 0
    while index < len(text):
        current = text[index]
        next_char = text[index + 1] if index + 1 < len(text) else ""
        if current == "/" and next_char in {"/", "*"}:
            block = next_char == "*"
            output.extend("  ")
            index += 2
            while index < len(text):
                if block and text[index:index + 2] == "*/":
                    output.extend("  ")
                    index += 2
                    break
                if not block and text[index] == "\n":
                    output.append("\n")
                    index += 1
                    break
                output.append("\n" if text[index] == "\n" else " ")
                index += 1
            continue
        if current in {"'", '"', "`"}:
            quote = current
            start = index
            index += 1
            escaped = False
            while index < len(text):
                char = text[index]
                index += 1
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    break
            literal = text[start + 1:index - 1]
            if literal in keep_literals:
                output.append(text[start:index])
            else:
                output.append(quote + quote)
                output.extend("\n" for char in literal if char == "\n")
            continue
        output.append(current)
        index += 1
    return "".join(output)


def _python_loads_decoded_payload(text: str) -> bool:
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False

    decoder_aliases = set(PYTHON_DECODER_NAMES)
    module_aliases = {"base64", "binascii"}
    codec_decoder_aliases: set[str] = set()
    codec_module_aliases = {"codecs"}
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module in {"base64", "binascii"}:
            for name in node.names:
                if name.name in PYTHON_DECODER_NAMES:
                    decoder_aliases.add(name.asname or name.name)
        elif isinstance(node, ast.ImportFrom) and node.module == "codecs":
            for name in node.names:
                if name.name == "decode":
                    codec_decoder_aliases.add(name.asname or name.name)
        elif isinstance(node, ast.Import):
            for name in node.names:
                if name.name in {"base64", "binascii"}:
                    module_aliases.add(name.asname or name.name)
                elif name.name == "codecs":
                    codec_module_aliases.add(name.asname or name.name)

    changed = True
    while changed:
        changed = False
        for node in ast.walk(tree):
            value: ast.AST | None = None
            targets: list[ast.AST] = []
            if isinstance(node, ast.Assign):
                value = node.value
                targets = node.targets
            elif isinstance(node, ast.AnnAssign):
                value = node.value
                targets = [node.target]
            if value is None or not _is_decoder_reference(value, decoder_aliases, module_aliases):
                continue
            for target in targets:
                for name in (child.id for child in ast.walk(target) if isinstance(child, ast.Name)):
                    if name not in decoder_aliases:
                        decoder_aliases.add(name)
                        changed = True

    decoded_names: set[str] = set()
    for node in ast.walk(tree):
        value: ast.AST | None = None
        targets: list[ast.AST] = []
        if isinstance(node, ast.Assign):
            value = node.value
            targets = node.targets
        elif isinstance(node, ast.AnnAssign):
            value = node.value
            targets = [node.target]
        if value is not None and _contains_decoder_call(
            value,
            decoder_aliases,
            module_aliases,
            codec_decoder_aliases,
            codec_module_aliases,
        ):
            for target in targets:
                decoded_names.update(
                    child.id for child in ast.walk(target) if isinstance(child, ast.Name)
                )

    for node in ast.walk(tree):
        if not isinstance(node, ast.Call) or not _is_execution_call(node.func):
            continue
        for argument in [*node.args, *(keyword.value for keyword in node.keywords)]:
            if _contains_decoder_call(
                argument,
                decoder_aliases,
                module_aliases,
                codec_decoder_aliases,
                codec_module_aliases,
            ):
                return True
            if any(
                isinstance(child, ast.Name) and child.id in decoded_names
                for child in ast.walk(argument)
            ):
                return True
    return False


def _is_decoder_reference(
    node: ast.AST,
    decoder_aliases: set[str],
    module_aliases: set[str],
) -> bool:
    if isinstance(node, ast.Name):
        return node.id in decoder_aliases
    return (
        isinstance(node, ast.Attribute)
        and node.attr in PYTHON_DECODER_NAMES
        and _imported_module_name(node.value) in module_aliases
    )


def _contains_decoder_call(
    node: ast.AST,
    decoder_aliases: set[str],
    module_aliases: set[str],
    codec_decoder_aliases: set[str],
    codec_module_aliases: set[str],
) -> bool:
    return any(
        isinstance(child, ast.Call)
        and _is_decoder_call(
            child,
            decoder_aliases,
            module_aliases,
            codec_decoder_aliases,
            codec_module_aliases,
        )
        for child in ast.walk(node)
    )


def _is_decoder_call(
    node: ast.Call,
    decoder_aliases: set[str],
    module_aliases: set[str],
    codec_decoder_aliases: set[str],
    codec_module_aliases: set[str],
) -> bool:
    function = node.func
    if isinstance(function, ast.Call) and _is_decoder_getattr(
        function,
        node,
        module_aliases,
        codec_module_aliases,
    ):
        return True
    if isinstance(function, ast.Name):
        return function.id in decoder_aliases or (
            function.id in codec_decoder_aliases and _uses_base64_codec(node)
        )
    if not isinstance(function, ast.Attribute):
        return False
    module_name = _imported_module_name(function.value)
    return (
        function.attr in PYTHON_DECODER_NAMES and module_name in module_aliases
    ) or (
        function.attr == "decode"
        and module_name in codec_module_aliases
        and _uses_base64_codec(node)
    )


def _is_decoder_getattr(
    function: ast.Call,
    decoder_call: ast.Call,
    module_aliases: set[str],
    codec_module_aliases: set[str],
) -> bool:
    if (
        not isinstance(function.func, ast.Name)
        or function.func.id != "getattr"
        or len(function.args) < 2
        or not isinstance(function.args[1], ast.Constant)
        or not isinstance(function.args[1].value, str)
    ):
        return False
    module_name = _imported_module_name(function.args[0])
    attribute = function.args[1].value
    return (
        module_name in module_aliases and attribute in PYTHON_DECODER_NAMES
    ) or (
        module_name in codec_module_aliases
        and attribute == "decode"
        and _uses_base64_codec(decoder_call)
    )


def _imported_module_name(node: ast.AST) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Name)
        and node.func.id == "__import__"
        and node.args
        and isinstance(node.args[0], ast.Constant)
        and isinstance(node.args[0].value, str)
    ):
        return node.args[0].value
    return None


def _is_execution_call(function: ast.expr) -> bool:
    if isinstance(function, ast.Name):
        return function.id in PYTHON_EXECUTION_NAMES
    return isinstance(function, ast.Attribute) and function.attr in PYTHON_EXECUTION_NAMES


def _uses_base64_codec(node: ast.Call) -> bool:
    codecs: list[ast.AST] = []
    if len(node.args) >= 2:
        codecs.append(node.args[1])
    codecs.extend(
        keyword.value for keyword in node.keywords if keyword.arg == "encoding"
    )
    return any(
        isinstance(codec, ast.Constant)
        and isinstance(codec.value, str)
        and codec.value.lower() == "base64"
        for codec in codecs
    )


def run() -> tuple[list[str], list[str]]:
    """Returns (errors, warnings)"""
    errors: list[str] = []
    warnings: list[str] = []
    errors.extend(
        _baseline_policy_errors(
            LEGACY_OVERSIZED_SOURCE_BASELINE,
            _load_previous_baseline(),
        )
    )
    page_files: list[Path] = []
    for page_dir in PAGE_DIRS:
        if page_dir.exists():
            page_files.extend(path for path in page_dir.rglob("*") if path.suffix in CHECK_SUFFIXES)
    page_file_set = set(page_files)
    for f in sorted(page_files):
        if ".stories." in f.name or ".spec." in f.name or ".test." in f.name:
            continue
        errors.extend(_payload_errors(f, f.read_text(encoding="utf-8")))
        lines, severity, is_error = _check_file(f)
        rel = f.relative_to(REPO_ROOT).as_posix()
        if is_error:
            errors.append(f"{rel}: {lines} 行 ≥ {ERROR_THRESHOLD}（门禁，必须提取组件或加 {SKIP_TAG} 豁免）")
        elif severity == "warning":
            warnings.append(f"{rel}: {lines} 行 ≥ {WARN_THRESHOLD}（警告，建议提取 PageHeader/DataTable/FilterBar 等）")

    source_files: list[Path] = []
    for source_dir in SOURCE_DIRS:
        if source_dir.exists():
            source_files.extend(path for path in source_dir.rglob("*") if path.suffix in SOURCE_SUFFIXES)
    for f in sorted(source_files):
        if f in page_file_set:
            continue
        if any(part in f.parts for part in ("node_modules", "target", "dist", ".vite-temp")):
            continue
        rel = f.relative_to(REPO_ROOT).as_posix()
        if rel in GENERATED_FILES:
            continue
        text = f.read_text(encoding="utf-8")
        errors.extend(_payload_errors(f, text))
        lines, severity, is_error = _check_file(f)
        baseline = LEGACY_OVERSIZED_SOURCE_BASELINE.get(rel)
        if is_error and baseline is not None and lines <= baseline:
            warnings.append(f"{rel}: {lines} 行 ≥ {ERROR_THRESHOLD}（历史遗留，已进入拆分基线，不允许增长）")
        elif is_error and baseline is not None:
            errors.append(f"{rel}: {lines} 行 > 历史基线 {baseline}（门禁，必须拆分或更新有理由豁免）")
        elif is_error:
            errors.append(f"{rel}: {lines} 行 ≥ {ERROR_THRESHOLD}（门禁，必须拆分或加 {SKIP_TAG} 豁免）")
        elif severity == "warning":
            warnings.append(f"{rel}: {lines} 行 ≥ {WARN_THRESHOLD}（警告，建议拆分）")
    return (errors, warnings)


def _load_previous_baseline() -> dict[str, int] | None:
    relative = BASELINE_PATH.relative_to(REPO_ROOT).as_posix()
    result = subprocess.run(
        ["git", "show", f"HEAD:{relative}"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return _parse_baseline(result.stdout) if result.returncode == 0 else None


def _baseline_policy_errors(
    current: dict[str, int],
    previous: dict[str, int] | None,
) -> list[str]:
    if previous is None:
        return []
    errors: list[str] = []
    for path, limit in current.items():
        if path not in previous:
            errors.append(f"{path}: 禁止新增历史超限基线，必须拆分或使用带理由的文件头豁免")
        elif limit > previous[path]:
            errors.append(f"{path}: 历史超限基线禁止从 {previous[path]} 放宽到 {limit}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    try:
        errors, warnings = run()
    except Exception as e:
        if args.json:
            print(json.dumps({"status": "error", "message": str(e)}))
        else:
            print(f"[ERROR] {e}", file=sys.stderr)
        sys.exit(2)

    if args.json:
        print(json.dumps({
            "status": "fail" if errors else "pass",
            "errors": errors,
            "warnings": warnings,
            "ok": not errors,
            "thresholds": {"warning": WARN_THRESHOLD, "error": ERROR_THRESHOLD},
        }))
    else:
        if errors:
            print(f"✗ check_page_size: {len(errors)} 项门禁违规")
            for e in errors:
                print(f"  - {e}")
        if warnings:
            tag = "⚠" if not errors else " "
            print(f"{tag} check_page_size: {len(warnings)} 项警告")
            for w in warnings:
                print(f"  - {w}")
        if not errors and not warnings:
            print(f"✓ check_page_size: 通过（阈值 {WARN_THRESHOLD}/{ERROR_THRESHOLD}）")

    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
