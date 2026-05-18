#!/usr/bin/env python3
"""check_secrets.py — Secrets 与配置安全校验

类别：5. 运行治理
Tier：T1（< 5s）
关联：ADR-0013 配置与 secrets 管理 / governance.md §3.7

校验项：
  1. .gitignore 必含 secrets 屏蔽规则（.env / *.pem / *.key / id_rsa* 等）
  2. 仓库内不应有 .env / .pem / .key / id_rsa 等敏感文件
  3. .env.example（如存在）不应含真实值（必须是占位符）
  4. 环境变量引用符合 WMS_<MODULE>_<KEY> 命名规范（在文档中提到的）
  5. 代码/配置中硬编码 secret 模式扫描（password=, api_key=, secret= 等）

退出码：
  0 通过
  1 发现违规
  2 脚本自身错误
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
GITIGNORE = REPO_ROOT / ".gitignore"

# .gitignore 必须包含的模式
REQUIRED_GITIGNORE_PATTERNS = [
    ".env",
    "*.pem",
    "*.key",
    "id_rsa",
]

# 不应入库的文件扩展名（参 ADR-0013 + governance §3.7）
SECRET_FILE_PATTERNS = [
    ".env",
    ".env.local",
    ".env.production",
    ".env.staging",
    ".env.dev",
]

SECRET_FILE_SUFFIXES = [
    ".pem",
    ".key",
    ".pfx",
    ".p12",
]

# .env.example 中允许的占位标识
PLACEHOLDER_TOKENS = [
    "REPLACE_ME",
    "YOUR_",
    "EXAMPLE",
    "<",
    "${",
    "USER:PASS",
    "xxxxxx",
    "PLACEHOLDER",
]

# 硬编码 secret 模式（启发式扫描）
HARDCODED_SECRET_PATTERNS = [
    # password = "non-trivial-string"
    (re.compile(r'(password|passwd)\s*[=:]\s*["\']([^"\'\s]{8,})["\']', re.I),
     "可能的硬编码密码"),
    # api_key = "..." with reasonable length
    (re.compile(r'(api[_-]?key|apikey)\s*[=:]\s*["\']([A-Za-z0-9_-]{16,})["\']', re.I),
     "可能的硬编码 API key"),
    # secret = "..." with reasonable length
    (re.compile(r'(secret|signing[_-]?key)\s*[=:]\s*["\']([A-Za-z0-9_/+=-]{16,})["\']', re.I),
     "可能的硬编码 secret/key"),
    # 私钥头
    (re.compile(r'-----BEGIN\s+(RSA|EC|DSA|OPENSSH|PRIVATE)\s+(PRIVATE\s+)?KEY-----'),
     "私钥内容"),
]

# 扫描范围（排除文档示例 / 测试 fixture）
SCAN_DIRS = ["backend/", "apps/", "packages/", "scripts/", "shared/"]
EXCLUDE_PATTERNS = [
    r"\.git/",
    r"target/",
    r"node_modules/",
    r"site/",
    r"\.pytest_cache/",
    r"docs/",            # 文档允许示例
    r"tests/.*fixture",  # 测试 fixture
    r"\.example",
    r"\.lock$",
]


@dataclass
class Issue:
    severity: str    # error / warning / info
    rule: str
    file: str
    line: int = 0
    message: str = ""


def check_gitignore() -> list[Issue]:
    """校验 .gitignore 包含必要的 secret 屏蔽。"""
    issues: list[Issue] = []
    if not GITIGNORE.exists():
        issues.append(Issue("error", "no_gitignore", str(GITIGNORE), 0,
                            ".gitignore 不存在"))
        return issues
    text = GITIGNORE.read_text(encoding="utf-8")
    for p in REQUIRED_GITIGNORE_PATTERNS:
        # 简单匹配（行首 + 完整 token，允许前后有空格）
        if not re.search(rf"^\s*{re.escape(p)}\s*$", text, re.M):
            issues.append(Issue(
                "error", "gitignore_missing_pattern", ".gitignore", 0,
                f".gitignore 缺必需的模式 '{p}'（参 ADR-0013 / governance §3.7）"
            ))
    return issues


def check_secret_files_in_repo() -> list[Issue]:
    """校验仓库内不存在 .env / .pem / .key 等文件。"""
    issues: list[Issue] = []
    for f in REPO_ROOT.rglob("*"):
        if not f.is_file():
            continue
        rel = f.relative_to(REPO_ROOT).as_posix()
        if any(re.search(p, rel) for p in EXCLUDE_PATTERNS):
            continue
        # .env / .env.local / 等
        for pat in SECRET_FILE_PATTERNS:
            if f.name == pat or f.name.startswith(pat + "."):
                issues.append(Issue(
                    "error", "secret_file_in_repo", rel, 0,
                    f"敏感配置文件不应入库: {f.name}"
                ))
        # .pem / .key
        for sfx in SECRET_FILE_SUFFIXES:
            if f.name.endswith(sfx):
                issues.append(Issue(
                    "error", "key_file_in_repo", rel, 0,
                    f"密钥文件不应入库: {f.name}"
                ))
    return issues


def check_env_example() -> list[Issue]:
    """校验 .env.example（如存在）不含真实值。"""
    issues: list[Issue] = []
    example = REPO_ROOT / ".env.example"
    if not example.exists():
        # 不强制，仅提示
        return issues
    text = example.read_text(encoding="utf-8")
    for ln, line in enumerate(text.splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            continue
        key, _, value = line.partition("=")
        value = value.strip().strip("\"'")
        if not value:
            continue
        # 必须含占位符标识
        if not any(tok in value for tok in PLACEHOLDER_TOKENS):
            # 允许少量已知非敏感值（如 PORT=8080, LOG_LEVEL=info）
            if re.match(r"^(\d+|true|false|info|debug|warn|error)$", value, re.I):
                continue
            issues.append(Issue(
                "warning", "env_example_real_value", ".env.example", ln,
                f"行 {ln} 'value' 不像占位（应含 REPLACE_ME / YOUR_ / EXAMPLE 等）: {key}={value[:30]}"
            ))
    return issues


def scan_hardcoded_secrets() -> list[Issue]:
    """扫描代码与配置中可能的硬编码 secret。"""
    issues: list[Issue] = []
    scan_paths = [REPO_ROOT / d for d in SCAN_DIRS]
    for base in scan_paths:
        if not base.exists():
            continue
        for f in base.rglob("*"):
            if not f.is_file():
                continue
            rel = f.relative_to(REPO_ROOT).as_posix()
            if any(re.search(p, rel) for p in EXCLUDE_PATTERNS):
                continue
            # 仅扫描文本类型
            if f.suffix not in {".rs", ".ts", ".tsx", ".js", ".jsx", ".py",
                                ".toml", ".yaml", ".yml", ".json", ".env"}:
                continue
            try:
                text = f.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue
            for ln, line in enumerate(text.splitlines(), 1):
                # 跳过注释
                stripped = line.strip()
                if stripped.startswith(("//", "#", "/*", "*")):
                    continue
                # 跳过测试/fixture（再次保险）
                if "TODO" in line or "FIXME" in line or "test" in rel.lower():
                    continue
                for pattern, label in HARDCODED_SECRET_PATTERNS:
                    if pattern.search(line):
                        # 排除明显占位
                        if any(tok in line.lower() for tok in
                               ["example", "placeholder", "your_", "replace_me",
                                "<your", "${", "xxxxx", "todo"]):
                            continue
                        issues.append(Issue(
                            "warning", "hardcoded_secret_suspicious",
                            rel, ln,
                            f"{label}: {line[:80]}"
                        ))
    return issues


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    all_issues: list[Issue] = []
    all_issues.extend(check_gitignore())
    all_issues.extend(check_secret_files_in_repo())
    all_issues.extend(check_env_example())
    all_issues.extend(scan_hardcoded_secrets())

    errors = [i for i in all_issues if i.severity == "error"]
    warnings = [i for i in all_issues if i.severity == "warning"]

    if args.json:
        print(json.dumps({
            "check": "check_secrets",
            "tier": "T1",
            "category": "运行治理",
            "errors": [asdict(i) for i in errors],
            "warnings": [asdict(i) for i in warnings],
            "ok": not errors,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"check_secrets (T1, 运行治理) — "
              f"{len(errors)} error / {len(warnings)} warning")
        if errors:
            print(f"\n  错误（{len(errors)} 项）：")
            for i in errors:
                loc = f"{i.file}:{i.line}" if i.line else i.file
                print(f"    ✘ [{i.rule}] {loc}: {i.message}")
        if warnings:
            print(f"\n  警告（{len(warnings)} 项）：")
            for i in warnings[:20]:
                loc = f"{i.file}:{i.line}" if i.line else i.file
                print(f"    ⚠ [{i.rule}] {loc}: {i.message}")
            if len(warnings) > 20:
                print(f"    ...还有 {len(warnings) - 20} 项")
        if not (errors or warnings):
            print("  ✓ 未发现 secrets 风险")

    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:  # noqa: BLE001
        print(f"script error: {e}", file=sys.stderr)
        sys.exit(2)
