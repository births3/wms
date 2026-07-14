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
} from "@wms/ui";
import { Upload } from "lucide-react";

import type { CreateProductRequest } from "@/features/master-data/master-data-queries";

interface ProductBatchImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImport: (requests: CreateProductRequest[]) => Promise<void>;
}

const placeholder = [
  "product_code,product_name,spec,approval_no,dosage_form,manufacturer,storage_condition,special_drug_category_code",
  "P-001,测试商品,10ml*1支,国药准字A,注射剂,测试药业,normal,none",
].join("\n");

export function ProductBatchImportDialog({
  open,
  onOpenChange,
  onImport,
}: ProductBatchImportDialogProps) {
  const [text, setText] = React.useState("");
  const [message, setMessage] = React.useState<string | null>(null);
  const [submitting, setSubmitting] = React.useState(false);
  const preview = React.useMemo(() => {
    if (!text.trim()) return { rows: [], error: null };
    try {
      return { rows: parseProductImportText(text), error: null };
    } catch (error) {
      return { rows: [], error: error instanceof Error ? error.message : "导入内容格式错误" };
    }
  }, [text]);

  function close(openValue: boolean) {
    if (openValue) return onOpenChange(true);
    if (!submitting) {
      setText("");
      setMessage(null);
      onOpenChange(false);
    }
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setMessage(null);
    try {
      await onImport(parseProductImportText(text));
      setText("");
      onOpenChange(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "批量导入商品失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-5xl">
        <form className="grid gap-3" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>批量导入商品</DialogTitle>
            <DialogDescription>
              粘贴 Excel 复制出的表格或 CSV/TSV 文本，提交前先预览解析结果。
            </DialogDescription>
          </DialogHeader>
          <label className="grid gap-1 text-xs text-muted-foreground">
            导入内容
            <textarea
              className="min-h-52 rounded-md border border-input bg-background px-3 py-2 font-mono text-sm text-foreground shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              value={text}
              onChange={(event) => {
                setText(event.target.value);
                setMessage(null);
              }}
              placeholder={placeholder}
              aria-label="商品批量导入内容"
            />
          </label>
          {preview.error && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
              {preview.error}
            </div>
          )}
          {preview.rows.length > 0 && (
            <div className="overflow-x-auto rounded-md border">
              <div className="border-b bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
                已解析 {preview.rows.length} 条，预览前 8 条
              </div>
              <table className="w-full min-w-[760px] text-left text-xs">
                <thead className="border-b bg-muted/20">
                  <tr>
                    <th className="px-3 py-2">编码</th>
                    <th className="px-3 py-2">名称</th>
                    <th className="px-3 py-2">规格</th>
                    <th className="px-3 py-2">储存条件</th>
                    <th className="px-3 py-2">特殊药品分类</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.rows.slice(0, 8).map((row) => (
                    <tr key={row.product_code} className="border-b last:border-0">
                      <td className="px-3 py-2 font-medium">{row.product_code}</td>
                      <td className="px-3 py-2">{row.product_name}</td>
                      <td className="px-3 py-2">{row.spec ?? "-"}</td>
                      <td className="px-3 py-2">{String(row.attrs?.storage_condition ?? "normal")}</td>
                      <td className="px-3 py-2">{row.special_drug_category_code ?? "none"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {message && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
              {message}
            </div>
          )}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={submitting}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={submitting || preview.rows.length === 0 || Boolean(preview.error)}>
              <Upload className="size-4" aria-hidden />
              {submitting ? "导入中..." : "确认导入"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function parseProductImportText(value: string): CreateProductRequest[] {
  const lines = value
    .split(/\r?\n/)
    .map((line, index) => ({ line: line.trim(), number: index + 1 }))
    .filter((item) => item.line.length > 0);
  if (lines.length === 0) throw new Error("请至少填写一行商品数据");
  const dataLines = isProductHeader(splitRow(lines[0].line)) ? lines.slice(1) : lines;
  if (dataLines.length === 0) throw new Error("请至少填写一行商品数据");
  return dataLines.map(({ line, number }) => {
    const [code, name, spec, approvalNo, dosageForm, manufacturer, storage, category] = splitRow(line);
    return {
      product_code: required(code, `第 ${number} 行商品编码`),
      product_name: required(name, `第 ${number} 行商品名称`),
      approval_no: nullable(approvalNo),
      spec: nullable(spec),
      dosage_form: nullable(dosageForm),
      manufacturer: nullable(manufacturer),
      special_drug_category_code: nullable(category) ?? "none",
      attrs: {
        storage_condition: nullable(storage) ?? "normal",
        source: "batch_import",
      },
    };
  });
}

function splitRow(line: string) {
  return line.split(/\t|,|，/).map((cell) => cell.trim());
}

function isProductHeader(cells: string[]) {
  const first = cells[0]?.toLowerCase();
  const second = cells[1]?.toLowerCase();
  return ["product_code", "商品编码", "编码"].includes(first ?? "") &&
    ["product_name", "商品名称", "名称"].includes(second ?? "");
}

function required(value: string | undefined, field: string) {
  const result = value?.trim() ?? "";
  if (!result) throw new Error(`${field}必填`);
  return result;
}

function nullable(value: string | undefined) {
  const result = value?.trim() ?? "";
  return result || null;
}
