import React from "react";

import { ws } from "../../../shared/ws/websocket";
import { pinsAPI, type PinnedMessage } from "../api/pins";
import { sharedScopeKey, type SharedScope } from "../api/shared";
import { useWsChannelMessages, useWsDmMessages } from "./useWebSocketMessages";

export const PIN_REFRESH_DEBOUNCE_MS = 250;

export const pinEventMatches = (
  data: { channelId?: unknown; participants?: unknown },
  scope: SharedScope,
  currentUserId?: string,
): boolean => {
  if (scope.kind === "channel") return data.channelId === scope.channelId;
  if (!Array.isArray(data.participants) || !currentUserId) return false;
  return data.participants.includes(scope.friendId) && data.participants.includes(currentUserId);
};

export function usePinnedMessages(scope: SharedScope | null, currentUserId?: string) {
  const key = scope ? sharedScopeKey(scope) : null;
  const scopeRef = React.useRef(scope);
  scopeRef.current = scope;
  const [state, setState] = React.useState<{ key: string | null; items: PinnedMessage[] }>({ key: null, items: [] });
  const pins = React.useMemo(() => (state.key === key ? state.items : []), [state, key]);
  const genRef = React.useRef(0);
  const requestRef = React.useRef(0);
  const inflightRef = React.useRef(0);
  const droppedRef = React.useRef(new Set<string>());
  const timerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  const refresh = React.useCallback(async () => {
    const current = scopeRef.current;
    if (!current) return;
    const requestKey = sharedScopeKey(current);
    const gen = genRef.current;
    const request = ++requestRef.current;
    inflightRef.current += 1;
    try {
      const items = await pinsAPI.list(current);
      if (gen !== genRef.current || request !== requestRef.current) return;
      if (sharedScopeKey(scopeRef.current ?? current) !== requestKey) return;
      setState({ key: requestKey, items: items.filter((p) => !droppedRef.current.has(p.messageId)) });
    } catch (e) {
      console.warn("[pins] load failed:", e);
    } finally {
      inflightRef.current -= 1;
    }
  }, []);

  const scheduleRefresh = React.useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      void refresh();
    }, PIN_REFRESH_DEBOUNCE_MS);
  }, [refresh]);

  React.useEffect(() => {
    genRef.current += 1;
    droppedRef.current = new Set();
    setState({ key, items: [] });
    if (key) void refresh();
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = null;
    };
  }, [key, refresh]);

  const drop = React.useCallback((messageId: string) => {
    droppedRef.current.add(messageId);
    setState((prev) =>
      prev.items.some((p) => p.messageId === messageId)
        ? { key: prev.key, items: prev.items.filter((p) => p.messageId !== messageId) }
        : prev,
    );
  }, []);

  const pinsRef = React.useRef(pins);
  pinsRef.current = pins;
  const handlers = {
    onEdited: (m: { id: string }) => {
      if (inflightRef.current > 0 || pinsRef.current.some((p) => p.messageId === m.id)) scheduleRefresh();
    },
    onDeleted: (m: { id: string }) => drop(m.id),
  };
  useWsChannelMessages(scope?.kind === "channel" ? scope.channelId : null, handlers);
  useWsDmMessages(scope?.kind === "dm" ? scope.friendId : null, currentUserId, handlers);

  React.useEffect(() => {
    if (!key) return;
    return ws.onOpen(() => {
      void refresh();
    });
  }, [key, refresh]);

  React.useEffect(() => {
    if (!key) return;
    return ws.on((env) => {
      const current = scopeRef.current;
      if (!current) return;
      const data = env.data as { messageId?: unknown; channelId?: unknown; participants?: unknown };
      if (env.type === "message.pin_changed" && pinEventMatches(data, current, currentUserId)) {
        scheduleRefresh();
      } else if (env.type === "message.hidden" && typeof data.messageId === "string") {
        drop(data.messageId);
      }
    });
  }, [key, currentUserId, scheduleRefresh, drop]);

  const setPinned = React.useCallback(
    async (messageId: string, pinned: boolean) => {
      const current = scopeRef.current;
      if (!current) return;
      if (pinned) {
        droppedRef.current.delete(messageId);
        await pinsAPI.pin(current, messageId);
      } else {
        await pinsAPI.unpin(current, messageId);
        drop(messageId);
      }
      scheduleRefresh();
    },
    [drop, scheduleRefresh],
  );

  const pinnedIds = React.useMemo(() => new Set(pins.map((p) => p.messageId)), [pins]);
  return { pins, pinnedIds, setPinned, refresh };
}
