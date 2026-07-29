import * as React from "react";

export interface PageQueryState<T> {
  draftQuery: T;
  setDraftQuery: React.Dispatch<React.SetStateAction<T>>;
  appliedQuery: T;
  setAppliedQuery: React.Dispatch<React.SetStateAction<T>>;
  /** 归一化后同时写入草稿与已应用查询（用于 onApplyQueryState / 查询按钮）。 */
  applyQuery: (next: T) => void;
  /** 恢复默认查询并同时清空草稿与已应用查询（用于 onReset / onClearQueryState）。 */
  resetQuery: () => void;
}

/**
 * 页面级「草稿 / 已应用」查询对。
 * QueryPanel 编辑 draftQuery，点击查询后落到 appliedQuery；
 * normalize 在 applyQuery 时统一执行，页面无需在每个回调里手动归一化。
 */
export function usePageQueryState<T>(
  makeDefault: () => T,
  normalize?: (value: T) => T,
): PageQueryState<T> {
  const [draftQuery, setDraftQuery] = React.useState<T>(makeDefault);
  const [appliedQuery, setAppliedQuery] = React.useState<T>(makeDefault);
  const makeDefaultRef = React.useRef(makeDefault);
  makeDefaultRef.current = makeDefault;
  const normalizeRef = React.useRef(normalize);
  normalizeRef.current = normalize;

  const applyQuery = React.useCallback((next: T) => {
    const normalized = normalizeRef.current ? normalizeRef.current(next) : next;
    setDraftQuery(normalized);
    setAppliedQuery(normalized);
  }, []);

  const resetQuery = React.useCallback(() => {
    const next = makeDefaultRef.current();
    setDraftQuery(next);
    setAppliedQuery(next);
  }, []);

  return { draftQuery, setDraftQuery, appliedQuery, setAppliedQuery, applyQuery, resetQuery };
}
