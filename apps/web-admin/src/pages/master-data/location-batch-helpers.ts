export function numberInputValue(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function uniqueSorted(values: number[]) {
  return Array.from(new Set(values)).sort((left, right) => left - right);
}

export function locationBatchGroupKey(group: { rowNo: number; columnNo: number }) {
  return `${group.rowNo}:${group.columnNo}`;
}

export function pad2(value: number) {
  return String(value).padStart(2, "0");
}
