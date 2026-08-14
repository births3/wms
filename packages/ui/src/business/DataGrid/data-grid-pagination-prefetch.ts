export interface DataGridPrefetchPageIndexesInput {
  pageIndex: number;
  pageSize: number;
  total: number;
  prefetchCount: number;
}

export function getDataGridPrefetchPageIndexes({
  pageIndex,
  pageSize,
  total,
  prefetchCount,
}: DataGridPrefetchPageIndexesInput): number[] {
  if (pageSize <= 0 || total <= 0 || prefetchCount <= 0) return [];

  const pageTotal = Math.max(1, Math.ceil(total / pageSize));
  const currentPageIndex = Math.min(Math.max(Math.trunc(pageIndex), 0), pageTotal - 1);
  const count = Math.trunc(prefetchCount);
  const pageIndexes: number[] = [];

  for (let offset = 1; offset <= count; offset += 1) {
    const nextPageIndex = currentPageIndex + offset;
    if (nextPageIndex >= pageTotal) break;
    pageIndexes.push(nextPageIndex);
  }

  return pageIndexes;
}
