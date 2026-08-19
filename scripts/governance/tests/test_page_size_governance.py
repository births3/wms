"""页面行数治理测试。"""
import sys
from pathlib import Path

import pytest

TEST_DIR = Path(__file__).resolve().parent
SCRIPTS_DIR = TEST_DIR.parent
sys.path.insert(0, str(SCRIPTS_DIR))


@pytest.fixture(autouse=True)
def isolated_page_size_probe_repo(tmp_path, monkeypatch):
    """探针只能写 pytest 临时仓库，避免与并行治理扫描互相污染。"""
    import check_page_size

    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(check_page_size, "REPO_ROOT", tmp_path)
    page_dirs = (
        tmp_path / "prototypes/src/pages",
        tmp_path / "prototypes/src/prototype-kit",
        tmp_path / "apps/web-admin/src/pages",
        tmp_path / "apps/pda-mobile/src/pages",
        tmp_path / "packages/ui/src/business",
    )
    source_dirs = (
        tmp_path / "backend/crates",
        tmp_path / "apps",
        tmp_path / "packages",
        tmp_path / "prototypes",
        tmp_path / "scripts",
    )
    probe_dirs = (
        *page_dirs,
        *source_dirs,
        tmp_path / "backend/crates/api/src",
        tmp_path / "apps/web-admin/self-checks",
        tmp_path / "scripts/governance",
    )
    for directory in probe_dirs:
        directory.mkdir(parents=True, exist_ok=True)
    monkeypatch.setattr(check_page_size, "PAGE_DIRS", page_dirs)
    monkeypatch.setattr(check_page_size, "SOURCE_DIRS", source_dirs)
    monkeypatch.setattr(check_page_size, "BASELINE_PATH", tmp_path / "governance/source-size-baseline.toml")
    monkeypatch.setattr(check_page_size, "LEGACY_OVERSIZED_SOURCE_BASELINE", {})


def test_governance_test_files_stay_below_800_lines():
    """治理测试文件必须保持可 review 的单文件规模。"""
    max_lines = 800
    # ponytail: existing oversized tests are baseline debt; only shrinking or holding is allowed.
    baseline_debt = {
        "test_wave3_pda_readiness_docs.py": 867,
        "test_wave3_pda_runtime_evidence.py": 1262,
        "test_wave3_pda_runtime_readiness.py": 2966,
    }
    oversized = []
    for path in sorted(TEST_DIR.glob("test_*.py")):
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        allowed = baseline_debt.get(path.name, max_lines)
        if line_count > allowed:
            oversized.append(f"{path.name}: {line_count} lines")

    assert not oversized, "治理测试文件达到 800 行，请拆分：\n" + "\n".join(oversized)


def test_check_page_size_scans_production_web_admin_pages():
    """生产 web-admin 页面也必须受页面行数门禁保护。"""
    import check_page_size

    page = Path("apps/web-admin/src/pages/__page_size_probe.tsx")
    page.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(
        f"export const probe{i} = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD)
    ) + "\n"

    try:
        page.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        page.unlink(missing_ok=True)

    assert any(page.as_posix() in error for error in errors)


def test_check_page_size_scans_shared_business_components():
    """共享业务组件也必须受行数门禁保护。"""
    import check_page_size

    component = Path("packages/ui/src/business/__page_size_probe.tsx")
    component.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(
        f"export const probe{i} = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD)
    ) + "\n"

    try:
        component.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        component.unlink(missing_ok=True)

    assert any(component.as_posix() in error for error in errors)


def test_check_page_size_scans_backend_source_files():
    """后端源码不能绕过通用行数门禁。"""
    import check_page_size

    source = Path("backend/crates/api/src/__page_size_probe.rs")
    source.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(
        f"pub const PROBE_{i}: usize = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD)
    ) + "\n"

    try:
        source.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error for error in errors)


def test_legacy_oversized_source_growth_fails():
    """历史超限基线只允许持平或减少，不能继续增长。"""
    import check_page_size

    source = Path("backend/crates/api/src/__legacy_page_size_probe.rs")
    source.parent.mkdir(parents=True, exist_ok=True)
    rel = source.as_posix()
    text = "\n".join(
        f"pub const LEGACY_PROBE_{i}: usize = {i};"
        for i in range(check_page_size.ERROR_THRESHOLD + 1)
    ) + "\n"

    try:
        check_page_size.LEGACY_OVERSIZED_SOURCE_BASELINE[rel] = check_page_size.ERROR_THRESHOLD
        source.write_text(text, encoding="utf-8")
        errors, _warnings = check_page_size.run()
    finally:
        check_page_size.LEGACY_OVERSIZED_SOURCE_BASELINE.pop(rel, None)
        source.unlink(missing_ok=True)

    assert any(rel in error and "历史基线" in error for error in errors)


def test_generated_payload_loader_fails_even_when_line_count_is_small():
    """禁止用 base64 载荷把大脚本伪装成短文件。"""
    import check_page_size

    source = Path("scripts/governance/__payload_probe.py")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "\n".join(
            [
                '"""Generated implementation ' + 'loader."""',
                "import base64",
                "_IMPL" + '_SOURCE_B64 = "abcd"',
                "exec(base64." + "b64decode(_IMPL" + "_SOURCE_B64))",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_urlsafe_base64_payload_loader_fails():
    """常见的 urlsafe base64 解码变体也不能绕过门禁。"""
    import check_page_size

    source = Path("scripts/governance/__urlsafe_payload_probe.py")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "import base64\n"
        "payload = 'abcd'\n"
        + "exec("
        + "base64."
        + "urlsafe_b64decode(payload))\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_imported_base64_decoder_with_spaced_exec_fails():
    """Python AST 必须识别直接导入解码器和带空格的动态执行。"""
    import check_page_size

    source = Path("scripts/governance/__imported_payload_probe.py")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "from base64 import b64decode\n"
        "payload = 'abcd'\n"
        "exec (b64decode(payload))\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_standard_base64_decoder_fails():
    """base64.standard_b64decode 不能绕过 Python payload 门禁。"""
    import check_page_size

    source = Path("scripts/governance/__standard_payload_probe.py")
    source.write_text(
        "import base64\n"
        "payload = 'abcd'\n"
        "exec(base64.standard_b64decode(payload))\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_codecs_base64_decoder_fails():
    """codecs.decode(..., base64) 不能绕过 Python payload 门禁。"""
    import check_page_size

    source = Path("scripts/governance/__codecs_payload_probe.py")
    source.write_text(
        "import codecs\n"
        "payload = b'abcd'\n"
        "exec(codecs.decode(payload, 'base64'))\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_codecs_keyword_base64_decoder_fails():
    """codecs.decode 的 encoding 关键字同样不能绕过门禁。"""
    import check_page_size

    source = Path("scripts/governance/__codecs_keyword_payload_probe.py")
    source.write_text(
        "import codecs\n"
        "payload = b'abcd'\n"
        "exec(codecs.decode(payload, encoding='base64'))\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_unrelated_decoder_and_exec_do_not_fail():
    """解码调用与固定 exec 无数据流关系时不能误报载荷。"""
    import check_page_size

    source = Path("scripts/governance/__unrelated_payload_probe.py")
    source.write_text(
        "import base64\n"
        "decoded = base64.b64decode(data)\n"
        "exec('trusted_expression')\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert not any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_unrelated_javascript_decoder_and_exec_do_not_fail():
    """JavaScript 解码结果未进入动态执行时不能误报。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__unrelated_payload_probe.mjs")
    source.write_text(
        "const decoded = Buffer.from('abcd', 'base64');\n"
        "eval('trusted_expression');\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert not any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_python_dynamic_getattr_decoder_payload_loader_fails():
    """getattr 取得 base64 解码器后动态执行不能绕过门禁。"""
    import check_page_size

    source = Path("scripts/governance/__dynamic_getattr_payload_probe.py")
    source.write_text(
        "import base64\n"
        "exec(getattr(base64, 'b64decode')(payload))\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_python_dynamic_import_attribute_payload_loader_fails():
    """__import__ 动态模块属性解码不能绕过门禁。"""
    import check_page_size

    assert check_page_size._python_loads_decoded_payload(
        "exec(__import__('base64').b64decode(payload))\n"
    )


def test_javascript_computed_global_eval_payload_loader_fails():
    """globalThis 计算属性 eval 不能绕过门禁。"""
    import check_page_size

    assert check_page_size._javascript_loads_decoded_payload(
        "const decoded = Buffer.from(payload, 'base64');\n"
        "globalThis['eval'](decoded);\n"
    )


def test_page_size_skip_tag_must_be_a_header_comment_with_reason(tmp_path):
    import check_page_size

    string_marker = tmp_path / "string-marker.ts"
    string_marker.write_text(
        'const marker = "@governance: skip-page-size because";\n'
        + "\n".join(f"export const value{i} = {i};" for i in range(check_page_size.ERROR_THRESHOLD)),
        encoding="utf-8",
    )
    missing_reason = tmp_path / "missing-reason.ts"
    missing_reason.write_text(
        "// @governance: skip-page-size\n"
        + "\n".join(f"export const value{i} = {i};" for i in range(check_page_size.ERROR_THRESHOLD)),
        encoding="utf-8",
    )
    valid_reason = tmp_path / "valid-reason.ts"
    valid_reason.write_text(
        "// @governance: skip-page-size 三栏布局暂不拆分\n"
        + "\n".join(f"export const value{i} = {i};" for i in range(check_page_size.ERROR_THRESHOLD)),
        encoding="utf-8",
    )

    assert check_page_size._check_file(string_marker)[2]
    assert check_page_size._check_file(missing_reason)[2]
    assert not check_page_size._check_file(valid_reason)[2]


def test_page_size_skip_tag_after_code_is_rejected(tmp_path):
    import check_page_size

    source = tmp_path / "late-skip.ts"
    source.write_text(
        "export const before = true;\n"
        "// @governance: skip-page-size 代码后标签无效\n"
        + "\n".join(f"export const value{i} = {i};" for i in range(check_page_size.ERROR_THRESHOLD)),
        encoding="utf-8",
    )

    assert check_page_size._check_file(source)[2]


def test_python_decoder_function_alias_and_decodebytes_are_detected():
    import check_page_size

    assert check_page_size._python_loads_decoded_payload(
        "import base64\ndecoder = base64.b64decode\ndecoded = decoder(payload)\nexec(decoded)\n"
    )
    assert check_page_size._python_loads_decoded_payload(
        "import base64\ndecoded = base64.decodebytes(payload)\nexec(decoded)\n"
    )


def test_javascript_plain_assignment_payload_is_detected():
    import check_page_size

    assert check_page_size._javascript_loads_decoded_payload(
        "let decoded;\ndecoded = Buffer.from(payload, 'base64');\neval(decoded);\n"
    )


def test_javascript_decoder_alias_and_dynamic_encoding_are_detected():
    import check_page_size

    assert check_page_size._javascript_loads_decoded_payload(
        'const decode = Buffer.from;\nconst decoded = decode(payload, "base64");\neval(decoded);\n'
    )
    assert check_page_size._javascript_loads_decoded_payload(
        'const encoding = "base64";\nconst decoded = Buffer.from(payload, encoding);\neval(decoded);\n'
    )
    assert check_page_size._javascript_loads_decoded_payload(
        "const decoded = Buffer.from(payload, `base64`);\neval(decoded);\n"
    )


def test_javascript_comments_and_strings_do_not_trigger_payload_detection():
    import check_page_size

    assert not check_page_size._javascript_loads_decoded_payload(
        "// const decoded = Buffer.from(payload, 'base64'); eval(decoded);\n"
    )
    assert not check_page_size._javascript_loads_decoded_payload(
        'const example = "const decoded = Buffer.from(payload, \'base64\'); eval(decoded);";\n'
    )


def test_legacy_source_baseline_cannot_grow_or_add_entries():
    import check_page_size

    previous = {"legacy.rs": 900}
    assert check_page_size._baseline_policy_errors({"legacy.rs": 901}, previous)
    assert check_page_size._baseline_policy_errors(
        {"legacy.rs": 900, "new-legacy.rs": 1000},
        previous,
    )
    assert not check_page_size._baseline_policy_errors({"legacy.rs": 850}, previous)


def test_javascript_buffer_payload_loader_fails():
    """JavaScript Buffer base64 解码后动态执行必须失败。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__buffer_payload_probe.mjs")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "const payload = 'abcd';\n"
        "const decoded = Buffer.from(payload, 'base64');\n"
        "eval(decoded);\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_javascript_vm_payload_loader_fails():
    """Node vm 动态执行解码载荷必须失败。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__vm_payload_probe.mjs")
    source.write_text(
        "import vm from 'node:vm';\n"
        "const decoded = Buffer.from('abcd', 'base64');\n"
        "vm.runInThisContext(decoded);\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_javascript_base64url_new_context_payload_loader_fails():
    """base64url 配合 vm.runInNewContext 不能绕过门禁。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__vm_new_context_payload_probe.mjs")
    source.write_text(
        "import vm from 'node:vm';\n"
        "const decoded = Buffer.from('abcd', 'base64url');\n"
        "vm.runInNewContext(decoded);\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_javascript_computed_buffer_from_payload_loader_fails():
    """Buffer['from'] 计算属性写法不能绕过门禁。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__computed_buffer_payload_probe.mjs")
    source.write_text(
        "const decoded = Buffer['from']('abcd', 'base64');\n"
        "eval(decoded);\n",
        encoding="utf-8",
    )
    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "生成实现载荷" in error for error in errors)


def test_check_page_size_scans_mjs_source():
    """应用自检 MJS 文件也必须受行数门禁保护。"""
    import check_page_size

    source = Path("apps/web-admin/self-checks/__page_size_probe.mjs")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "\n".join(
            f"export const probe{i} = {i};"
            for i in range(check_page_size.ERROR_THRESHOLD)
        )
        + "\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error for error in errors)


def test_oversized_single_line_payload_fails():
    """行数不多但单行巨大也必须失败。"""
    import check_page_size

    source = Path("scripts/governance/__single_line_payload_probe.py")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "PAYLOAD = '" + ("x" * (check_page_size.MAX_PAYLOAD_LINE_LENGTH + 1)) + "'\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error and "单行" in error for error in errors)


def test_oversized_single_line_payload_fails_in_page_file():
    """页面文件不能利用超长单行绕过 payload 门禁。"""
    import check_page_size

    page = Path("apps/web-admin/src/pages/__single_line_payload_probe.tsx")
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        "export const payload = '"
        + ("x" * (check_page_size.MAX_PAYLOAD_LINE_LENGTH + 1))
        + "';\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        page.unlink(missing_ok=True)

    assert any(page.as_posix() in error and "单行" in error for error in errors)


def test_check_page_size_scans_prototype_typescript_source():
    """原型支撑目录中的普通 TypeScript 源码也必须受门禁保护。"""
    import check_page_size

    source = Path("prototypes/src/prototype-kit/__page_size_probe.ts")
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text(
        "\n".join(
            f"export const probe{i} = {i};"
            for i in range(check_page_size.ERROR_THRESHOLD)
        )
        + "\n",
        encoding="utf-8",
    )

    try:
        errors, _warnings = check_page_size.run()
    finally:
        source.unlink(missing_ok=True)

    assert any(source.as_posix() in error for error in errors)


def test_production_page_changes_trigger_page_size_gate():
    """生产前端页面变更必须触发页面行数门禁。"""
    from _diff import load_gate_rules, match_rules

    rules = load_gate_rules()
    for changed_file in [
        "apps/web-admin/src/pages/auth/LoginPage.tsx",
        "apps/pda-mobile/src/pages/ScanPage.tsx",
    ]:
        triggered = match_rules([changed_file], rules)
        assert "check_page_size" in triggered


def test_shared_business_component_changes_trigger_page_size_gate():
    """共享业务组件变更必须触发行数门禁。"""
    from _diff import load_gate_rules, match_rules

    rules = load_gate_rules()
    triggered = match_rules(["packages/ui/src/business/DataGrid/DataGrid.tsx"], rules)

    assert "check_page_size" in triggered


def test_all_production_source_changes_trigger_page_size_gate():
    """普通生产源码也必须在 diff 调度阶段触发行数门禁。"""
    from _diff import load_gate_rules, match_rules

    rules = load_gate_rules()
    for changed_file in [
        "backend/crates/api/src/state_machine.rs",
        "apps/web-admin/src/features/example.ts",
        "packages/ui/src/lib/example.ts",
        "prototypes/src/prototype-kit/example.ts",
        "prototypes/e2e/example.spec.ts",
        "scripts/agents/example.py",
    ]:
        assert "check_page_size" in match_rules([changed_file], rules), changed_file
