import { Button, type DataGridColumn } from "@wms/ui";
import { Pencil } from "lucide-react";

import type {
  MasterDataRow,
  MasterDataViewId,
} from "@/features/master-data/master-data-queries";

export function masterDataGridClassName(viewId: MasterDataViewId) {
  if (viewId === "m1-locations") return "min-w-[1910px]";
  if (viewId === "m1-products") return "min-w-[1800px]";
  return "min-w-[1650px]";
}

export function productColumns(
  baseColumns: DataGridColumn<MasterDataRow>[],
  onEdit: (row: MasterDataRow) => void,
): DataGridColumn<MasterDataRow>[] {
  return [
    ...baseColumns,
    {
      key: "actions",
      header: "操作",
      width: 150,
      minWidth: 140,
      align: "right",
      sortable: false,
      filter: false,
      copyable: false,
      hideable: false,
      render: (row) => (
        <Button
          type="button"
          variant="outline"
          size="sm"
          aria-label={`编辑商品 ${row.code}`}
          onClick={(event) => {
            event.stopPropagation();
            onEdit(row);
          }}
        >
          <Pencil className="size-4" aria-hidden />
          编辑
        </Button>
      ),
    },
  ];
}
