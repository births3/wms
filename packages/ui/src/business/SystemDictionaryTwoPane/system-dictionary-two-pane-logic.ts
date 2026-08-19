export interface SystemDictionaryTwoPaneItem {
  code: string;
  name: string;
  source: string;
  enabled: boolean;
  sortOrder: number;
  params?: Record<string, unknown>;
}

export interface SystemDictionaryTwoPaneGroup {
  code: string;
  name: string;
  items: SystemDictionaryTwoPaneItem[];
}

export interface SystemDictionaryGroupSummary {
  code: string;
  name: string;
  enabledCount: number;
  totalCount: number;
}

export interface SystemDictionaryParamSummary {
  key: string;
  label: string;
  value: string;
}

const PARAM_LABELS: Record<string, string> = {
  field_library_code: "字段库编码",
  business_module: "业务模块",
  business_direction: "业务方向",
  paper_type: "纸张类型",
  default_scope: "默认作用域",
  direction: "业务方向",
  workflow_template: "流程模板",
  batch_policy: "批号策略",
};
const PARAM_VALUE_LABELS: Record<string, Record<string, string>> = {
  business_direction: { inbound: "入库", outbound: "出库", label: "标签" },
  paper_type: { a4: "A4", a5: "A5", label: "标签纸" },
  default_scope: { global: "全局", owner: "货主" },
};
const PARAM_KEY_ORDER = Object.keys(PARAM_LABELS);

export function summarizeSystemDictionaryGroup(
  group: SystemDictionaryTwoPaneGroup
): SystemDictionaryGroupSummary {
  return {
    code: group.code,
    name: group.name,
    enabledCount: group.items.filter((item) => item.enabled).length,
    totalCount: group.items.length,
  };
}

export function getSystemDictionarySelectedGroup(
  groups: SystemDictionaryTwoPaneGroup[],
  selectedCode?: string
) {
  return groups.find((group) => group.code === selectedCode) ?? groups[0];
}

export function summarizeSystemDictionaryParams(
  params: Record<string, unknown> = {}
): SystemDictionaryParamSummary[] {
  const knownKeys = new Set(PARAM_KEY_ORDER);
  const orderedEntries = PARAM_KEY_ORDER.flatMap((key) =>
    Object.prototype.hasOwnProperty.call(params, key) ? [[key, params[key]] as const] : []
  );
  const otherEntries = Object.entries(params).filter(([key]) => !knownKeys.has(key));

  return [...orderedEntries, ...otherEntries]
    .map(([key, value]) => {
      const rawValue = text(value);
      return {
        key,
        label: PARAM_LABELS[key] ?? key,
        value: PARAM_VALUE_LABELS[key]?.[rawValue] ?? rawValue,
      };
    })
    .filter((entry) => entry.value !== "-");
}

export function systemDictionarySourceText(source: string) {
  if (source === "global") return "全局";
  if (source === "owner" || source === "owner_override") return "货主覆盖";
  return source || "-";
}

function text(value: unknown) {
  if (typeof value === "string" && value.trim()) return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value === null || value === undefined) return "-";
  return JSON.stringify(value);
}
