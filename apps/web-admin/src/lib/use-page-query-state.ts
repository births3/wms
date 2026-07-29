import * as React from "react";

export function usePageQueryState<T>(makeDefault: () => T, normalize?: (value: T) => T) {
  const [draftQuery, setDraftQuery] = React.useState<T>(makeDefault);
  const [appliedQuery, setAppliedQuery] = React.useState<T>(makeDefault);
  const makeDefaultRef = React.useRef(makeDefault);
  const normalizeRef = React.useRef(normalize);
  makeDefaultRef.current = makeDefault;
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
