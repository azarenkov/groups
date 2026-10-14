import React from "react";
import { useQuery } from "@tanstack/react-query";

import { queryClient, queryKeys, refetchFresh } from "../../../shared/query/queryClient";
import { ws } from "../../../shared/ws/websocket";
import { pinsAPI, type PinnedMessage } from "../api/pins";
import { sharedScopeKey, type SharedScope } from "../api/shared";
import { useWsChannelMessages, useWsDmMessages } from "./useWebSocketMessages";
import { dmClearedFor } from "../../dm/lib/dmCleared";

export const pinEventMatches = (
  data: { channelId?: unknown; participants?: unknown },
  scope: SharedScope,
  currentUserId?: string,
): boolean => {
  if (scope.kind === "channel") return data.channelId === scope.channelId;
  if (!Array.isArray(data.participants) || !currentUserId) return false;
  return data.participants.includes(scope.friendId) && data.participants.includes(currentUserId);
};

const droppedByScope = new Map<string, Map<string, number>>();
let dropSeq = 0;

export function clearPinTombstones(): void {
  droppedByScope.clear();
}

const markDropped = (key: string, messageId: string): void => {
  let dropped = droppedByScope.get(key);
  if (!dropped) {
    dropped = new Map();
    droppedByScope.set(key, dropped);
  }
  dropSeq += 1;
  dropped.set(messageId, dropSeq);
};

const forgetDropped = (key: string, messageId: string): void => {
  droppedByScope.get(key)?.delete(messageId);
};

const fetchPins = async (scope: SharedScope, key: string): Promise<PinnedMessage[]> => {
  const startedAt = dropSeq;
  const items = await pinsAPI.list(scope);
  const dropped = droppedByScope.get(key);
  if (!dropped) return items;
  for (const [messageId, seq] of dropped) {
    if (seq <= startedAt) dropped.delete(messageId);
  }
  if (dropped.size === 0) {
    droppedByScope.delete(key);
    return items;
  }
  return items.filter((p) => !dropped.has(p.messageId));
};

const EMPTY: PinnedMessage[] = [];

export function usePinnedMessages(scope: SharedScope | null, currentUserId?: string) {
  const key = scope ? sharedScopeKey(scope) : null;
  const scopeRef = React.useRef(scope);
  scopeRef.current = scope;

  const query = useQuery(
    {
      queryKey: queryKeys.pins(key ?? ""),
      queryFn: () => fetchPins(scope as SharedScope, key as string),
      enabled: !!key,
    },
    queryClient,
  );
  const pins = (key && query.data) || EMPTY;

  const invalidate = React.useCallback(async () => {
    if (!key) return;
    await refetchFresh(queryKeys.pins(key));
  }, [key]);

  const drop = React.useCallback(
    (messageId: string) => {
      if (!key) return;
      markDropped(key, messageId);
      queryClient.setQueryData<PinnedMessage[]>(queryKeys.pins(key), (prev: PinnedMessage[] | undefined) =>
        prev && prev.some((p) => p.messageId === messageId) ? prev.filter((p) => p.messageId !== messageId) : prev,
      );
    },
    [key],
  );

  const pinsRef = React.useRef(pins);
  pinsRef.current = pins;
  const handlers = {
    onEdited: (m: { id: string }) => {
      if (!key) return;
      const fetching = queryClient.isFetching({ queryKey: queryKeys.pins(key) }) > 0;
      if (fetching || pinsRef.current.some((p) => p.messageId === m.id)) void invalidate();
    },
    onDeleted: (m: { id: string }) => drop(m.id),
  };
  useWsChannelMessages(scope?.kind === "channel" ? scope.channelId : null, handlers);
  useWsDmMessages(scope?.kind === "dm" ? scope.friendId : null, currentUserId, handlers);

  React.useEffect(() => {
    if (!key) return;
    return ws.on((env) => {
      const current = scopeRef.current;
      if (!current) return;
      const data = env.data as { messageId?: unknown; channelId?: unknown; participants?: unknown; pinned?: unknown };
      if (env.type === "message.pin_changed" && pinEventMatches(data, current, currentUserId)) {
        if (data.pinned === true && typeof data.messageId === "string" && key) {
          forgetDropped(key, data.messageId);
        }
        void invalidate();
      } else if (env.type === "message.hidden" && typeof data.messageId === "string") {
        drop(data.messageId);
      } else if (
        env.type === "dm_conversation.cleared" &&
        current.kind === "dm" &&
        dmClearedFor(env.data, currentUserId)?.peerId === current.friendId
      ) {
        void invalidate();
      }
    });
  }, [key, currentUserId, invalidate, drop]);

  const setPinned = React.useCallback(
    async (messageId: string, pinned: boolean) => {
      const current = scopeRef.current;
      if (!current) return;
      const scopeKey = sharedScopeKey(current);
      if (pinned) {
        forgetDropped(scopeKey, messageId);
        await pinsAPI.pin(current, messageId);
      } else {
        await pinsAPI.unpin(current, messageId);
        markDropped(scopeKey, messageId);
        queryClient.setQueryData<PinnedMessage[]>(queryKeys.pins(scopeKey), (prev: PinnedMessage[] | undefined) =>
          prev ? prev.filter((p) => p.messageId !== messageId) : prev,
        );
      }
      await refetchFresh(queryKeys.pins(scopeKey));
    },
    [],
  );

  const pinnedIds = React.useMemo(() => new Set(pins.map((p) => p.messageId)), [pins]);
  return { pins, pinnedIds, setPinned, refresh: invalidate };
}
