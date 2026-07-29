import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const pageSource = read("src/pages/print-template/H9PrintTemplatePage.tsx");
const querySource = read("src/features/print-template/print-template-queries.ts");
const designerSource = read("src/pages/print-template/H9TemplateDesignerDialog.tsx");
const hiprintSource = read("src/pages/print-template/H9HiprintDesigner.tsx");
const previewSource = read("src/pages/print-template/H9TemplatePreviewDialog.tsx");
const businessPrintSource = read("src/pages/print-template/H9BusinessPrintDialog.tsx");
const businessPrintConsumers = [
  read("src/pages/inbound/M2InboundPrintDialog.tsx"),
  read("src/pages/outbound/M4OutboundPage.tsx"),
  read("src/pages/outbound/m4-outbound-print.ts"),
  read("src/pages/master-data/M1MasterDataPage.tsx"),
  read("src/pages/inventory/M3BatchManagementPage.tsx"),
].join("\n");
const fieldLibrarySource = read("src/pages/print-template/H9FieldLibraryDialog.tsx");
const dictionaryPageSource = read("src/pages/master-data/SystemDictionaryPage.tsx");
const dictionaryPrintTypeFieldsSource = read(
  "src/pages/master-data/PrintTemplateTypeFields.tsx",
);
const dictionaryPaneSource = read("../../packages/ui/src/business/SystemDictionaryTwoPane/SystemDictionaryTwoPane.tsx");
const dictionaryLogicSource = read(
  "../../packages/ui/src/business/SystemDictionaryTwoPane/system-dictionary-two-pane-logic.ts",
);
const devMockCoreSource = read("dev-mocks/web-admin-dev-mock-core.ts");
const devMockSource = [
  devMockCoreSource,
  read("dev-mocks/web-admin-dev-mock-print-inventory.ts"),
].join("\n");
const businessIndexSource = read("../../packages/ui/src/business/index.ts");
const componentRegistrySource = read("../../docs/prototypes/component-registry.md");

assert.match(pageSource, /TreeCatalog/);
assert.match(pageSource, /buildH9TreeNodes/);
assert.match(pageSource, /filterRowsByTree/);
assert.match(pageSource, /storageKey="h9\.print-template\.tree"/);
assert.match(pageSource, /searchable=\{false\}/);
assert.doesNotMatch(pageSource, /searchPlaceholder="搜索模板类型、字段库"/);
assert.match(pageSource, /createAction=\{canWriteTemplate \? createAction : undefined\}/);
assert.match(pageSource, /editAction=\{canWriteTemplate \? editAction : undefined\}/);
assert.match(pageSource, /disableAction=\{canWriteTemplate \? disableAction : undefined\}/);
assert.match(pageSource, /toolbarActions=\{toolbarActions\}/);
assert.match(pageSource, /copy-template/);
assert.match(pageSource, /publish-template/);
assert.match(pageSource, /version-history/);
assert.match(pageSource, /版本历史/);
assert.match(pageSource, /H9TemplateDesignerDialog/);
assert.match(pageSource, /H9TemplatePreviewDialog/);
assert.match(pageSource, /H9FieldLibraryDialog/);
assert.match(pageSource, /h9\.print_template\.write/);
assert.match(pageSource, /h9\.print_template\.publish/);
assert.match(querySource, /usePrintTemplateTypesQuery/);
assert.match(querySource, /usePrintTemplatesQuery/);
assert.match(querySource, /usePrintFieldDefinitionsQuery/);
assert.match(querySource, /useSavePrintTemplateMutation/);
assert.match(querySource, /useResolvePrintTemplateMutation/);
assert.match(querySource, /usePrintTemplateVersionsMutation/);
assert.match(querySource, /usePublishPrintTemplateMutation/);
assert.match(querySource, /useSetPrintTemplateEnabledMutation/);
assert.match(querySource, /usePreviewPrintTemplateMutation/);
assert.match(querySource, /useRecordPrintTemplateMutation/);
assert.match(querySource, /useGeneratePrintFieldLibraryDraftMutation/);
assert.match(querySource, /useUpdatePrintFieldDefinitionMutation/);
assert.match(querySource, /usePublishPrintFieldLibraryMutation/);
assert.match(querySource, /print_template_type/);
assert.match(querySource, /\/api\/v1\/print-templates\/field-libraries/);
assert.match(querySource, /\/api\/v1\/print-templates\/field-libraries\/\{version_id\}\/fields/);
assert.match(querySource, /\/api\/v1\/print-templates\/field-libraries\/drafts/);
assert.match(querySource, /\/api\/v1\/print-templates\/field-libraries\/\{version_id\}\/fields\/\{field_id\}/);
assert.match(querySource, /\/api\/v1\/print-templates\/field-libraries\/\{version_id\}\/publish/);
assert.match(querySource, /\/api\/v1\/print-templates\/templates/);
assert.match(querySource, /\/api\/v1\/print-templates\/templates\/\{template_id\}\/versions/);
assert.match(querySource, /\/api\/v1\/print-templates\/templates\/\{template_id\}\/versions\/\{version_id\}\/publish/);
assert.match(querySource, /\/api\/v1\/print-templates\/templates\/\{template_id\}\/enabled/);
assert.match(querySource, /\/api\/v1\/print-templates\/resolve/);
assert.match(querySource, /\/api\/v1\/print-templates\/preview/);
assert.match(querySource, /\/api\/v1\/print-templates\/print/);
assert.match(designerSource, /H9HiprintDesigner/);
assert.match(designerSource, /纸张大小/);
assert.match(designerSource, /纸张方向/);
assert.match(designerSource, /applyPaperToTemplateJson/);
assert.doesNotMatch(designerSource, /模板与纸张设置/);
assert.doesNotMatch(designerSource, /templateSettingsOpen/);
assert.match(designerSource, /jsonOpen/);
assert.match(designerSource, /const templateSettingsPanel = \(/);
assert.match(designerSource, /templateSettingsPanel=\{templateSettingsPanel\}/);
assert.match(designerSource, /fieldBindingPanel/);
assert.doesNotMatch(designerSource, /grid gap-3 lg:grid-cols-4/);
assert.doesNotMatch(designerSource, /<aside className="rounded-md border p-3">/);
assert.match(designerSource, /field_bindings/);
assert.match(designerSource, /template_id: mode === "edit"/);
assert.match(designerSource, /designer_version: "hiprint@0\.4\.0"/);
assert.match(designerSource, /保存新草稿/);
assert.doesNotMatch(designerSource, /保存后发布/);
assert.match(hiprintSource, /import\("hiprint"\)/);
assert.match(hiprintSource, /字段面板/);
assert.match(hiprintSource, /fieldPanelOpen/);
assert.match(hiprintSource, /fieldPanelTab/);
assert.match(hiprintSource, /designerFullscreen/);
assert.match(hiprintSource, /templateSettingsPanel: React\.ReactNode/);
assert.match(hiprintSource, /\{templateSettingsPanel\}/);
assert.match(hiprintSource, /Maximize2/);
assert.match(hiprintSource, /退出/);
assert.match(hiprintSource, /data-h9-hiprint-designer/);
assert.match(hiprintSource, /requestFullscreen/);
assert.match(hiprintSource, /exitFullscreen/);
assert.match(hiprintSource, /fullscreenchange/);
assert.match(hiprintSource, /fixed inset-0 z-\[70\]/);
assert.doesNotMatch(hiprintSource, /fixed inset-3 z-\[70\]/);
assert.doesNotMatch(hiprintSource, /style=\{\{/);
assert.match(hiprintSource, /PrintElementTypeManager\.buildByHtml/);
assert.match(hiprintSource, /PrintTemplate/);
assert.match(previewSource, /template\.getHtml/);
assert.match(previewSource, /纸张方向/);
assert.match(previewSource, /applyPreviewPaperDirection/);
assert.match(previewSource, /templateRef\.current\?\.print/);
assert.match(businessPrintSource, /usePreviewPrintTemplateMutation/);
assert.match(businessPrintSource, /useRecordPrintTemplateMutation/);
for (const errorCode of [
  "H9_TEMPLATE_NOT_FOUND",
  "H9_TEMPLATE_DISABLED",
  "H9_FIELD_LIBRARY_NOT_PUBLISHED",
  "H9_TEMPLATE_FIELD_MISMATCH",
]) {
  assert.match(businessPrintSource, new RegExp(errorCode), `H9 业务打印必须处理 ${errorCode}`);
}
for (const templateType of [
  "asn",
  "acceptance_record",
  "delivery_note",
  "location_label",
  "lpn_label",
  "product_label",
]) {
  assert.match(businessPrintConsumers, new RegExp(`"${templateType}"`), `业务页面必须接入 ${templateType}`);
}
for (const label of [
  "字段库编码",
  "字段库名称",
  "业务模块",
  "来源 Schema",
  "显示名称",
  "分组编码",
  "分组名称",
  "说明",
  "示例值",
  "脱敏规则",
  "格式化规则",
  "支持条码",
  "支持二维码",
  "表格明细字段",
]) {
  assert.match(fieldLibrarySource, new RegExp(label));
}
assert.match(fieldLibrarySource, /latestVersionId/);
assert.match(designerSource, /publishedVersionId/);
assert.match(dictionaryPageSource, /activeGroup\.code === "print_template_type"/);
assert.match(dictionaryPageSource, /m1\.system_dictionary\.write/);
assert.match(dictionaryPageSource, /m1\.system_dictionary\.global\.write/);
for (const label of ["字段库编码", "业务模块", "业务方向", "纸张类型", "默认作用域", "排序号"]) {
  assert.match(`${dictionaryPageSource}\n${dictionaryPrintTypeFieldsSource}`, new RegExp(label));
}
assert.match(dictionaryPageSource, /sort_order/);
assert.match(dictionaryPaneSource, /排序号/);
assert.match(dictionaryLogicSource, /field_library_code: "字段库编码"/);
assert.match(dictionaryLogicSource, /business_module: "业务模块"/);
assert.match(dictionaryLogicSource, /business_direction: "业务方向"/);
assert.match(dictionaryLogicSource, /paper_type: "纸张类型"/);
assert.match(dictionaryLogicSource, /default_scope: "默认作用域"/);
assert.match(devMockSource, /\/api\/v1\/print-templates\/field-libraries/);
assert.match(devMockSource, /fieldDefinitions/);
assert.match(devMockSource, /\/api\/v1\/print-templates\/templates/);
assert.match(devMockSource, /templateVersions/);
assert.match(devMockSource, /publishTemplateVersion/);
assert.match(devMockSource, /setTemplateEnabled/);
assert.match(devMockSource, /\/api\/v1\/print-templates\/resolve/);
assert.match(devMockSource, /\/api\/v1\/print-templates\/preview/);
assert.match(devMockSource, /\/api\/v1\/print-templates\/print/);
assert.match(devMockSource, /fieldLibraries/);
assert.match(devMockSource, /templates/);
assert.match(devMockSource, /library_code/);
assert.match(devMockCoreSource, /await handlePrintInventoryDevMock\(req, res, pathname\)/);
assert.match(businessIndexSource, /export \{ TreeCatalog \}/);
assert.match(componentRegistrySource, /\*\*TreeCatalog\*\*/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}
