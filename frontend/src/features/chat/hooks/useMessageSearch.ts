import React from "react";
import { ApiError } from "../../../shared/lib/apiError";
import type { ChannelMessage } from "../../../shared/types/channel";
import type { Messenger } from "./useMessenger";

const SEARCH_PAGE_SIZE = 50;

export function useMessageSearch(params: {
  messenger: Messenger | null;
  convId?: string;
  nameSyncTick: number;
  onClose: () => void;
}) {
  const { messenger, convId, nameSyncTick, onClose } = params;
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [searchQuery, setSearchQuery] = React.useState("");
  const [searchInput, setSearchInput] = React.useState("");
  const [searchResults, setSearchResults] = React.useState<ChannelMessage[]>([]);
  const [searchLoading, setSearchLoading] = React.useState(false);
  const [searchError, setSearchError] = React.useState<string | null>(null);
  const [searchTotal, setSearchTotal] = React.useState(0);
  const messengerRef = React.useRef(messenger);
  messengerRef.current = messenger;
  const onCloseRef = React.useRef(onClose);
  onCloseRef.current = onClose;

  const clearResults = React.useCallback(() => {
    setSearchInput("");
    setSearchQuery("");
    setSearchResults([]);
    setSearchError(null);
    setSearchTotal(0);
  }, []);

  React.useEffect(() => {
    setSearchOpen(false);
    clearResults();
  }, [convId, clearResults]);

  React.useEffect(() => {
    const trimmed = searchInput.trim();
    if (!searchOpen) return;
    if (!messengerRef.current) return;
    if (trimmed.length === 0) {
      setSearchQuery("");
      setSearchResults([]);
      setSearchError(null);
      setSearchTotal(0);
      return;
    }
    const timeout = window.setTimeout(() => setSearchQuery(trimmed), 250);
    return () => window.clearTimeout(timeout);
  }, [searchInput, searchOpen, convId]);

  React.useEffect(() => {
    const messengerNow = messengerRef.current;
    if (!searchOpen || !messengerNow) return;
    const q = searchQuery.trim();
    if (q.length === 0) return;
    let cancelled = false;
    setSearchLoading(true);
    setSearchError(null);
    messengerNow
      .searchMessages(q, 1, SEARCH_PAGE_SIZE)
      .then((res) => {
        if (cancelled) return;
        setSearchResults(res.messages);
        setSearchTotal(res.pagination?.total ?? res.messages.length);
      })
      .catch((err) => {
        if (cancelled) return;
        const apiErr = err instanceof ApiError ? err : null;
        setSearchError(apiErr?.message || "Search failed.");
        setSearchResults([]);
        setSearchTotal(0);
      })
      .finally(() => {
        if (!cancelled) setSearchLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [searchQuery, searchOpen, convId, nameSyncTick]);

  const closeSearch = React.useCallback(() => {
    setSearchOpen(false);
    clearResults();
  }, [clearResults]);

  const toggleSearch = React.useCallback(() => {
    setSearchOpen((prev) => {
      const next = !prev;
      if (!next) {
        clearResults();
        onCloseRef.current();
      }
      return next;
    });
  }, [clearResults]);

  React.useEffect(() => {
    if (!searchOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        toggleSearch();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [searchOpen, toggleSearch]);

  return {
    searchOpen,
    searchQuery,
    searchInput,
    setSearchInput,
    searchResults,
    searchLoading,
    searchError,
    searchTotal,
    clearResults,
    closeSearch,
    toggleSearch,
  };
}
