import React from "react";

import { sharedAPI, sharedScopeKey, type SharedItem, type SharedKind, type SharedScope } from "../api/shared";

export const SHARED_PAGE_SIZE = 30;
export const SHARED_SEARCH_DEBOUNCE_MS = 250;

interface SharedState {
  items: SharedItem[];
  nextCursor: string | null;
  loading: boolean;
  failed: boolean;
  loaded: boolean;
}

const EMPTY: SharedState = { items: [], nextCursor: null, loading: false, failed: false, loaded: false };

export function useSharedContent(scope: SharedScope, kind: SharedKind, query: string, refreshKey: number) {
  const [state, setState] = React.useState<SharedState>(EMPTY);
  const requestRef = React.useRef(0);
  const scopeKey = sharedScopeKey(scope);
  const scopeRef = React.useRef(scope);
  scopeRef.current = scope;
  const [debouncedQuery, setDebouncedQuery] = React.useState(query.trim());

  React.useEffect(() => {
    const trimmed = query.trim();
    const timer = window.setTimeout(() => setDebouncedQuery(trimmed), SHARED_SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  const fetchPage = React.useCallback(
    async (before: string | null, append: boolean) => {
      const request = ++requestRef.current;
      setState((s) => ({ ...(append ? s : EMPTY), loading: true, failed: false, loaded: append ? s.loaded : false }));
      try {
        const page = await sharedAPI.list(scopeRef.current, kind, {
          q: debouncedQuery || undefined,
          before,
          limit: SHARED_PAGE_SIZE,
        });
        if (request !== requestRef.current) return;
        setState((s) => {
          const seen = new Set(append ? s.items.map(itemKey) : []);
          const fresh = page.items.filter((item) => !seen.has(itemKey(item)));
          return {
            items: append ? [...s.items, ...fresh] : page.items,
            nextCursor: page.nextCursor,
            loading: false,
            failed: false,
            loaded: true,
          };
        });
      } catch {
        if (request !== requestRef.current) return;
        setState((s) => ({ ...s, loading: false, failed: true }));
      }
    },
    [kind, debouncedQuery],
  );

  React.useEffect(() => {
    void fetchPage(null, false);
  }, [fetchPage, scopeKey, refreshKey]);

  React.useEffect(
    () => () => {
      requestRef.current += 1;
    },
    [],
  );

  const loadMore = React.useCallback(() => {
    if (state.loading || !state.nextCursor || state.failed) return;
    void fetchPage(state.nextCursor, true);
  }, [fetchPage, state.loading, state.nextCursor, state.failed]);

  const retry = React.useCallback(() => {
    if (state.items.length > 0 && state.nextCursor) void fetchPage(state.nextCursor, true);
    else void fetchPage(null, false);
  }, [fetchPage, state.items.length, state.nextCursor]);

  return { ...state, searching: query.trim() !== debouncedQuery, loadMore, retry };
}

export const itemKey = (item: SharedItem): string =>
  `${item.messageId}:${item.attachment?.id ?? item.url ?? ""}`;
