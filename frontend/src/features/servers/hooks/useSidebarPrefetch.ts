import { useEffect, useRef } from "react";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useDmPreviewStore } from "../../dm/store/dmPreviewStore";
import type { Channel } from "../../../shared/types/channel";
import type { Server } from "../../../shared/types/server";

export function useSidebarPrefetch(params: {
  query: string;
  hasSession: boolean;
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  fetchChannels: (serverId: string) => unknown;
}) {
  const { query, hasSession, servers, channelsByServer, fetchChannels } = params;
  const friendsNextCursor = useFriendshipStore((s) => s.nextCursor);
  const loadMoreFriends = useFriendshipStore((s) => s.loadMore);
  const previewsNextCursor = useDmPreviewStore((s) => s.nextCursor);
  const loadMorePreviews = useDmPreviewStore((s) => s.loadMore);
  const prefetchInflightRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!hasSession) return;
    if (!query.trim()) return;
    const handle = window.setTimeout(() => {
      const inflight = prefetchInflightRef.current;
      servers.forEach((s) => {
        if (channelsByServer[s.id]) return;
        if (inflight.has(s.id)) return;
        inflight.add(s.id);
        Promise.resolve(fetchChannels(s.id)).finally(() => {
          inflight.delete(s.id);
        });
      });
    }, 250);
    return () => window.clearTimeout(handle);
  }, [query, hasSession, servers, channelsByServer, fetchChannels]);

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

  return {
    hasMoreDirects: !!friendsNextCursor || !!previewsNextCursor,
    loadMoreDirects: () => {
      void loadMoreFriends();
      void loadMorePreviews();
    },
  };
}
