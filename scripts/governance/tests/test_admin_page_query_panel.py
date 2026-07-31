"""管理端页面级查询条件治理脚本测试。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_page_query_suggestion_for_m1_master_data_page():
    """M1 基础档案页默认只把关键词放入核心查询。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("m1-zones", "M1 库区管理")

    assert suggestion["required"] is True
    assert suggestion["source"] == "apps/web-admin/src/pages/master-data/M1MasterDataPage.tsx"
    assert suggestion["core"] == ["keyword"]
    assert suggestion["more"] == []


def test_page_query_suggestion_for_m2_business_page():
    """M2 入库业务页默认把关键词、货主、状态放入核心查询。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("m2-receiving", "M2 收货管理")

    assert suggestion["required"] is True
    assert suggestion["source"] == "apps/web-admin/src/pages/inbound/M2InboundPage.tsx"
    assert suggestion["core"] == ["keyword", "ownerKeyword", "statusFilter"]
    assert suggestion["more"] == ["documentTypeFilter", "arrivalDate", "createdAt"]


def test_page_query_suggestion_for_m3_batch_page():
    """M3 批号列表必须接入公共页面级查询。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("m3-batches", "M3 批号管理")

    assert suggestion["required"] is True
    assert suggestion["source"] == "apps/web-admin/src/pages/inventory/M3BatchManagementPage.tsx"
    assert suggestion["core"] == ["keyword", "qualityStatus"]
    assert suggestion["more"] == ["recallFlag", "productionDate", "expiryDate", "createdAt"]


def test_page_query_suggestion_for_m4_outbound_page():
    """M4 出库列表必须接入公共页面级查询。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("m4-orders", "M4 出库订单管理")

    assert suggestion["required"] is True
    assert suggestion["source"] == "apps/web-admin/src/pages/outbound/M4OutboundPage.tsx"
    assert suggestion["core"] == ["keyword", "statusFilter"]
    assert suggestion["more"] == []


def test_page_query_suggestion_for_h9_print_template_page():
    """H9 打印模板字段库列表必须接入公共页面级查询。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("h9-print-templates", "H9 打印模板")

    assert suggestion["required"] is True
    assert suggestion["source"] == "apps/web-admin/src/pages/print-template/H9PrintTemplatePage.tsx"
    assert suggestion["core"] == ["keyword", "templateType"]
    assert suggestion["more"] == []


def test_page_query_suggestion_requires_confirmation_for_unknown_page():
    """未知页面族不能自动造查询字段，必须进入待确认状态。"""
    from check_admin_page_query_panel import suggest_page_config

    suggestion = suggest_page_config("h1-devices", "H1 设备管理")

    assert suggestion["required"] is False
    assert "待确认" in suggestion["reason"]


def test_page_query_supports_dynamic_field_builder():
    """动态字典字段也必须能被治理脚本解析。"""
    from check_admin_page_query_panel import field_keys

    source = """
    function buildFields(options) {
        return [{ key: "documentType", label: "单据类型", options }];
    }
    const queryFields = React.useMemo(() => buildFields(options), [options]);
    """

    assert field_keys(source, "queryFields") == {"documentType"}
