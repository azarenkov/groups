import React from "react";
import { useQuery } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { useAuthStore } from "../../auth/store/authStore";
import { useBlockStore } from "../../social/store/blockStore";
import { type HistoryItem, type HistoryTarget } from "../api/searchHistory";
import {
  clearSearches,
  HistorySessionChanged,
  installHistoryPersistence,
  readStoredHistory,
  recordSearch,
  refreshSearchHistory,
  fetchSearchHistory,
  historyVerified,
  isPendingHistoryId,
  removeSearch,
} from "../lib/searchHistory";

const quietly = (promise: Promise<void>) => {
  promise.catch((err) => {
    if (err instanceof HistorySessionChanged) return;
    console.warn("[search] history update failed:", err);
  });
};

export const blockedPeerOf = (item: HistoryItem): string | null => {
  switch (item.type) {
    case "person":
    case "dm":
      return item.userId;
    case "message":
      return item.place.type === "dm" ? item.place.peerId : null;
    default:
      return null;
  }
};

export function useSearchHistory(load: boolean) {
  installHistoryPersistence();
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const query = useQuery(
    {
      queryKey: queryKeys.searchHistory(userId ?? ""),
      queryFn: () => fetchSearchHistory(userId ?? ""),
      enabled: userId !== null && load,
      initialData: () => (userId ? readStoredHistory(userId) : undefined),
      initialDataUpdatedAt: 0,
      staleTime: 30_000,
    },
    queryClient,
  );
  const blocked = useBlockStore((s) => s.blocked);
  const blockedBy = useBlockStore((s) => s.blockedBy);
  const seenBlocks = React.useRef({ blocked, blockedBy });
  React.useEffect(() => {
    if (seenBlocks.current.blocked === blocked && seenBlocks.current.blockedBy === blockedBy) return;
    seenBlocks.current = { blocked, blockedBy };
    refreshSearchHistory();
  }, [blocked, blockedBy]);
  const items = React.useMemo(
    () =>
      ((userId ? query.data : undefined) ?? []).filter((item) => {
        if (item.type !== "query" && !isPendingHistoryId(item.id) && !(userId && historyVerified(userId))) {
          return false;
        }
        const peer = blockedPeerOf(item);
        return !peer || (!blocked[peer] && !blockedBy[peer]);
      }),
    [userId, query.data, blocked, blockedBy],
  );
  return {
    items,
    record: (target: HistoryTarget) => {
      if (userId) quietly(recordSearch(userId, target));
    },
    remove: (id: string) => {
      if (userId) quietly(removeSearch(userId, id));
    },
    clear: () => {
      if (userId) quietly(clearSearches(userId));
    },
  };
}
