import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
  StatusBadge,
} from "@wms/ui";

import {
  useGeneratePrintFieldLibraryDraftMutation,
  usePrintFieldDefinitionsQuery,
  usePublishPrintFieldLibraryMutation,
  useUpdatePrintFieldDefinitionMutation,
  type PrintFieldDefinitionRow,
  type PrintFieldLibraryRow,
  type UpdatePrintFieldDefinitionRequest,
} from "@/features/print-template/print-template-queries";

interface H9FieldLibraryDialogProps {
  open: boolean;
  libraries: PrintFieldLibraryRow[];
  canMaintain: boolean;
  canPublish: boolean;
  onOpenChange: (open: boolean) => void;
}

const emptyDraft = {
  libraryCode: "",
  libraryName: "",
  businessModule: "",
  sourceSchema: "",
};

export function H9FieldLibraryDialog({
  open,
  libraries,
  canMaintain,
  canPublish,
  onOpenChange,
}: H9FieldLibraryDialogProps) {
  const generateMutation = useGeneratePrintFieldLibraryDraftMutation();
  const updateMutation = useUpdatePrintFieldDefinitionMutation();
  const publishMutation = usePublishPrintFieldLibraryMutation();
  const [draft, setDraft] = React.useState(emptyDraft);
  const [selectedLibraryCode, setSelectedLibraryCode] = React.useState("");
  const [editing, setEditing] = React.useState<PrintFieldDefinitionRow | null>(null);
  const [fieldForm, setFieldForm] = React.useState<FieldForm>(() => emptyFieldForm());
  const [notice, setNotice] = React.useState<string | null>(null);
  const selectedLibrary =
    libraries.find((library) => library.libraryCode === selectedLibraryCode) ?? libraries[0] ?? null;
  const fieldsQuery = usePrintFieldDefinitionsQuery(open ? selectedLibrary?.latestVersionId ?? null : null);
  const busy = generateMutation.isPending || updateMutation.isPending || publishMutation.isPending;

  React.useEffect(() => {
    if (!open) return;
    if (selectedLibraryCode) return;
    setSelectedLibraryCode(libraries[0]?.libraryCode ?? "");
  }, [libraries, open, selectedLibraryCode]);

  async function generateDraft(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice(null);
    try {
      const result = await generateMutation.mutateAsync({
        library_code: required(draft.libraryCode, "字段库编码"),
        library_name: required(draft.libraryName, "字段库名称"),
        business_module: required(draft.businessModule, "业务模块"),
        source_schema: required(draft.sourceSchema, "来源 Schema"),
      });
      setSelectedLibraryCode(result.library_code);
      setDraft(emptyDraft);
      setNotice(`${result.library_code} v${result.version_no} 草稿已生成`);
    } catch (error) {
      setNotice(errorMessage(error, "生成字段库草稿失败"));
    }
  }

  function editField(field: PrintFieldDefinitionRow) {
    setEditing(field);
    setFieldForm({
      displayName: field.displayName,
      groupCode: field.groupCode,
      groupName: field.groupName,
      description: field.description,
      exampleValue: exampleText(field.exampleValue),
      printable: field.printable,
      sensitive: field.sensitive,
      maskingRule: field.maskingRule ?? "",
      formattingRule: field.formattingRule ?? "",
      supportsBarcode: field.supportsBarcode,
      supportsQrcode: field.supportsQrcode,
      isTableDetail: field.isTableDetail,
      sortOrder: String(field.sortOrder),
    });
  }

  async function saveField(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editing || !selectedLibrary) return;
    setNotice(null);
    try {
      await updateMutation.mutateAsync({
        libraryVersionId: selectedLibrary.latestVersionId,
        fieldId: editing.id,
        body: fieldRequest(fieldForm),
      });
      setEditing(null);
      setNotice(`${editing.fieldPath} 元数据已保存`);
    } catch (error) {
      setNotice(errorMessage(error, "保存字段元数据失败"));
    }
  }

  async function publishDraft() {
    if (!selectedLibrary || !window.confirm(`确认发布 ${selectedLibrary.libraryName} v${selectedLibrary.versionNo}？`)) return;
    setNotice(null);
    try {
      const result = await publishMutation.mutateAsync(selectedLibrary.latestVersionId);
      setNotice(`${result.library_code} v${result.version_no} 已发布`);
    } catch (error) {
      setNotice(errorMessage(error, "发布字段库失败"));
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !busy && onOpenChange(next)}>
      <DialogContent className="max-h-[92vh] max-w-6xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>字段库管理</DialogTitle>
          <DialogDescription>从当前 H3 OpenAPI 生成草稿，维护字段元数据后发布；已发布版本不可修改。</DialogDescription>
        </DialogHeader>

        {canMaintain && (
          <form className="grid gap-3 rounded-md border p-4 md:grid-cols-5" onSubmit={generateDraft}>
            <TextField label="字段库编码">
              <Input
                aria-label="字段库编码"
                value={draft.libraryCode}
                onChange={(event) => setDraft({ ...draft, libraryCode: event.target.value })}
                placeholder="m2_receiving_order"
              />
            </TextField>
            <TextField label="字段库名称">
              <Input
                aria-label="字段库名称"
                value={draft.libraryName}
                onChange={(event) => setDraft({ ...draft, libraryName: event.target.value })}
                placeholder="M2 收货单字段库"
              />
            </TextField>
            <TextField label="业务模块">
              <Input
                aria-label="业务模块"
                value={draft.businessModule}
                onChange={(event) => setDraft({ ...draft, businessModule: event.target.value })}
                placeholder="M2"
              />
            </TextField>
            <TextField label="来源 Schema">
              <Input
                aria-label="来源 Schema"
                value={draft.sourceSchema}
                onChange={(event) => setDraft({ ...draft, sourceSchema: event.target.value })}
                placeholder="CreateReceivingOrderRequest"
              />
            </TextField>
            <div className="flex items-end">
              <Button type="submit" className="w-full" disabled={busy}>
                生成草稿
              </Button>
            </div>
          </form>
        )}

        <div className="flex flex-wrap items-end justify-between gap-3">
          <TextField label="字段库版本">
            <select
              aria-label="字段库版本"
              className="h-10 min-w-72 rounded-md border border-input bg-background px-3 text-sm"
              value={selectedLibrary?.libraryCode ?? ""}
              onChange={(event) => {
                setSelectedLibraryCode(event.target.value);
                setEditing(null);
              }}
            >
              {libraries.map((library) => (
                <option key={library.libraryCode} value={library.libraryCode}>
                  {library.libraryName} · v{library.versionNo} · {library.statusLabel}
                </option>
              ))}
            </select>
          </TextField>
          {selectedLibrary && (
            <div className="flex items-center gap-3 text-sm">
              <span>{selectedLibrary.businessModule}</span>
              <span className="font-mono">{selectedLibrary.sourceSchema}</span>
              <StatusBadge
                status={selectedLibrary.status === "published" ? "completed" : "pending"}
                label={selectedLibrary.statusLabel}
                size="sm"
              />
              {selectedLibrary.status === "draft" && canPublish && (
                <Button type="button" onClick={() => void publishDraft()} disabled={busy}>
                  发布字段库
                </Button>
              )}
            </div>
          )}
        </div>

        {notice && <div role="status" className="rounded-md border bg-muted/30 px-3 py-2 text-sm">{notice}</div>}
        {fieldsQuery.error && <div role="alert" className="text-sm text-destructive">{fieldsQuery.error.message}</div>}

        <div className="max-h-80 overflow-auto rounded-md border">
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-muted">
              <tr>
                <th className="px-3 py-2 text-left">显示名称</th>
                <th className="px-3 py-2 text-left">字段路径</th>
                <th className="px-3 py-2 text-left">类型 / 来源</th>
                <th className="px-3 py-2 text-left">字段分组</th>
                <th className="px-3 py-2 text-left">能力</th>
                <th className="px-3 py-2 text-right">操作</th>
              </tr>
            </thead>
            <tbody>
              {(fieldsQuery.data ?? []).map((field) => (
                <tr key={field.id} className="border-t">
                  <td className="px-3 py-2">{field.displayName}</td>
                  <td className="px-3 py-2 font-mono text-xs">{field.fieldPath}</td>
                  <td className="px-3 py-2">{field.fieldType} · {field.sourceSchema}</td>
                  <td className="px-3 py-2">{field.groupName}（{field.groupCode}）</td>
                  <td className="px-3 py-2">{fieldCapabilities(field)}</td>
                  <td className="px-3 py-2 text-right">
                    {selectedLibrary?.status === "draft" && canMaintain && (
                      <Button type="button" size="sm" variant="outline" onClick={() => editField(field)}>
                        编辑
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
              {!fieldsQuery.isPending && (fieldsQuery.data?.length ?? 0) === 0 && (
                <tr><td colSpan={6} className="px-3 py-8 text-center text-muted-foreground">暂无字段</td></tr>
              )}
            </tbody>
          </table>
        </div>

        {editing && (
          <form className="grid gap-3 rounded-md border p-4 md:grid-cols-4" onSubmit={saveField}>
            <div className="md:col-span-4 text-sm font-medium">编辑字段：<span className="font-mono">{editing.fieldPath}</span></div>
            <TextInput label="显示名称" value={fieldForm.displayName} onChange={(value) => setFieldForm({ ...fieldForm, displayName: value })} />
            <TextInput label="分组编码" value={fieldForm.groupCode} onChange={(value) => setFieldForm({ ...fieldForm, groupCode: value })} />
            <TextInput label="分组名称" value={fieldForm.groupName} onChange={(value) => setFieldForm({ ...fieldForm, groupName: value })} />
            <TextInput label="排序号" type="number" value={fieldForm.sortOrder} onChange={(value) => setFieldForm({ ...fieldForm, sortOrder: value })} />
            <TextInput label="说明" value={fieldForm.description} onChange={(value) => setFieldForm({ ...fieldForm, description: value })} />
            <TextInput label="示例值" value={fieldForm.exampleValue} onChange={(value) => setFieldForm({ ...fieldForm, exampleValue: value })} />
            <TextInput label="脱敏规则" value={fieldForm.maskingRule} onChange={(value) => setFieldForm({ ...fieldForm, maskingRule: value })} />
            <TextInput label="格式化规则" value={fieldForm.formattingRule} onChange={(value) => setFieldForm({ ...fieldForm, formattingRule: value })} />
            <div className="md:col-span-4 flex flex-wrap gap-4">
              {([
                ["printable", "可打印"],
                ["sensitive", "敏感字段"],
                ["supportsBarcode", "支持条码"],
                ["supportsQrcode", "支持二维码"],
                ["isTableDetail", "表格明细字段"],
              ] as const).map(([key, label]) => (
                <label key={key} className="flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={fieldForm[key]} onChange={(event) => setFieldForm({ ...fieldForm, [key]: event.target.checked })} />
                  {label}
                </label>
              ))}
            </div>
            <div className="md:col-span-4 flex justify-end gap-2">
              <Button type="button" variant="outline" onClick={() => setEditing(null)} disabled={busy}>取消</Button>
              <Button type="submit" disabled={busy}>保存字段元数据</Button>
            </div>
          </form>
        )}

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface FieldForm {
  displayName: string;
  groupCode: string;
  groupName: string;
  description: string;
  exampleValue: string;
  printable: boolean;
  sensitive: boolean;
  maskingRule: string;
  formattingRule: string;
  supportsBarcode: boolean;
  supportsQrcode: boolean;
  isTableDetail: boolean;
  sortOrder: string;
}

function emptyFieldForm(): FieldForm {
  return {
    displayName: "", groupCode: "", groupName: "", description: "", exampleValue: "",
    printable: true, sensitive: false, maskingRule: "", formattingRule: "",
    supportsBarcode: false, supportsQrcode: false, isTableDetail: false, sortOrder: "0",
  };
}

function fieldRequest(form: FieldForm): UpdatePrintFieldDefinitionRequest {
  return {
    display_name: required(form.displayName, "显示名称"),
    group_code: required(form.groupCode, "分组编码"),
    group_name: required(form.groupName, "分组名称"),
    description: form.description.trim(),
    example_value: form.exampleValue.trim() || null,
    printable: form.printable,
    sensitive: form.sensitive,
    masking_rule: form.maskingRule.trim() || null,
    formatting_rule: form.formattingRule.trim() || null,
    supports_barcode: form.supportsBarcode,
    supports_qrcode: form.supportsQrcode,
    is_table_detail: form.isTableDetail,
    sort_order: nonNegativeInteger(form.sortOrder),
  };
}

function TextInput({ label, value, onChange, type = "text" }: { label: string; value: string; onChange: (value: string) => void; type?: string }) {
  return <TextField label={label}><Input aria-label={label} type={type} value={value} onChange={(event) => onChange(event.target.value)} /></TextField>;
}

function TextField({ label, children }: { label: string; children: React.ReactNode }) {
  return <div className="grid gap-1.5"><Label>{label}</Label>{children}</div>;
}

function fieldCapabilities(field: PrintFieldDefinitionRow) {
  const labels = [
    field.printable && "可打印",
    field.sensitive && "敏感",
    field.supportsBarcode && "条码",
    field.supportsQrcode && "二维码",
    field.isTableDetail && "明细",
  ].filter(Boolean);
  return labels.join(" / ") || "-";
}

function exampleText(value: unknown) {
  if (value == null) return "";
  return typeof value === "string" ? value : JSON.stringify(value);
}

function required(value: string, label: string) {
  const result = value.trim();
  if (!result) throw new Error(`${label}不能为空`);
  return result;
}

function nonNegativeInteger(value: string) {
  const result = Number(value);
  if (!Number.isInteger(result) || result < 0) throw new Error("排序号必须是非负整数");
  return result;
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
