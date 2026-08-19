"""字段编码规范与业务规则注册表治理测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
def test_field_coding_standards_accepts_current_type_shapes():
    """字段编码规范脚本应接受现有词典使用的 PostgreSQL 类型形态。"""
    from check_field_coding_standards import is_valid_data_type

    for data_type in ["VARCHAR(32)", "NUMERIC(15,3)", "TIMESTAMPTZ", "JSONB", "TEXT[]", "INT"]:
        assert is_valid_data_type(data_type)
    for data_type in ["FLOAT", "DOUBLE", "TIMESTAMP", "CHAR(8)", "ARRAY[]"]:
        assert not is_valid_data_type(data_type)


def test_field_coding_standards_rejects_int_for_id_fields():
    """INT 仅允许有界计数/配置阈值，ID 类字段必须使用 BIGINT。"""
    from check_gsp_field_traceability import FieldEntry
    from check_field_coding_standards import validate_entries

    issues = validate_entries([
        FieldEntry(
            canonical="customer_id",
            aliases=["客户 ID"],
            data_type="INT",
            validation=">0",
            nullable=False,
            encryption="none",
            audit_required=True,
            field_class="business",
        )
    ], [])

    assert any(issue.rule == "int_id_type" for issue in issues)


def test_business_rules_registry_parser_requires_detail_fields():
    """业务规则注册表详情段必须含字段表。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 批号 | 拣货 | 7.94 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 规则描述",
        "按批号排序。",
    ])

    rules, issues = parse_business_rules(text)

    assert [rule.rule_id for rule in rules] == ["BR-1"]
    assert any(issue.rule == "missing_fields_section" for issue in issues)
    assert any(issue.rule == "missing_detail_fields" for issue in issues)


def test_business_rules_registry_state_enum_exception_only_applies_to_br8():
    """只有 BR-8 状态机可用“状态枚举”替代“规则描述”。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 批号 | 拣货 | 7.94 |",
        "| BR-8 | 库存状态机 | 状态 | 状态变更 | 7.95 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 状态枚举",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `batch_no` | VARCHAR(20) | 批号 |",
        "",
        "## 10. BR-8: 库存状态机",
        "",
        "### 10.1 状态枚举",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `status` | VARCHAR(20) | 当前状态 |",
        "",
        "### 10.3 涉及字段",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `previous_status` | VARCHAR(20) | 上一状态 |",
    ])

    _, issues = parse_business_rules(text)

    assert any(issue.rule_id == "BR-1" and issue.rule == "missing_description" for issue in issues)
    assert not any(issue.rule_id == "BR-8" and issue.rule == "missing_description" for issue in issues)


def test_business_rules_registry_parses_text_array_field_type():
    """业务规则字段表类型解析应与字段编码脚本一致支持 TEXT[]。"""
    from check_business_rules_registry import parse_business_rules

    text = "\n".join([
        "| ID | 规则名 | 涉及字段 | 触发场景 | GSP 关联 |",
        "|----|----|----|----|----|",
        "| BR-1 | FIFO | 经营范围 | 拣货 | 7.94 |",
        "",
        "## 3. BR-1: FIFO",
        "",
        "### 3.1 规则描述",
        "按经营范围过滤。",
        "",
        "### 3.2 涉及字段",
        "| 字段 | data_type | 用途 |",
        "|----|----|----|",
        "| `business_scopes` | TEXT[] | 经营范围 |",
    ])

    rules, issues = parse_business_rules(text)

    assert rules[0].detail_fields == ["business_scopes"]
    assert not any(issue.rule == "missing_detail_fields" for issue in issues)
