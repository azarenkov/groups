import React from "react";

import {
  GLOBAL_SEARCH_KINDS,
  globalSearchAPI,
  type GlobalSearchHit,
  type GlobalSearchKind,
} from "../api/globalSearch";

export const GLOBAL_SEARCH_MIN_CHARS = 2;
export const GLOBAL_SEARCH_MAX_CHARS = 100;
export const GLOBAL_SEARCH_DEBOUNCE_MS = 300;
export const GLOBAL_SEARCH_PREVIEW = 5;
export const GLOBAL_SEARCH_PAGE = 20;

export interface GlobalSearchSection {
  items: GlobalSearchHit[];
  nextCursor: string | null;
  loading: boolean;
  loaded: boolean;
  failed: boolean;
}

type Sections = Record<GlobalSearchKind, GlobalSearchSection>;

const emptySection = (): GlobalSearchSection => ({
  items: [],
  nextCursor: null,
  loading: false,
  loaded: false,
  failed: false,
});

const emptySections = (): Sections => ({
  messages: emptySection(),
  files: emptySection(),
  links: emptySection(),
});

export const globalHitKey = (hit: GlobalSearchHit): string =>
  `${hit.messageId}:${hit.attachment?.id ?? hit.position ?? ""}`;

export const normalizeGlobalQuery = (raw: string): string | null => {
  const trimmed = raw.trim();
  const length = Array.from(trimmed).length;
  return length >= GLOBAL_SEARCH_MIN_CHARS && length <= GLOBAL_SEARCH_MAX_CHARS ? trimmed : null;
};

export function useGlobalSearch(rawQuery: string) {
  const query = normalizeGlobalQuery(rawQuery);
  const [debounced, setDebounced] = React.useState<string | null>(null);
  const [sections, setSections] = React.useState<Sections>(emptySections);
  const genRef = React.useRef(0);
  const sectionsRef = React.useRef(sections);
  sectionsRef.current = sections;

  React.useEffect(() => {
    if (query === null) {
      setDebounced(null);
      return;
    }
    const timer = setTimeout(() => setDebounced(query), GLOBAL_SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query]);

  const patch = React.useCallback((kind: GlobalSearchKind, next: Partial<GlobalSearchSection>) => {
    setSections((prev) => ({ ...prev, [kind]: { ...prev[kind], ...next } }));
  }, []);

  const fetchPage = React.useCallback(
    async (q: string, kind: GlobalSearchKind, before: string | null) => {
      const gen = genRef.current;
      patch(kind, { loading: true, failed: false });
      try {
        const page = await globalSearchAPI.search(q, kind, {
          before,
          limit: before ? GLOBAL_SEARCH_PAGE : GLOBAL_SEARCH_PREVIEW,
        });
        if (gen !== genRef.current) return;
        setSections((prev) => {
          const current = prev[kind];
          const base = before ? current.items : [];
          const seen = new Set(base.map(globalHitKey));
          const items = [...base, ...page.items.filter((hit) => !seen.has(globalHitKey(hit)))];
          return {
            ...prev,
            [kind]: { items, nextCursor: page.nextCursor, loading: false, loaded: true, failed: false },
          };
        });
      } catch {
        if (gen !== genRef.current) return;
        patch(kind, { loading: false, failed: true });
      }
    },
    [patch],
  );

  React.useEffect(() => {
    genRef.current += 1;
    setSections(emptySections());
    if (debounced === null) return;
    for (const kind of GLOBAL_SEARCH_KINDS) void fetchPage(debounced, kind, null);
  }, [debounced, fetchPage]);

  const loadMore = React.useCallback(
    (kind: GlobalSearchKind) => {
      const section = sectionsRef.current[kind];
      if (debounced === null || section.loading || !section.nextCursor) return;
      void fetchPage(debounced, kind, section.nextCursor);
    },
    [debounced, fetchPage],
  );

  const retry = React.useCallback(
    (kind: GlobalSearchKind) => {
      const section = sectionsRef.current[kind];
      if (debounced === null || section.loading) return;
      void fetchPage(debounced, kind, section.loaded ? section.nextCursor : null);
    },
    [debounced, fetchPage],
  );

  const settled = debounced !== null && debounced === query;
  return { active: query !== null, settled, sections, loadMore, retry };
}
