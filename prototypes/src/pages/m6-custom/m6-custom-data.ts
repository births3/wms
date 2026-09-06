export interface PreviewRow {
  dim: string;
  m1: number;
  m2: number;
  m3: number;
}

export const PREVIEW_DATA: PreviewRow[] = [
  { dim: "国药控股北京", m1: 12, m2: 1820, m3: 215400 },
  { dim: "上海医药华东", m1: 9, m2: 1320, m3: 168200 },
  { dim: "九州通医药", m1: 8, m2: 980, m3: 92400 },
  { dim: "甘李药业", m1: 5, m2: 580, m3: 286800 },
  { dim: "东北制药", m1: 4, m2: 480, m3: 24600 },
  { dim: "其他（12 家）", m1: 18, m2: 2240, m3: 137200 },
];

export interface SavedTemplate {
  id: string;
  name: string;
  scope: "私有" | "部门" | "全局";
  lastRun: string;
  isFavorite?: boolean;
  dashboard: number;
}

export const SAVED_TEMPLATES: SavedTemplate[] = [
  { id: "t1", name: "供应商月度采购排行", scope: "私有", lastRun: "今日 09:14", isFavorite: true, dashboard: 12 },
  { id: "t2", name: "冷链商品销售趋势", scope: "私有", lastRun: "昨日 16:30", isFavorite: true, dashboard: 13 },
  { id: "t3", name: "麻精药品月度核对", scope: "部门", lastRun: "5/15 10:08", dashboard: 14 },
  { id: "t4", name: "客户退货率分析", scope: "全局", lastRun: "5/12 14:22", dashboard: 15 },
  { id: "t5", name: "盘点差异趋势", scope: "私有", lastRun: "5/10 11:00", dashboard: 16 },
];

export interface FieldChip {
  id: string;
  label: string;
  type: "dim" | "metric";
  agg?: "sum" | "avg" | "count";
}

export const ALL_FIELDS: FieldChip[] = [
  { id: "supplier", label: "供应商", type: "dim" },
  { id: "date_month", label: "日期（月）", type: "dim" },
  { id: "date_day", label: "日期（日）", type: "dim" },
  { id: "warehouse", label: "仓库", type: "dim" },
  { id: "owner", label: "货主", type: "dim" },
  { id: "category", label: "商品分类", type: "dim" },
  { id: "asn_count", label: "入库单数", type: "metric", agg: "sum" },
  { id: "qty", label: "总件数", type: "metric", agg: "sum" },
  { id: "amount", label: "总金额", type: "metric", agg: "sum" },
  { id: "exception", label: "异常单数", type: "metric", agg: "count" },
  { id: "avg_price", label: "平均单价", type: "metric", agg: "avg" },
];

export interface FilterCondition {
  id: string;
  field: string;
  op: string;
  value: string;
}

export interface FilterGroup {
  id: string;
  connector: "AND" | "OR";
  conditions: FilterCondition[];
}

export const INITIAL_FILTERS: FilterGroup[] = [
  {
    id: "g1",
    connector: "AND",
    conditions: [
      { id: "c1", field: "date_month", op: ">=", value: "2026-04" },
      { id: "c2", field: "date_month", op: "<=", value: "2026-04" },
      { id: "c3", field: "amount", op: ">", value: "10000" },
    ],
  },
];

export const VALUE_METRICS = [
  { label: "Σ 入库单数", agg: "sum" },
  { label: "Σ 总件数", agg: "sum" },
  { label: "Σ 总金额", agg: "sum" },
];

export type ChartType = "table" | "bar" | "line" | "pie" | "area";
