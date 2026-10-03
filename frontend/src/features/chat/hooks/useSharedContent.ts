import React from "react";

import { URL_REGEX } from "../../../shared/lib/links";
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

export const itemKey = (item: SharedItem): string =>
  `${item.messageId}:${item.attachment?.id ?? item.url ?? ""}`;

const olderThan = (item: SharedItem, edge: SharedItem): boolean =>
  item.createdAt < edge.createdAt || (item.createdAt === edge.createdAt && item.messageId < edge.messageId);

export const mergeFirstPage = (current: SharedItem[], page: SharedItem[], pageComplete: boolean): SharedItem[] => {
  if (pageComplete || page.length === 0) return page;
  const edge = page[page.length - 1];
  const keys = new Set(page.map(itemKey));
  return [
    ...page,
    ...current.filter(
      (item) => !keys.has(itemKey(item)) && (olderThan(item, edge) || item.messageId === edge.messageId),
    ),
  ];
};

export interface MessageEdit {
  messageId: string;
  attachmentIds?: string[];
  content?: string;
}

export const matchingLinks = (content: string, query: string): string[] => {
  const needle = query.trim().toLowerCase();
  return (content.match(URL_REGEX) ?? []).filter((url) => !needle || url.toLowerCase().includes(needle)).reverse();
};

export const insertMessageLinks = (
  items: SharedItem[],
  message: { messageId: string; authorId: string; authorName: string; createdAt: string; urls: string[] },
  complete: boolean,
): SharedItem[] => {
  if (message.urls.length === 0 || items.some((item) => item.messageId === message.messageId)) return items;
  const entries = message.urls.map((url) => ({
    messageId: message.messageId,
    authorId: message.authorId,
    authorName: message.authorName,
    createdAt: message.createdAt,
    url,
  }));
  const anchor: SharedItem = entries[0];
  const at = items.findIndex((item) => olderThan(item, anchor));
  if (at < 0) return complete ? [...items, ...entries] : items;
  return [...items.slice(0, at), ...entries, ...items.slice(at)];
};

export const reconcileEdit = (items: SharedItem[], edit: MessageEdit, kind: SharedKind, query: string): SharedItem[] => {
  const first = items.findIndex((item) => item.messageId === edit.messageId);
  if (first < 0) return items;
  if (kind !== "links") {
    if (!edit.attachmentIds) return items;
    const kept = new Set(edit.attachmentIds);
    return items.filter((item) => item.messageId !== edit.messageId || (!!item.attachment && kept.has(item.attachment.id)));
  }
  if (edit.content === undefined) return items;
  const template = items[first];
  const urls = matchingLinks(edit.content, query);
  const replaced = urls.map((url) => ({ ...template, attachment: undefined, url }));
  const rest = items.filter((item) => item.messageId !== edit.messageId);
  return [...rest.slice(0, first), ...replaced, ...rest.slice(first)];
};

export function useSharedContent(scope: SharedScope, kind: SharedKind, query: string) {
  const [state, setState] = React.useState<SharedState>(EMPTY);
  const requestRef = React.useRef(0);
  const failedRef = React.useRef<{ before: string | null; mode: "reset" | "append" | "merge" } | null>(null);
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
    async (before: string | null, mode: "reset" | "append" | "merge") => {
      const request = ++requestRef.current;
      failedRef.current = null;
      setState((s) => (mode === "reset" ? { ...EMPTY, loading: true } : { ...s, loading: true, failed: false }));
      try {
        const page = await sharedAPI.list(scopeRef.current, kind, {
          q: debouncedQuery || undefined,
          before,
          limit: SHARED_PAGE_SIZE,
        });
        if (request !== requestRef.current) return;
        setState((s) => {
          if (mode === "append") {
            const seen = new Set(s.items.map(itemKey));
            return {
              items: [...s.items, ...page.items.filter((item) => !seen.has(itemKey(item)))],
              nextCursor: page.nextCursor,
              loading: false,
              failed: false,
              loaded: true,
            };
          }
          if (mode === "merge") {
            const items = mergeFirstPage(s.items, page.items, page.nextCursor === null);
            const extended = items.length > page.items.length;
            return {
              items,
              nextCursor: extended ? s.nextCursor : page.nextCursor,
              loading: false,
              failed: false,
              loaded: true,
            };
          }
          return { items: page.items, nextCursor: page.nextCursor, loading: false, failed: false, loaded: true };
        });
      } catch {
        if (request !== requestRef.current) return;
        failedRef.current = { before, mode };
        setState((s) => ({ ...s, loading: false, failed: true }));
      }
    },
    [kind, debouncedQuery],
  );

  React.useEffect(() => {
    void fetchPage(null, "reset");
  }, [fetchPage, scopeKey]);

  React.useEffect(
    () => () => {
      requestRef.current += 1;
    },
    [],
  );

  const loadMore = React.useCallback(() => {
    if (state.loading || !state.nextCursor || state.failed) return;
    void fetchPage(state.nextCursor, "append");
  }, [fetchPage, state.loading, state.nextCursor, state.failed]);

  const retry = React.useCallback(() => {
    const failed = failedRef.current ?? { before: null, mode: "reset" as const };
    void fetchPage(failed.before, failed.mode);
  }, [fetchPage]);

  const refreshTop = React.useCallback(() => {
    void fetchPage(null, "merge");
  }, [fetchPage]);

  const dropMessage = React.useCallback((messageId: string) => {
    setState((s) => ({ ...s, items: s.items.filter((item) => item.messageId !== messageId) }));
  }, []);

  const itemsRef = React.useRef(state.items);
  itemsRef.current = state.items;
  const viewRef = React.useRef("");
  viewRef.current = `${scopeKey}|${kind}|${debouncedQuery}`;

  const applyEdit = React.useCallback(
    (edit: MessageEdit) => {
      const known = itemsRef.current.some((item) => item.messageId === edit.messageId);
      if (known || kind !== "links" || !edit.content || matchingLinks(edit.content, debouncedQuery).length === 0) {
        setState((s) => ({ ...s, items: reconcileEdit(s.items, edit, kind, debouncedQuery) }));
        return;
      }
      const viewAtEdit = viewRef.current;
      sharedAPI
        .message(scopeRef.current, edit.messageId)
        .then((message) => {
          if (viewRef.current !== viewAtEdit) return;
          setState((s) => ({
            ...s,
            items: insertMessageLinks(
              s.items,
              {
                messageId: message.id,
                authorId: message.authorId,
                authorName: message.authorName ?? "",
                createdAt: message.createdAt,
                urls: matchingLinks(message.content, debouncedQuery),
              },
              s.nextCursor === null,
            ),
          }));
        })
        .catch(() => {});
    },
    [kind, debouncedQuery],
  );

  return { ...state, searching: query.trim() !== debouncedQuery, loadMore, retry, refreshTop, dropMessage, applyEdit };
}
