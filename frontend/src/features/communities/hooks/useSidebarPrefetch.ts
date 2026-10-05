import { useEffect, useRef, useState } from "react";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useDmPreviewStore } from "../../dm/store/dmPreviewStore";
import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";
import { useConversationPinsStore } from "../store/conversationPinsStore";

export const PINNED_DM_RETRY_MS = 2000;
export const PINNED_DM_MAX_RETRIES = 5;

export function useSidebarPrefetch(params: {
  query: string;
  hasSession: boolean;
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  fetchChannels: (communityId: string) => unknown;
}) {
  const { query, hasSession, communities, channelsByCommunity, fetchChannels } = params;
  const friendsNextCursor = useFriendshipStore((s) => s.nextCursor);
  const loadMoreFriends = useFriendshipStore((s) => s.loadMore);
  const previewsNextCursor = useDmPreviewStore((s) => s.nextCursor);
  const loadMorePreviews = useDmPreviewStore((s) => s.loadMore);
  const prefetchInflightRef = useRef<Set<string>>(new Set());
  const pinnedDms = useConversationPinsStore((s) => s.dms);
  const friends = useFriendshipStore((s) => s.friends);
  const previews = useDmPreviewStore((s) => s.byFriend);
  const missingPinnedDm =
    hasSession &&
    pinnedDms.some((id) => !previews[id] && !friends.some((f) => f.friendId === id && f.status === "accepted"));

  useEffect(() => {
    if (!hasSession) return;
    if (!query.trim()) return;
    const handle = window.setTimeout(() => {
      const inflight = prefetchInflightRef.current;
      communities.forEach((s) => {
        if (channelsByCommunity[s.id]) return;
        if (inflight.has(s.id)) return;
        inflight.add(s.id);
        Promise.resolve(fetchChannels(s.id)).finally(() => {
          inflight.delete(s.id);
        });
      });
    }, 250);
    return () => window.clearTimeout(handle);
  }, [query, hasSession, communities, channelsByCommunity, fetchChannels]);

  useEffect(() => {
    if (!hasSession) return;
    if (!query.trim()) return;
    if (friendsNextCursor) void loadMoreFriends();
    if (previewsNextCursor) void loadMorePreviews();
  }, [
    query,
    hasSession,
    friendsNextCursor,
    previewsNextCursor,
    loadMoreFriends,
    loadMorePreviews,
  ]);

  const [pinRetry, setPinRetry] = useState(0);
  useEffect(() => {
    if (!missingPinnedDm) setPinRetry(0);
  }, [missingPinnedDm]);
  useEffect(() => {
    if (!missingPinnedDm || (!friendsNextCursor && !previewsNextCursor)) return;
    if (friendsNextCursor) void loadMoreFriends();
    if (previewsNextCursor) void loadMorePreviews();
    if (pinRetry >= PINNED_DM_MAX_RETRIES) return;
    const timer = window.setTimeout(
      () => setPinRetry((n) => n + 1),
      PINNED_DM_RETRY_MS * 2 ** pinRetry,
    );
    return () => window.clearTimeout(timer);
  }, [missingPinnedDm, friendsNextCursor, previewsNextCursor, loadMoreFriends, loadMorePreviews, pinRetry]);

  return {
    hasMoreDirects: !!friendsNextCursor || !!previewsNextCursor,
    loadMoreDirects: () => {
      void loadMoreFriends();
      void loadMorePreviews();
    },
  };
}
