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
} from "@wms/ui";
import { Plus, Upload } from "lucide-react";

import type {
  CreateCustomerRequest,
  CreateSupplierRequest,
} from "@/features/master-data/master-data-queries";

type SourceActionKind = "supplier" | "customer";
type ActiveDialog = "create" | "import" | null;

type SourceActionProps =
  | {
      kind: "supplier";
      onCreate: (request: CreateSupplierRequest) => Promise<void>;
      onImport: (requests: CreateSupplierRequest[]) => Promise<void>;
      showTriggers?: boolean;
    }
  | {
      kind: "customer";
      onCreate: (request: CreateCustomerRequest) => Promise<void>;
      onImport: (requests: CreateCustomerRequest[]) => Promise<void>;
      showTriggers?: boolean;
    };

export interface MasterDataSourceActionsHandle {
  openCreate: () => void;
  openImport: () => void;
}

interface SourceActionForm {
  code: string;
  name: string;
  licenseNo: string;
  contactName: string;
}

const emptyForm: SourceActionForm = {
  code: "",
  name: "",
  licenseNo: "",
  contactName: "",
};

const copy: Record<
  SourceActionKind,
  {
    createTitle: string;
    createButton: string;
    codeLabel: string;
    nameLabel: string;
    importButton: string;
    importTitle: string;
    importPlaceholder: string;
  }
> = {
  supplier: {
    createTitle: "新建供应商",
    createButton: "新建供应商",
    codeLabel: "供应商编码",
    nameLabel: "供应商名称",
    importButton: "导入供应商",
    importTitle: "批量导入供应商",
    importPlaceholder: [
      "supplier_code,supplier_name,license_no,contact_name",
      "S-001,配送供应商A,SPL-001,王供应",
      "S-002,配送供应商B,,",
    ].join("\n"),
  },
  customer: {
    createTitle: "新建客户",
    createButton: "新建客户",
    codeLabel: "客户编码",
    nameLabel: "客户名称",
    importButton: "导入客户",
    importTitle: "批量导入客户",
    importPlaceholder: [
      "customer_code,customer_name,license_no",
      "C-001,连锁门店A,LIC-001",
      "C-002,连锁门店B,",
    ].join("\n"),
  },
};

export const MasterDataSourceActions = React.forwardRef<MasterDataSourceActionsHandle, SourceActionProps>(
  function MasterDataSourceActions(props, ref) {
  const labels = copy[props.kind];
  const showTriggers = props.showTriggers ?? true;
  const [activeDialog, setActiveDialog] = React.useState<ActiveDialog>(null);
  const [form, setForm] = React.useState<SourceActionForm>(emptyForm);
  const [importText, setImportText] = React.useState("");
  const [message, setMessage] = React.useState<string | null>(null);
  const [submitting, setSubmitting] = React.useState(false);

  function updateForm(patch: Partial<SourceActionForm>) {
    setForm((value) => ({ ...value, ...patch }));
    setMessage(null);
  }

  function openDialog(dialog: ActiveDialog) {
    setForm(emptyForm);
    setImportText("");
    setMessage(null);
    setActiveDialog(dialog);
  }

  React.useImperativeHandle(ref, () => ({
    openCreate: () => openDialog("create"),
    openImport: () => openDialog("import"),
  }));

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setMessage(null);
    try {
      if (props.kind === "supplier") {
        await props.onCreate(supplierFormToRequest(form));
      } else {
        await props.onCreate(customerFormToRequest(form));
      }
      setActiveDialog(null);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `${labels.createTitle}失败`);
    } finally {
      setSubmitting(false);
    }
  }

  async function submitImport(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setMessage(null);
    try {
      if (props.kind === "supplier") {
        await props.onImport(parseSupplierImportText(importText));
      } else {
        await props.onImport(parseCustomerImportText(importText));
      }
      setActiveDialog(null);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : `${labels.importTitle}失败`);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      {showTriggers && (
        <>
          <Button type="button" onClick={() => openDialog("create")} disabled={submitting}>
            <Plus className="size-4" aria-hidden />
            {labels.createButton}
          </Button>
          <Button type="button" variant="outline" onClick={() => openDialog("import")} disabled={submitting}>
            <Upload className="size-4" aria-hidden />
            {labels.importButton}
          </Button>
        </>
      )}

      <Dialog open={activeDialog !== null} onOpenChange={(open) => !open && !submitting && setActiveDialog(null)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
          {activeDialog === "create" && (
            <form className="grid gap-3 md:grid-cols-2" onSubmit={submitCreate}>
              <DialogHeader className="md:col-span-2">
                <DialogTitle>{labels.createTitle}</DialogTitle>
                <DialogDescription>录入档案基础字段。</DialogDescription>
              </DialogHeader>
              <TextField label={labels.codeLabel} required value={form.code} onChange={(code) => updateForm({ code })} />
              <TextField label={labels.nameLabel} required value={form.name} onChange={(name) => updateForm({ name })} />
              <TextField label="资质证号" value={form.licenseNo} onChange={(licenseNo) => updateForm({ licenseNo })} />
              {props.kind === "supplier" && (
                <TextField
                  label="联系人"
                  value={form.contactName}
                  onChange={(contactName) => updateForm({ contactName })}
                />
              )}
              <DialogMessage message={message} />
              <DialogFooter className="md:col-span-2">
                <CancelButton disabled={submitting} />
                <Button type="submit" disabled={submitting}>
                  <Plus className="size-4" aria-hidden />
                  {submitting ? "提交中..." : labels.createButton}
                </Button>
              </DialogFooter>
            </form>
          )}

          {activeDialog === "import" && (
            <form className="grid gap-3" onSubmit={submitImport}>
              <DialogHeader>
                <DialogTitle>{labels.importTitle}</DialogTitle>
                <DialogDescription>每行一条档案，支持逗号或 Tab 分隔。</DialogDescription>
              </DialogHeader>
              <label className="grid gap-1 text-xs text-muted-foreground">
                导入内容
                <textarea
                  className={[
                    "min-h-56 rounded-md border border-input bg-background px-3 py-2",
                    "font-mono text-sm text-foreground shadow-sm",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  ].join(" ")}
                  value={importText}
                  onChange={(event) => {
                    setImportText(event.target.value);
                    setMessage(null);
                  }}
                  placeholder={labels.importPlaceholder}
                  aria-label={labels.importTitle}
                />
              </label>
              <DialogMessage message={message} />
              <DialogFooter>
                <CancelButton disabled={submitting} />
                <Button type="submit" disabled={submitting}>
                  <Upload className="size-4" aria-hidden />
                  {submitting ? "导入中..." : "确认导入"}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
});
MasterDataSourceActions.displayName = "MasterDataSourceActions";

export function parseSupplierImportText(textValue: string): CreateSupplierRequest[] {
  return importRows(textValue, isSupplierImportHeader).map((row) => {
    const [supplierCode, supplierName, licenseNo = "", contactName = ""] = splitImportRow(row.line);
    return {
      supplier_code: requiredText(supplierCode, `第 ${row.lineNumber} 行供应商编码`),
      supplier_name: requiredText(supplierName, `第 ${row.lineNumber} 行供应商名称`),
      license_no: nullableText(licenseNo),
      contact_name: nullableText(contactName),
      source: "batch_import",
    };
  });
}

export function parseCustomerImportText(textValue: string): CreateCustomerRequest[] {
  return importRows(textValue, isCustomerImportHeader).map((row) => {
    const [customerCode, customerName, licenseNo = ""] = splitImportRow(row.line);
    return {
      customer_code: requiredText(customerCode, `第 ${row.lineNumber} 行客户编码`),
      customer_name: requiredText(customerName, `第 ${row.lineNumber} 行客户名称`),
      license_no: nullableText(licenseNo),
      source: "batch_import",
    };
  });
}

function supplierFormToRequest(form: SourceActionForm): CreateSupplierRequest {
  return {
    supplier_code: requiredText(form.code, "供应商编码"),
    supplier_name: requiredText(form.name, "供应商名称"),
    license_no: nullableText(form.licenseNo),
    contact_name: nullableText(form.contactName),
    source: "manual",
  };
}

function customerFormToRequest(form: SourceActionForm): CreateCustomerRequest {
  return {
    customer_code: requiredText(form.code, "客户编码"),
    customer_name: requiredText(form.name, "客户名称"),
    license_no: nullableText(form.licenseNo),
    source: "manual",
  };
}

function importRows(textValue: string, isHeader: (cells: string[]) => boolean) {
  const rows = textValue
    .split(/\r?\n/)
    .map((line, index) => ({ line: line.trim(), lineNumber: index + 1 }))
    .filter((row) => row.line.length > 0);
  const dataRows = rows.filter((row, index) => index !== 0 || !isHeader(splitImportRow(row.line)));
  if (dataRows.length === 0) throw new Error("请至少填写一行导入数据");
  return dataRows;
}

function splitImportRow(line: string) {
  return line.split(/\t|,|，/).map((cell) => cell.trim());
}

function isSupplierImportHeader(cells: string[]) {
  return isHeaderPair(cells, ["supplier_code", "供应商编码", "编码"], ["supplier_name", "供应商名称", "名称"]);
}

function isCustomerImportHeader(cells: string[]) {
  return isHeaderPair(cells, ["customer_code", "客户编码", "编码"], ["customer_name", "客户名称", "名称"]);
}

function isHeaderPair(cells: string[], firstValues: string[], secondValues: string[]) {
  const first = cells[0]?.toLowerCase() ?? "";
  const second = cells[1]?.toLowerCase() ?? "";
  return firstValues.includes(first) && secondValues.includes(second);
}

function DialogMessage({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div
      className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2"
      role="alert"
    >
      {message}
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  required,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
}) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Input required={required} value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function CancelButton({ disabled }: { disabled: boolean }) {
  return (
    <DialogClose asChild>
      <Button type="button" variant="outline" disabled={disabled}>
        取消
      </Button>
    </DialogClose>
  );
}

function requiredText(value: string | undefined, field: string) {
  const trimmed = value?.trim() ?? "";
  if (!trimmed) throw new Error(`${field}必填`);
  return trimmed;
}

function nullableText(value: string | undefined) {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}
