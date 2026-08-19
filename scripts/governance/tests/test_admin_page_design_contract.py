from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import check_admin_page_design_contract as check


def test_design_contract_rejects_resident_tracking_panel(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/express"
    pages.mkdir(parents=True)
    page = pages / "H5ExpressPage.tsx"
    page.write_text(
        """
        const columns: DataGridColumn<Row>[] = [];
        export function H5ExpressPage() {
          return <><DataGrid columns={columns} data={[]} /><WaybillPanel /></>;
        }
        function WaybillPanel() {
          return <section>运单与轨迹</section>;
        }
        const actions = [{ key: "tracking", label: "轨迹" }];
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    issues = check.scan()

    assert any(issue.kind == "resident_detail_panel" for issue in issues)
    assert any(issue.kind == "action_without_dialog" for issue in issues)


def test_design_contract_rejects_direct_write_actions(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/admin-menu"
    pages.mkdir(parents=True)
    page = pages / "H1AdminMenuPage.tsx"
    page.write_text(
        """
        export function H1AdminMenuPage() {
          return <Button onClick={() => void publishMutation.mutateAsync({ note: "发布" })}>发布</Button>;
        }
        function createChild() {
          const title = window.prompt("菜单名称", "新页面");
          return title;
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    issues = check.scan()

    assert any(issue.kind == "direct_write_click" for issue in issues)
    assert any(issue.kind == "browser_action_without_dialog" for issue in issues)


def test_design_contract_rejects_named_click_handler_write_actions(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/admin-menu"
    pages.mkdir(parents=True)
    page = pages / "H1AdminMenuPage.tsx"
    page.write_text(
        """
        export function H1AdminMenuPage() {
          return <Button onClick={handlePublish}>发布</Button>;
        }
        async function handlePublish() {
          await publishMutation.mutateAsync({ note: "发布" });
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    issues = check.scan()

    assert any(issue.kind == "direct_write_click" for issue in issues)


def test_design_contract_rejects_disable_action_without_confirm(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/print-template"
    pages.mkdir(parents=True)
    page = pages / "H9PrintTemplatePage.tsx"
    page.write_text(
        """
        import type { DataGridDisableAction } from "@wms/ui";
        const disableAction: DataGridDisableAction = {
          label: "停用",
          onClick: () => void toggleTemplateEnabled(),
        };
        export function H9PrintTemplatePage() {
          return <DataGrid disableAction={disableAction} data={[]} columns={[]} />;
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    issues = check.scan()

    assert any(issue.kind == "action_without_dialog" for issue in issues)


def test_design_contract_allows_write_action_that_opens_dialog(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/print-template"
    pages.mkdir(parents=True)
    page = pages / "H9PrintTemplatePage.tsx"
    page.write_text(
        """
        export function H9PrintTemplatePage() {
          return <Button onClick={openDesigner}>修改</Button>;
        }
        async function openDesigner() {
          await versionsMutation.mutateAsync("tpl-1");
          setDesignerOpen(true);
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    assert check.scan() == []


def test_design_contract_allows_write_action_that_opens_shared_dialog_state(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/print-template"
    pages.mkdir(parents=True)
    page = pages / "H9PrintTemplatePage.tsx"
    page.write_text(
        """
        export function H9PrintTemplatePage() {
          return <Button onClick={openDesigner}>修改</Button>;
        }
        async function openDesigner() {
          const version = await versionsMutation.mutateAsync("tpl-1");
          designerDialog.openWith(version);
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    assert check.scan() == []


def test_design_contract_supports_explicit_governance_skip(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/config-center"
    pages.mkdir(parents=True)
    page = pages / "FeatureFlagConfigCenterPage.tsx"
    page.write_text(
        """
        // @governance: skip-admin-page-design-contract 配置型双栏页面，动作由专项自检覆盖
        export function FeatureFlagConfigCenterPage() {
          return <Button onClick={() => void migrateMutation.mutateAsync()}>迁移</Button>;
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    assert check.scan() == []


def test_design_contract_accepts_plain_datagrid_page(tmp_path, monkeypatch):
    pages = tmp_path / "apps/web-admin/src/pages/inventory"
    pages.mkdir(parents=True)
    page = pages / "M3BatchManagementPage.tsx"
    page.write_text(
        """
        const columns: DataGridColumn<Row>[] = [];
        export function M3BatchManagementPage() {
          return <DataGrid columns={columns} data={[]} />;
        }
        """,
        encoding="utf-8",
    )
    monkeypatch.setattr(check, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(check, "PAGES_DIR", tmp_path / "apps/web-admin/src/pages")

    assert check.scan() == []
