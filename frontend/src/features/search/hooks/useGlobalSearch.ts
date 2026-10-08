import React from "react";
import { useInfiniteQuery } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";

import {
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

export const globalHitKey = (hit: GlobalSearchHit): string =>
  `${hit.messageId}:${hit.attachment?.id ?? hit.position ?? ""}`;

export const normalizeGlobalQuery = (raw: string): string | null => {
  const trimmed = raw.trim();
  const length = Array.from(trimmed).length;
  return length >= GLOBAL_SEARCH_MIN_CHARS && length <= GLOBAL_SEARCH_MAX_CHARS ? trimmed : null;
};

type SearchPage = Awaited<ReturnType<typeof globalSearchAPI.search>>;

function useSearchSection(kind: GlobalSearchKind, searched: string | null) {
  const query = useInfiniteQuery(
    {
      queryKey: queryKeys.globalSearch(kind, searched ?? ""),
      queryFn: ({ pageParam }: { pageParam: string | null }) =>
        globalSearchAPI.search(searched as string, kind, {
          before: pageParam,
          limit: pageParam ? GLOBAL_SEARCH_PAGE : GLOBAL_SEARCH_PREVIEW,
        }),
      initialPageParam: null as string | null,
      getNextPageParam: (last: SearchPage) => last.nextCursor ?? undefined,
      enabled: searched !== null,
    },
    queryClient,
  );
  const section = React.useMemo<GlobalSearchSection>(() => {
    if (searched === null) return emptySection();
    const seen = new Set<string>();
    const items: GlobalSearchHit[] = [];
    for (const page of query.data?.pages ?? []) {
      for (const hit of page.items) {
        const key = globalHitKey(hit);
        if (seen.has(key)) continue;
        seen.add(key);
        items.push(hit);
      }
    }
    const pages = query.data?.pages;
    return {
      items,
      nextCursor: pages && pages.length > 0 ? pages[pages.length - 1].nextCursor : null,
      loading: query.isFetching,
      loaded: !!query.data,
      failed: query.isError || query.isFetchNextPageError,
    };
  }, [searched, query.data, query.isFetching, query.isError, query.isFetchNextPageError]);
  const { fetchNextPage, refetch, hasNextPage, isFetching, isFetchNextPageError } = query;
  const loadMore = React.useCallback(() => {
    if (searched === null || isFetching || !hasNextPage) return;
    void fetchNextPage();
  }, [searched, isFetching, hasNextPage, fetchNextPage]);
  const retry = React.useCallback(() => {
    if (searched === null || isFetching) return;
    if (isFetchNextPageError) void fetchNextPage();
    else void refetch();
  }, [searched, isFetching, isFetchNextPageError, fetchNextPage, refetch]);
  return { section, loadMore, retry };
}

export function useGlobalSearch(rawQuery: string) {
  const query = normalizeGlobalQuery(rawQuery);
  const [searched, setSearched] = React.useState<string | null>(null);

  React.useEffect(() => {
    setSearched(null);
    if (query === null) return;
    const timer = setTimeout(() => setSearched(query), GLOBAL_SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query]);

  const messages = useSearchSection("messages", searched);
  const files = useSearchSection("files", searched);
  const links = useSearchSection("links", searched);
  const byKind = React.useMemo(() => ({ messages, files, links }), [messages, files, links]);

  const sections: Sections = React.useMemo(
    () => ({ messages: messages.section, files: files.section, links: links.section }),
    [messages.section, files.section, links.section],
  );
  const loadMore = React.useCallback((kind: GlobalSearchKind) => byKind[kind].loadMore(), [byKind]);
  const retry = React.useCallback((kind: GlobalSearchKind) => byKind[kind].retry(), [byKind]);

  const settled = searched !== null && searched === query;
  return { active: query !== null, settled, sections, loadMore, retry };
}
