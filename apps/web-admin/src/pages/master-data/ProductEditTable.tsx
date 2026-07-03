import { Button, type DataGridColumn } from "@wms/ui";
import { Pencil } from "lucide-react";

import type {
  MasterDataRow,
  MasterDataViewId,
} from "@/features/master-data/master-data-queries";

export function masterDataGridClassName(viewId: MasterDataViewId) {
  if (viewId === "m1-locations") return "min-w-[1910px]";
  if (viewId === "m1-products") return "min-w-[2380px]";
  return "min-w-[1650px]";
}

export function productColumns(
  baseColumns: DataGridColumn<MasterDataRow>[],
  onEdit: (row: MasterDataRow) => void,
): DataGridColumn<MasterDataRow>[] {
  return [
    ...baseColumns,
    {
      key: "productPackaging",
      header: "包装层级",
      width: 220,
      minWidth: 190,
      filterValue: productPackagingText,
      copyValue: productPackagingText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`中包装 ${productValue(row, "middlePackage")}`}
          bottom={`大包装 ${productValue(row, "largePackage")}`}
        />
      ),
    },
    {
      key: "unitSize",
      header: "单位尺寸",
      width: 260,
      minWidth: 220,
      filterValue: unitSizeText,
      copyValue: unitSizeText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`长 ${productValue(row, "unitLengthMm")} mm / 宽 ${productValue(row, "unitWidthMm")} mm`}
          bottom={`高 ${productValue(row, "unitHeightMm")} mm`}
        />
      ),
    },
    {
      key: "unitWeightVolume",
      header: "重量 / 体积",
      width: 230,
      minWidth: 200,
      filterValue: unitWeightVolumeText,
      copyValue: unitWeightVolumeText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`重量 ${productValue(row, "unitWeightG")} g`}
          bottom={`体积 ${productValue(row, "unitVolumeCm3")} cm³`}
        />
      ),
    },
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

function productValue(
  row: MasterDataRow,
  key:
    | "middlePackage"
    | "largePackage"
    | "unitLengthMm"
    | "unitWidthMm"
    | "unitHeightMm"
    | "unitWeightG"
    | "unitVolumeCm3",
) {
  return row.productFields?.[key]?.trim() || "-";
}

function productPackagingText(row: MasterDataRow) {
  return `中包装 ${productValue(row, "middlePackage")} / 大包装 ${productValue(row, "largePackage")}`;
}

function unitSizeText(row: MasterDataRow) {
  return `长 ${productValue(row, "unitLengthMm")} mm / 宽 ${productValue(row, "unitWidthMm")} mm / 高 ${productValue(row, "unitHeightMm")} mm`;
}

function unitWeightVolumeText(row: MasterDataRow) {
  return `重量 ${productValue(row, "unitWeightG")} g / 体积 ${productValue(row, "unitVolumeCm3")} cm³`;
}

function TwoLine({ top, bottom }: { top: string; bottom: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{top}</div>
      <div className="text-xs text-muted-foreground">{bottom}</div>
    </div>
  );
}
