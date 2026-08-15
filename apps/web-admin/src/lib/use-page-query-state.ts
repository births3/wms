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

export interface PageQueryStateOptions<T> {
  normalize?: (value: T) => T;
  syncWithUrl?: boolean;
  urlPrefix?: string;
}

export function usePageQueryState<T extends Record<string, unknown>>(
  makeDefault: () => T,
  optionsOrNormalize?: ((value: T) => T) | PageQueryStateOptions<T>,
): PageQueryState<T> {
  const options: PageQueryStateOptions<T> =
    typeof optionsOrNormalize === "function"
      ? { normalize: optionsOrNormalize }
      : optionsOrNormalize ?? {};

  const { normalize, syncWithUrl = false } = options;

  // 初始值：如果开启 URL 同步，优先尝试从 search params 读取
  const initialValue = React.useMemo(() => {
    const fallback = makeDefault();
    if (!syncWithUrl || typeof window === "undefined") return fallback;
    try {
      const params = new URLSearchParams(window.location.search);
      const restored = { ...fallback };
      let hasUrlParams = false;
      for (const [key, value] of params.entries()) {
        if (key in fallback) {
          hasUrlParams = true;
          // 支持 JSON 或基础字符串
          try {
            (restored as Record<string, unknown>)[key] = JSON.parse(value);
          } catch {
            (restored as Record<string, unknown>)[key] = value;
          }
        }
      }
      return hasUrlParams ? (normalize ? normalize(restored) : restored) : fallback;
    } catch {
      return fallback;
    }
  }, [syncWithUrl]);

  const [draftQuery, setDraftQuery] = React.useState<T>(initialValue);
  const [appliedQuery, setAppliedQuery] = React.useState<T>(initialValue);
  const makeDefaultRef = React.useRef(makeDefault);
  makeDefaultRef.current = makeDefault;
  const normalizeRef = React.useRef(normalize);
  normalizeRef.current = normalize;

  const updateUrlParams = React.useCallback(
    (query: T) => {
      if (!syncWithUrl || typeof window === "undefined") return;
      try {
        const url = new URL(window.location.href);
        const defaults = makeDefaultRef.current();
        for (const [key, val] of Object.entries(query)) {
          if (val === undefined || val === null || val === "" || JSON.stringify(val) === JSON.stringify(defaults[key])) {
            url.searchParams.delete(key);
          } else {
            url.searchParams.set(key, typeof val === "object" ? JSON.stringify(val) : String(val));
          }
        }
        window.history.replaceState(null, "", url.toString());
      } catch {
        // 静默捕获
      }
    },
    [syncWithUrl],
  );

  const applyQuery = React.useCallback(
    (next: T) => {
      const normalized = normalizeRef.current ? normalizeRef.current(next) : next;
      setDraftQuery(normalized);
      setAppliedQuery(normalized);
      updateUrlParams(normalized);
    },
    [updateUrlParams],
  );

  const resetQuery = React.useCallback(() => {
    const next = makeDefaultRef.current();
    setDraftQuery(next);
    setAppliedQuery(next);
    updateUrlParams(next);
  }, [updateUrlParams]);

  return { draftQuery, setDraftQuery, appliedQuery, setAppliedQuery, applyQuery, resetQuery };
}

