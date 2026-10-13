import { useQuery } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { useAuthStore } from "../../auth/store/authStore";
import { searchHistoryAPI, type HistoryItem, type HistoryTarget } from "../api/searchHistory";
import {
  clearSearches,
  HistorySessionChanged,
  installHistoryPersistence,
  readStoredHistory,
  recordSearch,
  removeSearch,
} from "../lib/searchHistory";

const quietly = (promise: Promise<void>) => {
  promise.catch((err) => {
    if (err instanceof HistorySessionChanged) return;
    console.warn("[search] history update failed:", err);
  });
};

export function useSearchHistory(load: boolean) {
  installHistoryPersistence();
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const query = useQuery(
    {
      queryKey: queryKeys.searchHistory(userId ?? ""),
      queryFn: () => searchHistoryAPI.list(),
      enabled: userId !== null && load,
      initialData: () => (userId ? readStoredHistory(userId) : undefined),
      initialDataUpdatedAt: 0,
      staleTime: 30_000,
    },
    queryClient,
  );
  return {
    items: (userId ? query.data : undefined) ?? ([] as HistoryItem[]),
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
