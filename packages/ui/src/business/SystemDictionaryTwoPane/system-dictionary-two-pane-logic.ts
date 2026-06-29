export interface SystemDictionaryTwoPaneItem {
  code: string;
  name: string;
  source: string;
  enabled: boolean;
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
  value: string;
}

const PARAM_KEY_ORDER = ["direction", "workflow_template", "batch_policy"];

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
    .map(([key, value]) => ({ key, value: text(value) }))
    .filter((entry) => entry.value !== "-");
}

export function systemDictionarySourceText(source: string) {
  if (source === "global") return "全局";
  if (source === "owner_override") return "货主覆盖";
  return source || "-";
}

function text(value: unknown) {
  if (typeof value === "string" && value.trim()) return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value === null || value === undefined) return "-";
  return JSON.stringify(value);
}
