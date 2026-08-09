/** 从未知值中提取非空字符串数组（过滤掉非字符串与空白项）。 */
export function stringArray(value: unknown) {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string" && item.trim().length > 0) : [];
}
