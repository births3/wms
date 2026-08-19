import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  formatDateTime,
} from "@wms/ui";

import {
  storageConditionDisplayLabel,
  type MasterDataRow,
} from "@/features/master-data/master-data-queries";
import { COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_UPDATED_AT } from "@/lib/ui-strings";

export interface ProductDetailDialogProps {
  /** 选中的商品行；null 时对话框关闭 */
  row: MasterDataRow | null;
  /** 特殊药品字典 code → 名称映射，无映射时显示 code */
  specialDrugCategoryLabels: ReadonlyMap<string, string>;
  onOpenChange: (open: boolean) => void;
}

/** M1 商品档案只读详情对话框（本页只读，无编辑入口） */
export function ProductDetailDialog({
  row,
  specialDrugCategoryLabels,
  onOpenChange,
}: ProductDetailDialogProps) {
  const open = Boolean(row);
  const fields = row?.productFields;
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] w-[calc(100vw-2rem)] max-w-3xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {row ? `${row.name}（${row.code}）` : "商品详情"}
          </DialogTitle>
          <DialogDescription>商品档案只读详情，变更由 H8 商品消息同步。</DialogDescription>
        </DialogHeader>

        {row && fields && (
          <div className="grid gap-5 py-2">
            <div className="grid gap-x-6 gap-y-3 sm:grid-cols-2">
              <FieldRow label="商品编号" value={row.code} mono />
              <FieldRow label="商品名称" value={row.name} />
              <FieldRow label="规格" value={text(fields.spec)} />
              <FieldRow label="批准文号" value={text(fields.approvalNo)} />
              <FieldRow label="剂型" value={text(fields.dosageForm)} />
              <FieldRow label="生产厂家" value={text(fields.manufacturer)} />
              <FieldRow
                label="特殊药品标识"
                value={specialDrugCategoryLabel(fields.specialDrugCategoryCode, specialDrugCategoryLabels)}
              />
              <FieldRow label="储存条件" value={storageConditionDisplayLabel(fields.storageCondition)} />
              <FieldRow label="UDI" value={text(fields.udiCode)} />
              <FieldRow label="电子监管码" value={text(fields.electronicRegulatoryCode)} />
              <FieldRow label="69 码" value={text(fields.barcode69)} mono />
              <FieldRow label={COLUMN_STATUS} value={row.statusLabel} />
              <FieldRow label={COLUMN_CREATED_AT} value={formatDateTime(row.createdAt)} />
              <FieldRow label={COLUMN_UPDATED_AT} value={formatDateTime(row.updatedAt)} />
            </div>

            <div>
              <h3 className="mb-2 text-sm font-medium">包装层级</h3>
              {fields.packagingLevels.length === 0 ? (
                <p className="text-sm text-muted-foreground">-</p>
              ) : (
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-xs text-muted-foreground">
                      <th className="py-1.5 pr-4 font-medium">单位</th>
                      <th className="py-1.5 pr-4 font-medium">相对基础转换比</th>
                      <th className="py-1.5 font-medium">属性</th>
                    </tr>
                  </thead>
                  <tbody>
                    {fields.packagingLevels.map((level) => (
                      <tr key={level.id} className="border-b border-border/60">
                        <td className="py-1.5 pr-4">{level.unitName}</td>
                        <td className="py-1.5 pr-4 font-mono">{level.ratioToBase}</td>
                        <td className="py-1.5">
                          {[level.isBase ? "基础" : null, level.isDefault ? "默认" : null]
                            .filter(Boolean)
                            .join(" / ") || "-"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">关闭</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function FieldRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={`text-sm ${mono ? "font-mono" : ""}`}>{value || "-"}</span>
    </div>
  );
}

function specialDrugCategoryLabel(
  code: string | null | undefined,
  labels: ReadonlyMap<string, string>,
) {
  const normalized = code?.trim();
  if (!normalized) return "-";
  return labels.get(normalized) ?? normalized;
}

function text(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : "-";
}
