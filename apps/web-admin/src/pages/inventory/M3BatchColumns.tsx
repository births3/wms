import { StatusBadge, type DataGridColumn } from "@wms/ui";

import type { InventoryBatch } from "@/features/inventory/inventory-queries";
import {
  availableQty,
  expiryCopyValue,
  ExpiryDateCell,
  formatDateTime,
  qualityStatusKey,
  qualityStatusLabel,
  type QualityStatusOption,
} from "./M3BatchViewHelpers";

export function buildBatchColumns(
  onOpenDetail: (id: string) => void,
  expiryWarningDays: number,
  qualityStatusOptions: QualityStatusOption[],
): DataGridColumn<InventoryBatch>[] {
  return [
    {
      key: "batch_no", header: "批号", mono: true, width: 190, minWidth: 170, sortable: true,
      sortValue: (row) => row.batch_no, filterValue: (row) => row.batch_no, copyValue: (row) => row.batch_no,
      filter: { type: "text" }, onDoubleClick: (row) => onOpenDetail(row.id),
      render: (row) => <span className="text-primary">{row.batch_no}</span>,
    },
    {
      key: "product_code", header: "商品编码", mono: true, width: 170, minWidth: 150, sortable: true,
      sortValue: (row) => row.product_code, filterValue: (row) => row.product_code,
      copyValue: (row) => row.product_code, filter: { type: "text" },
    },
    {
      key: "product_name", header: "商品信息", width: 220, minWidth: 180, sortable: true,
      sortValue: (row) => row.product_name ?? "",
      filterValue: (row) => [row.product_name, row.specification, row.manufacturer].filter(Boolean).join(" "),
      copyValue: (row) => [row.product_name, row.specification, row.manufacturer].filter(Boolean).join(" / "),
      filter: { type: "text" },
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.product_name ?? "—"}</div>
          <div className="text-xs text-muted-foreground">
            {[row.specification, row.manufacturer].filter(Boolean).join(" / ") || "暂无规格/厂家"}
          </div>
        </div>
      ),
    },
    {
      key: "location_code", header: "库位", mono: true, width: 150, minWidth: 130, sortable: true,
      sortValue: (row) => row.location_code, filterValue: (row) => row.location_code,
      copyValue: (row) => row.location_code, filter: { type: "text" },
    },
    {
      key: "container_lpn", header: "容器 / 托盘", mono: true, width: 160, minWidth: 140, sortable: true,
      sortValue: (row) => row.container_lpn ?? "", filterValue: (row) => row.container_lpn ?? "",
      copyValue: (row) => row.container_lpn ?? "", filter: { type: "text" },
      render: (row) => row.container_lpn ?? <span className="text-muted-foreground">未绑定</span>,
    },
    {
      key: "quantity", header: "数量", width: 210, minWidth: 190, sortable: true,
      sortValue: availableQty, filterValue: availableQty,
      copyValue: (row) => `现存 ${row.qty_on_hand} / 锁定 ${row.qty_locked} / 可用 ${availableQty(row)}`,
      filter: { type: "numberRange" },
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.qty_on_hand} 件</div>
          <div className="text-xs text-muted-foreground">
            锁定 {row.qty_locked} / 可用 {availableQty(row)}
          </div>
        </div>
      ),
    },
    {
      key: "quality_status", header: "质量状态", width: 150, minWidth: 130, sortable: true,
      sortValue: (row) => qualityStatusLabel(row.quality_status, qualityStatusOptions),
      filterValue: (row) => row.quality_status,
      copyValue: (row) => qualityStatusLabel(row.quality_status, qualityStatusOptions),
      filter: { type: "multiSelect", options: qualityStatusOptions },
      render: (row) => (
        <StatusBadge
          status={qualityStatusKey(row.quality_status, row.recall_flag)}
          label={qualityStatusLabel(row.quality_status, qualityStatusOptions)}
          size="sm"
        />
      ),
    },
    {
      key: "recall_flag", header: "召回", width: 120, minWidth: 110, sortable: true,
      sortValue: (row) => (row.recall_flag ? 1 : 0),
      filterValue: (row) => (row.recall_flag ? "true" : "false"),
      copyValue: (row) => (row.recall_flag ? "已标记" : "未标记"),
      filter: {
        type: "multiSelect",
        options: [{ label: "已标记", value: "true" }, { label: "未标记", value: "false" }],
      },
      render: (row) => row.recall_flag
        ? <StatusBadge status="isolated" label="已标记" size="sm" />
        : <span className="text-muted-foreground">未标记</span>,
    },
    {
      key: "production_date", header: "生产日期", width: 150, minWidth: 130, sortable: true,
      sortValue: (row) => row.production_date, filterValue: (row) => row.production_date,
      copyValue: (row) => row.production_date, filter: { type: "dateRange" },
    },
    {
      key: "expiry_date", header: "有效期", width: 170, minWidth: 150, sortable: true,
      sortValue: (row) => row.expiry_date, filterValue: (row) => row.expiry_date,
      copyValue: (row) => expiryCopyValue(row.expiry_date, expiryWarningDays), filter: { type: "dateRange" },
      render: (row) => <ExpiryDateCell expiryDate={row.expiry_date} warningDays={expiryWarningDays} />,
    },
    {
      key: "created_at", header: "创建时间", width: 190, minWidth: 180, sortable: true,
      sortValue: (row) => row.created_at, filterValue: (row) => row.created_at,
      copyValue: (row) => formatDateTime(row.created_at), filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.created_at),
    },
    {
      key: "updated_at", header: "更新时间", width: 190, minWidth: 180, sortable: true,
      sortValue: (row) => row.updated_at, filterValue: (row) => row.updated_at,
      copyValue: (row) => formatDateTime(row.updated_at), filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.updated_at),
    },
  ];
}
