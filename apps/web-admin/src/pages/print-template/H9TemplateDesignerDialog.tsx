import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
} from "@wms/ui";
import { Save } from "lucide-react";

import {
  usePrintFieldDefinitionsQuery,
  type PrintFieldLibraryRow,
  type PrintTemplateVersion,
  type PrintTemplateTypeRow,
  type SavePrintTemplateRequest,
} from "@/features/print-template/print-template-queries";

import { FIELD_SCOPE } from "@/lib/ui-strings";

import { H9HiprintDesigner, type H9HiprintDesignerHandle } from "./H9HiprintDesigner";

export type H9TemplateDesignerMode = "create" | "edit" | "copy";

interface H9TemplateDesignerDialogProps {
  open: boolean;
  mode?: H9TemplateDesignerMode;
  initialTemplate?: PrintTemplateVersion | null;
  templateTypes: PrintTemplateTypeRow[];
  libraries: PrintFieldLibraryRow[];
  onOpenChange: (open: boolean) => void;
  onSave: (request: SavePrintTemplateRequest) => Promise<void>;
}

export function H9TemplateDesignerDialog({
  open,
  mode = "create",
  initialTemplate = null,
  templateTypes,
  libraries,
  onOpenChange,
  onSave,
}: H9TemplateDesignerDialogProps) {
  const firstType = templateTypes[0];
  const designerRef = React.useRef<H9HiprintDesignerHandle | null>(null);
  const lastLibraryVersionIdRef = React.useRef<string | null>(null);
  const [templateCode, setTemplateCode] = React.useState("m2_asn_default");
  const [templateName, setTemplateName] = React.useState("M2 ASN 默认模板");
  const [templateTypeCode, setTemplateTypeCode] = React.useState(firstType?.code ?? "m2_asn");
  const [scope, setScope] = React.useState<"global" | "owner">("global");
  const [isDefault, setIsDefault] = React.useState(true);
  const [remark, setRemark] = React.useState("PC Web hiprint 设计器保存");
  const [paperPreset, setPaperPreset] = React.useState<PaperPreset>("A4");
  const [paperDirection, setPaperDirection] = React.useState<PaperDirection>("portrait");
  const [customPaperWidth, setCustomPaperWidth] = React.useState("100");
  const [customPaperHeight, setCustomPaperHeight] = React.useState("150");
  const [jsonText, setJsonText] = React.useState(() => JSON.stringify(defaultTemplateJson("asn.code", defaultPaper()), null, 2));
  const [jsonOpen, setJsonOpen] = React.useState(false);
  const [boundFields, setBoundFields] = React.useState<string[]>(["asn.code"]);
  const [error, setError] = React.useState<string | null>(null);
  const [designerReadyState, setDesignerReadyState] = React.useState<"initializing" | "ready" | "error">("initializing");
  const formRef = React.useRef<HTMLFormElement | null>(null);
  const selectedType = templateTypes.find((type) => type.code === templateTypeCode) ?? firstType;
  const selectedLibrary =
    libraries.find((library) => library.libraryCode === selectedType?.fieldLibraryCode) ?? null;
  const selectedLibraryVersionId = selectedLibrary?.publishedVersionId ?? null;
  const fieldsQuery = usePrintFieldDefinitionsQuery(selectedLibraryVersionId);
  const fields = fieldsQuery.data ?? [];
  const busy = fieldsQuery.isPending;

  React.useEffect(() => {
    if (!open) return;
    if (!templateTypes.some((type) => type.code === templateTypeCode) && firstType) {
      setTemplateTypeCode(firstType.code);
    }
  }, [firstType, open, templateTypeCode, templateTypes]);

  React.useEffect(() => {
    if (!open) {
      lastLibraryVersionIdRef.current = null;
      return;
    }
    const libraryVersionId = selectedLibraryVersionId;
    if (initialTemplate) {
      lastLibraryVersionIdRef.current = libraryVersionId;
      return;
    }
    if (!libraryVersionId || fields.length === 0) return;
    if (lastLibraryVersionIdRef.current === libraryVersionId) return;
    lastLibraryVersionIdRef.current = libraryVersionId;
    setBoundFields([fields[0].fieldPath]);
    setJsonText(JSON.stringify(defaultTemplateJson(fields[0].fieldPath, currentPaper()), null, 2));
  }, [fields, initialTemplate, open, selectedLibraryVersionId]);

  React.useEffect(() => {
    if (!open) return;
    setError(null);
    setJsonOpen(false);
    setDesignerReadyState("initializing");
    if (!initialTemplate) {
      const paper = defaultPaper();
      setTemplateCode("m2_asn_default");
      setTemplateName("M2 ASN 默认模板");
      setTemplateTypeCode(firstType?.code ?? "m2_asn");
      setScope("global");
      setIsDefault(true);
      setRemark("PC Web hiprint 设计器保存");
      setPaperPreset("A4");
      setPaperDirection("portrait");
      setCustomPaperWidth("100");
      setCustomPaperHeight("150");
      setBoundFields(["asn.code"]);
      setJsonText(JSON.stringify(defaultTemplateJson("asn.code", paper), null, 2));
      return;
    }
    const paperControls = paperControlsFromValue(initialTemplate.paper);
    setTemplateCode(mode === "copy" ? `${initialTemplate.template_code}_copy` : initialTemplate.template_code);
    setTemplateName(mode === "copy" ? `${initialTemplate.template_name} 副本` : initialTemplate.template_name);
    setTemplateTypeCode(initialTemplate.template_type_code);
    setScope(initialTemplate.scope);
    setIsDefault(initialTemplate.is_default);
    setRemark(initialTemplate.remark ?? "");
    setPaperPreset(paperControls.paperPreset);
    setPaperDirection(paperControls.paperDirection);
    setCustomPaperWidth(paperControls.customPaperWidth);
    setCustomPaperHeight(paperControls.customPaperHeight);
    setBoundFields(initialTemplate.field_bindings.map((binding) => binding.field_path));
    setJsonText(JSON.stringify(initialTemplate.hiprint_json, null, 2));
  }, [firstType?.code, initialTemplate, mode, open]);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      const paper = currentPaper();
      const hiprintJson = applyPaperToTemplateJson(designerRef.current?.getJson() ?? parseJson(jsonText), paper);
      const request: SavePrintTemplateRequest = {
        template_id: mode === "edit" ? initialTemplate?.template_id ?? null : null,
        template_code: requiredText(templateCode, "模板编码"),
        template_name: requiredText(templateName, "模板名称"),
        template_type_code: requiredText(templateTypeCode, "模板类型"),
        scope,
        is_default: isDefault,
        remark: remark.trim() || null,
        field_library_version_id: selectedLibraryVersionId ?? "",
        hiprint_json: hiprintJson,
        field_bindings: boundFields.map((fieldPath, index) => ({ field_path: fieldPath, required: index === 0 })),
        paper,
        designer_version: "hiprint@0.4.0",
      };
      await onSave(request);
      onOpenChange(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "保存失败");
    }
  }

  function toggleField(fieldPath: string) {
    setBoundFields((current) =>
      current.includes(fieldPath) ? current.filter((item) => item !== fieldPath) : [...current, fieldPath],
    );
  }

  function currentPaper() {
    return buildPaper(paperPreset, paperDirection, customPaperWidth, customPaperHeight);
  }

  function syncPaper(next: Partial<PaperControlState>) {
    const preset = next.paperPreset ?? paperPreset;
    const direction = next.paperDirection ?? paperDirection;
    const width = next.customPaperWidth ?? customPaperWidth;
    const height = next.customPaperHeight ?? customPaperHeight;
    const paper = buildPaper(preset, direction, width, height);
    setJsonText((current) => JSON.stringify(applyPaperToTemplateJson(parseJsonOrDefault(current), paper), null, 2));
  }

  const fieldBindingPanel = (
    <>
      <div className="text-xs text-muted-foreground">
        {selectedLibrary?.publishedVersionId
          ? `${selectedLibrary.libraryName} v${selectedLibrary.publishedVersionNo}`
          : "未绑定已发布字段库"}
      </div>
      <div className="mt-3 max-h-72 overflow-auto">
        {fields.map((field) => (
          <label key={field.fieldPath} className="flex items-start gap-2 rounded px-2 py-1 text-sm hover:bg-muted/40">
            <input type="checkbox" checked={boundFields.includes(field.fieldPath)} onChange={() => toggleField(field.fieldPath)} />
            <span>
              <span className="block text-foreground">{field.displayName}</span>
              <span className="block font-mono text-xs text-muted-foreground">{field.fieldPath}</span>
            </span>
          </label>
        ))}
        {busy && <div className="px-2 py-3 text-sm text-muted-foreground">加载字段...</div>}
      </div>
    </>
  );
  const templateSettingsPanel = (
    <div className="grid grid-cols-[repeat(auto-fit,minmax(9rem,12rem))] items-end gap-3 p-3">
      <Field label="模板编码">
        <Input className="h-8" value={templateCode} readOnly={mode === "edit"} onChange={(event) => setTemplateCode(event.target.value)} />
      </Field>
      <Field label="模板名称">
        <Input className="h-8" value={templateName} onChange={(event) => setTemplateName(event.target.value)} />
      </Field>
      <Field label="模板类型">
        <select className="h-8 w-full rounded-md border bg-background px-2 text-sm" value={templateTypeCode} disabled={mode === "edit"} onChange={(event) => setTemplateTypeCode(event.target.value)}>
          {templateTypes.map((type) => (
            <option key={type.code} value={type.code}>
              {type.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label={FIELD_SCOPE}>
        <select className="h-8 w-full rounded-md border bg-background px-2 text-sm" value={scope} onChange={(event) => setScope(event.target.value as "global" | "owner")}>
          <option value="global">全局默认</option>
          <option value="owner">货主覆盖</option>
        </select>
      </Field>
      <Field label="备注">
        <Input className="h-8" value={remark} onChange={(event) => setRemark(event.target.value)} />
      </Field>
      <label className="flex h-8 items-center gap-2 text-sm">
        <input type="checkbox" checked={isDefault} onChange={(event) => setIsDefault(event.target.checked)} />
        默认模板
      </label>
      <Field label="纸张大小">
        <select
          className="h-8 w-full rounded-md border bg-background px-2 text-sm"
          value={paperPreset}
          onChange={(event) => {
            const next = event.target.value as PaperPreset;
            setPaperPreset(next);
            syncPaper({ paperPreset: next });
          }}
        >
          <option value="A4">A4</option>
          <option value="A5">A5</option>
          <option value="custom">自定义</option>
        </select>
      </Field>
      <Field label="纸张方向">
        <select
          className="h-8 w-full rounded-md border bg-background px-2 text-sm"
          value={paperDirection}
          onChange={(event) => {
            const next = event.target.value as PaperDirection;
            setPaperDirection(next);
            syncPaper({ paperDirection: next });
          }}
        >
          <option value="portrait">竖向</option>
          <option value="landscape">横向</option>
        </select>
      </Field>
      {paperPreset === "custom" && (
        <>
          <Field label="自定义宽(mm)">
            <Input
              className="h-8"
              type="number"
              min="20"
              value={customPaperWidth}
              onChange={(event) => {
                setCustomPaperWidth(event.target.value);
                syncPaper({ customPaperWidth: event.target.value });
              }}
            />
          </Field>
          <Field label="自定义高(mm)">
            <Input
              className="h-8"
              type="number"
              min="20"
              value={customPaperHeight}
              onChange={(event) => {
                setCustomPaperHeight(event.target.value);
                syncPaper({ customPaperHeight: event.target.value });
              }}
            />
          </Field>
        </>
      )}
    </div>
  );
  const dialogTitle = mode === "edit" ? "修改打印模板" : mode === "copy" ? "复制打印模板" : "新增打印模板";
  const saveLabel = mode === "edit" ? "保存新草稿" : "保存草稿";
  const designSessionKey = open
    ? `${mode}:${initialTemplate?.id ?? "new"}:${initialTemplate?.template_code ?? "create"}`
    : "closed";
  const formSaveDisabled =
    designerReadyState !== "ready" || !selectedLibraryVersionId || boundFields.length === 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[92vh] max-w-[96vw] overflow-auto">
        <form ref={formRef} className="flex flex-col gap-4" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{dialogTitle}</DialogTitle>
            <DialogDescription>使用 hiprint 设计模板并保存草稿；发布需在模板列表中单独执行。</DialogDescription>
          </DialogHeader>

          <H9HiprintDesigner
            ref={designerRef}
            designSessionKey={designSessionKey}
            templateSettingsPanel={templateSettingsPanel}
            fieldBindingPanel={fieldBindingPanel}
            fields={fields.map((field) => ({ fieldPath: field.fieldPath, displayName: field.displayName }))}
            templateJson={parseJsonOrDefault(jsonText)}
            onJsonChange={(value) => setJsonText(JSON.stringify(value, null, 2))}
            onReadyStateChange={setDesignerReadyState}
            onCancel={() => onOpenChange(false)}
            onSave={() => formRef.current?.requestSubmit()}
            saveLabel={saveLabel}
            saveDisabled={!selectedLibraryVersionId || boundFields.length === 0}
          />

          <div className="rounded-md border bg-background">
            <button
              type="button"
              className="flex w-full items-center justify-between px-3 py-2 text-sm font-medium"
              onClick={() => setJsonOpen((current) => !current)}
            >
              <span>hiprint JSON</span>
              <span className="text-xs text-muted-foreground">{jsonOpen ? "隐藏" : "展开"}</span>
            </button>
            {jsonOpen && (
              <textarea
                className="min-h-40 w-full border-t bg-background p-3 font-mono text-xs outline-none"
                value={jsonText}
                onChange={(event) => setJsonText(event.target.value)}
              />
            )}
          </div>

          {error && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</div>}
          {designerReadyState === "error" && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              设计器未就绪，可取消关闭或检查浏览器控制台后重试。
            </div>
          )}

          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">取消</Button>
            </DialogClose>
            <Button type="submit" disabled={formSaveDisabled}>
              <Save className="size-4" aria-hidden />
              {saveLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Label className="flex flex-col items-stretch gap-1.5">
      <span>{label}</span>
      {children}
    </Label>
  );
}

type PaperPreset = "A4" | "A5" | "custom";
type PaperDirection = "portrait" | "landscape";
type PaperSettings = { paperType: PaperPreset; width: number; height: number; direction: PaperDirection };
type PaperControlState = {
  paperPreset: PaperPreset;
  paperDirection: PaperDirection;
  customPaperWidth: string;
  customPaperHeight: string;
};

const standardPaperSize: Record<Exclude<PaperPreset, "custom">, { width: number; height: number }> = {
  A4: { width: 210, height: 297 },
  A5: { width: 148, height: 210 },
};

function defaultPaper() {
  return buildPaper("A4", "portrait", "100", "150");
}

function paperControlsFromValue(value: unknown): PaperControlState {
  const record = cloneRecord(value);
  const paperType = record.paperType === "A4" || record.paperType === "A5" ? record.paperType : "custom";
  return {
    paperPreset: paperType,
    paperDirection: record.direction === "landscape" ? "landscape" : "portrait",
    customPaperWidth: String(typeof record.width === "number" ? record.width : 100),
    customPaperHeight: String(typeof record.height === "number" ? record.height : 150),
  };
}

function buildPaper(
  paperType: PaperPreset,
  direction: PaperDirection,
  customWidthText: string,
  customHeightText: string,
): PaperSettings {
  const base =
    paperType === "custom"
      ? { width: positiveNumber(customWidthText, 100), height: positiveNumber(customHeightText, 150) }
      : standardPaperSize[paperType];
  return {
    paperType,
    direction,
    width: direction === "landscape" ? base.height : base.width,
    height: direction === "landscape" ? base.width : base.height,
  };
}

function applyPaperToTemplateJson(value: unknown, paper: PaperSettings) {
  const record = cloneRecord(value);
  const panels = Array.isArray(record.panels) ? record.panels : [];
  const firstPanel = cloneRecord(panels[0]);
  record.panels = [
    {
      ...firstPanel,
      index: typeof firstPanel.index === "number" ? firstPanel.index : 0,
      paperType: paper.paperType,
      width: paper.width,
      height: paper.height,
      orient: paper.direction,
    },
    ...panels.slice(1),
  ];
  return record;
}

function defaultTemplateJson(fieldPath: string, paper: PaperSettings) {
  return {
    panels: [
      {
        index: 0,
        paperType: paper.paperType,
        width: paper.width,
        height: paper.height,
        orient: paper.direction,
        printElements: [
          {
            options: { field: fieldPath, title: fieldPath, left: 20, top: 20, width: 260, height: 20 },
            printElementType: { type: "text" },
          },
        ],
      },
    ],
  };
}

function parseJson(value: string): unknown {
  const parsed = JSON.parse(value) as unknown;
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error("hiprint JSON 必须是对象");
  return parsed;
}

function parseJsonOrDefault(value: string): unknown {
  try {
    return parseJson(value);
  } catch {
    return defaultTemplateJson("asn.code", defaultPaper());
  }
}

function requiredText(value: string, label: string) {
  const trimmed = value.trim();
  if (!trimmed) throw new Error(`${label}不能为空`);
  return trimmed;
}

function positiveNumber(value: string, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function cloneRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}
