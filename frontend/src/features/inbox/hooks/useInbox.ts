import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { useAuthStore } from "../../auth/store/authStore";
import { inboxAPI, type InboxPage } from "../api/inbox";
import { flattenInbox } from "../lib/inbox";

export function useInboxUnread(): number {
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const query = useQuery(
    {
      queryKey: queryKeys.inboxUnread(userId ?? ""),
      queryFn: () => inboxAPI.unread(),
      enabled: userId !== null,
      staleTime: 30_000,
    },
    queryClient,
  );
  return userId ? query.data ?? 0 : 0;
}

export function useInboxItems(open: boolean) {
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const query = useInfiniteQuery(
    {
      queryKey: queryKeys.inbox(userId ?? ""),
      queryFn: ({ pageParam }: { pageParam: string | null }) => inboxAPI.list(pageParam),
      initialPageParam: null as string | null,
      getNextPageParam: (last: InboxPage) => last.nextCursor ?? undefined,
      enabled: userId !== null && open,
      staleTime: 30_000,
    },
    queryClient,
  );
  return {
    userId,
    items: flattenInbox(query.data),
    loading: query.isPending && open,
    failed: query.isError,
    hasMore: !!query.hasNextPage,
    loadingMore: query.isFetchingNextPage,
    loadMore: () => void query.fetchNextPage(),
    retry: () => void query.refetch(),
  };
}
