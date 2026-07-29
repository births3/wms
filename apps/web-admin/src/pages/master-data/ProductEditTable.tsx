import type { DataGridColumn } from "@wms/ui";

import type { MasterDataRow } from "@/features/master-data/master-data-queries";

export function productColumns(
  baseColumns: DataGridColumn<MasterDataRow>[],
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
        <span className="text-sm">{productValue(row, "packagingText")}</span>
      ),
    },
    {
      key: "regulatoryCodes",
      header: "监管标识",
      width: 260,
      minWidth: 220,
      filterValue: regulatoryCodesText,
      copyValue: regulatoryCodesText,
      filter: { type: "text" },
      render: (row) => (
        <TwoLine
          top={`UDI ${productValue(row, "udiCode")}`}
          bottom={`电子监管码 ${productValue(row, "electronicRegulatoryCode")}`}
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
          top={`长 ${productValue(row, "lengthMm")} mm / 宽 ${productValue(row, "widthMm")} mm`}
          bottom={`高 ${productValue(row, "heightMm")} mm`}
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
          top={`重量 ${productValue(row, "weightG")} g`}
          bottom={`体积 ${productValue(row, "volumeCm3")} cm³`}
        />
      ),
    },
    {
      key: "mappingTrace",
      header: "映射溯源",
      width: 220,
      minWidth: 190,
      filterValue: mappingTraceText,
      copyValue: mappingTraceText,
      filter: { type: "text" },
      render: (row) => <span className="text-sm">{mappingTraceText(row)}</span>,
    },
  ];
}

function productValue(
  row: MasterDataRow,
  key:
    | "electronicRegulatoryCode"
    | "heightMm"
    | "lengthMm"
    | "packagingText"
    | "udiCode"
    | "volumeCm3"
    | "weightG"
    | "widthMm",
) {
  return row.productFields?.[key]?.trim() || "-";
}

function productPackagingText(row: MasterDataRow) {
  return productValue(row, "packagingText");
}

function unitSizeText(row: MasterDataRow) {
  return `长 ${productValue(row, "lengthMm")} mm / 宽 ${productValue(row, "widthMm")} mm / 高 ${productValue(row, "heightMm")} mm`;
}

function unitWeightVolumeText(row: MasterDataRow) {
  return `重量 ${productValue(row, "weightG")} g / 体积 ${productValue(row, "volumeCm3")} cm³`;
}

function regulatoryCodesText(row: MasterDataRow) {
  return `UDI ${productValue(row, "udiCode")} / 电子监管码 ${productValue(row, "electronicRegulatoryCode")}`;
}

function mappingTraceText(row: MasterDataRow) {
  const traces = row.productFields?.mappingTraces ?? [];
  if (traces.length === 0) return "无外部映射";
  const latest = traces[traces.length - 1];
  return `${latest?.source_system ?? "-"} · ${traces.length} 条`;
}

function TwoLine({ top, bottom }: { top: string; bottom: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{top}</div>
      <div className="text-xs text-muted-foreground">{bottom}</div>
    </div>
  );
}
