import type { InfiniteData } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import type { InboxItem, InboxPage } from "../api/inbox";

export type InboxPages = InfiniteData<InboxPage, string | null>;

export function refreshInbox(): void {
  void queryClient.invalidateQueries({ queryKey: ["inbox"] });
  void queryClient.invalidateQueries({ queryKey: ["inboxUnread"] });
}

export const flattenInbox = (pages: { pages: InboxPage[] } | undefined): InboxItem[] =>
  (pages?.pages ?? []).flatMap((page) => page.items);

const mapItems = (pages: InboxPages | undefined, map: (items: InboxItem[]) => InboxItem[]): InboxPages | undefined =>
  pages && { ...pages, pages: pages.pages.map((page) => ({ ...page, items: map(page.items) })) };

export function markAllReadLocally(userId: string): () => void {
  const listKey = queryKeys.inbox(userId);
  const unreadKey = queryKeys.inboxUnread(userId);
  const previousList = queryClient.getQueryData<InboxPages>(listKey);
  const previousUnread = queryClient.getQueryData<number>(unreadKey);
  queryClient.setQueryData<InboxPages | undefined>(listKey, (pages: InboxPages | undefined) =>
    mapItems(pages, (items) => items.map((item) => (item.read ? item : { ...item, read: true }))),
  );
  queryClient.setQueryData<number>(unreadKey, 0);
  return () => {
    queryClient.setQueryData(listKey, previousList);
    queryClient.setQueryData(unreadKey, previousUnread);
  };
}

export function removeLocally(userId: string, id: string): () => void {
  const listKey = queryKeys.inbox(userId);
  const unreadKey = queryKeys.inboxUnread(userId);
  const previousList = queryClient.getQueryData<InboxPages>(listKey);
  const previousUnread = queryClient.getQueryData<number>(unreadKey);
  const removed = flattenInbox(previousList).find((item) => item.id === id);
  queryClient.setQueryData<InboxPages | undefined>(listKey, (pages: InboxPages | undefined) =>
    mapItems(pages, (items) => items.filter((item) => item.id !== id)),
  );
  if (removed && !removed.read && typeof previousUnread === "number") {
    queryClient.setQueryData<number>(unreadKey, Math.max(0, previousUnread - 1));
  }
  return () => {
    queryClient.setQueryData(listKey, previousList);
    queryClient.setQueryData(unreadKey, previousUnread);
  };
}
