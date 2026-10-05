import React from "react";
import i18n from "../../../i18n";
import { ApiError } from "../../../shared/lib/apiError";
import type { ChannelMessage } from "../../../shared/types/channel";
import { PAGE_LIMIT } from "../constants";
import { mergeWithLive } from "../lib/messageFormat";
import { appendedOwnMessage, shouldStickToBottom } from "../lib/scrollRules";
import type { Messenger } from "./useMessenger";
import {
  dropConversation,
  readConversation,
  reconcileLatestPage,
  saveConversation,
  type ScrollMemory,
} from "../lib/messageCache";

export function captureScroll(body: HTMLDivElement | null, atBottom: boolean): ScrollMemory {
  if (!body || atBottom) return { atBottom: true, anchorId: null, offset: 0 };
  const top = body.getBoundingClientRect().top;
  const nodes = body.querySelectorAll<HTMLElement>("[data-msg-id]");
  for (const node of Array.from(nodes)) {
    const offset = node.getBoundingClientRect().top - top;
    if (offset >= 0) return { atBottom: false, anchorId: node.dataset.msgId ?? null, offset };
  }
  return { atBottom: true, anchorId: null, offset: 0 };
}

const HIGHLIGHT_MS = 2500;
const JUMP_CONTEXT = 25;

export function useMessages(params: {
  messenger: Messenger | null;
  convId?: string;
  isText: boolean;
  isDm: boolean;
  nameSyncTick: number;
  reseedChannelReads: () => void;
  currentUserId?: string | null;
}) {
  const {
    messenger,
    convId,
    isText,
    isDm,
    nameSyncTick,
    reseedChannelReads,
    currentUserId,
  } = params;

  const [messages, setMessages] = React.useState<ChannelMessage[]>([]);
  const [loadingInitial, setLoadingInitial] = React.useState(false);
  const [loadingOlder, setLoadingOlder] = React.useState(false);
  const [hasMore, setHasMore] = React.useState(false);
  const [accessError, setAccessError] = React.useState<string | null>(null);
  const [highlightedMessageId, setHighlightedMessageId] = React.useState<
    string | null
  >(null);

  const messagesRef = React.useRef<ChannelMessage[]>([]);
  messagesRef.current = messages;
  const messengerRef = React.useRef(messenger);
  messengerRef.current = messenger;

  const bodyRef = React.useRef<HTMLDivElement>(null);
  const loadingOlderRef = React.useRef(false);
  const skipNextAutoScrollRef = React.useRef(false);
  const restoreScrollRef = React.useRef<{ anchorId: string; offset: number } | null>(
    null,
  );
  const tailIdRef = React.useRef<string | null>(null);
  const nearBottomRef = React.useRef(true);
  const currentUserIdRef = React.useRef(currentUserId);
  currentUserIdRef.current = currentUserId;
  const loadedChannelIdRef = React.useRef<string | null>(null);
  const currentChannelIdRef = React.useRef<string | undefined>();
  const convIdRef = React.useRef(convId);
  convIdRef.current = convId;
  currentChannelIdRef.current = convId;

  const isConversation = isText || isDm;
  const conversationNotReady =
    isConversation && loadedChannelIdRef.current !== convId;
  const showInitialLoader = loadingInitial || conversationNotReady;

  const hasMoreRef = React.useRef(hasMore);
  hasMoreRef.current = hasMore;
  const accessErrorRef = React.useRef(accessError);
  accessErrorRef.current = accessError;
  const restoredFromCacheRef = React.useRef<ChannelMessage[] | null>(null);

  React.useLayoutEffect(() => {
    const body = bodyRef.current;
    const cached = convId ? readConversation(convId) : undefined;
    restoredFromCacheRef.current = cached ? cached.messages : null;
    setAccessError(null);
    restoreScrollRef.current = null;
    tailIdRef.current = null;
    if (bodyRef.current) bodyRef.current.style.overflowAnchor = "";
    if (cached && convId) {
      setMessages(cached.messages);
      setHasMore(cached.hasMore);
      nearBottomRef.current = cached.scroll.atBottom;
      if (!cached.scroll.atBottom && cached.scroll.anchorId) {
        if (bodyRef.current) bodyRef.current.style.overflowAnchor = "none";
        restoreScrollRef.current = { anchorId: cached.scroll.anchorId, offset: cached.scroll.offset };
      }
      loadedChannelIdRef.current = convId;
    } else {
      setMessages([]);
      setHasMore(false);
      nearBottomRef.current = true;
    }
    return () => {
      if (!convId || loadedChannelIdRef.current !== convId || accessErrorRef.current) return;
      saveConversation(convId, {
        messages: messagesRef.current,
        hasMore: hasMoreRef.current,
        scroll: captureScroll(body, nearBottomRef.current),
      });
    };
  }, [convId]);

  React.useEffect(() => {
    const messengerNow = messengerRef.current;
    if (!messengerNow || !convId) return;

    let cancelled = false;
    const restored = restoredFromCacheRef.current;
    const fromCache = restored !== null;
    if (!fromCache) setLoadingInitial(true);
    const targetId = convId;
    const knownBefore = new Set((restored ?? []).map((m) => m.id));
    messengerNow
      .listMessages({ limit: PAGE_LIMIT })
      .then((res) => {
        if (cancelled) return;
        const ordered = [...res.messages].reverse();
        const fullPage = res.messages.length >= PAGE_LIMIT;
        if (!fromCache) {
          setMessages((prev) => mergeWithLive(ordered, prev));
          setHasMore(fullPage);
          return;
        }
        const reconciled = reconcileLatestPage(restored ?? [], ordered, fullPage, knownBefore);
        setMessages((prev) => reconcileLatestPage(prev, ordered, fullPage, knownBefore).messages);
        if (!fullPage) setHasMore(false);
        else if (reconciled.gap) setHasMore(true);
      })
      .catch((err) => {
        if (cancelled) return;
        const apiErr = err instanceof ApiError ? err : null;
        if (
          isText &&
          (apiErr?.code === "CHANNEL_NOT_MEMBER" ||
            apiErr?.code === "CHANNEL_PASSWORD_REQUIRED" ||
            apiErr?.code === "INVALID_CHANNEL_PASSWORD")
        ) {
          dropConversation(targetId);
          setMessages([]);
          setAccessError(apiErr.message || "You haven't joined this channel yet.");
        } else if (isDm && apiErr?.code === "NOT_FRIENDS") {
          dropConversation(targetId);
          setMessages([]);
          setAccessError(i18n.t("notFriends", { ns: "chat" }));
        } else {
          console.warn("[ChatView] failed to load messages:", err);
        }
      })
      .finally(() => {
        if (cancelled) return;
        loadedChannelIdRef.current = targetId;
        setLoadingInitial(false);
      });
    return () => {
      cancelled = true;
    };
  }, [convId, isText, isDm]);

  React.useEffect(() => {
    const messengerNow = messengerRef.current;
    if (nameSyncTick === 0 || !messengerNow || !convIdRef.current) return;
    let cancelled = false;
    const targetId = convIdRef.current;
    const oldestLoadedAt = messagesRef.current[0]?.createdAt;
    (async () => {
      for (let attempt = 0; attempt < 2; attempt++) {
        try {
          const acc: ChannelMessage[] = [];
          let before: string | undefined;
          while (!cancelled && convIdRef.current === targetId) {
            const res = await messengerNow.listMessages({
              limit: PAGE_LIMIT,
              before,
            });
            const ordered = [...res.messages].reverse();
            acc.unshift(...ordered);
            if (
              res.messages.length < PAGE_LIMIT ||
              !oldestLoadedAt ||
              (acc[0] && acc[0].createdAt <= oldestLoadedAt)
            ) {
              break;
            }
            before = acc[0]?.id;
            if (!before) break;
          }
          if (cancelled || acc.length === 0 || convIdRef.current !== targetId) {
            return;
          }
          setMessages((prev) => mergeWithLive(acc, prev));
          return;
        } catch {
          await new Promise((resolve) => setTimeout(resolve, 5000));
          if (cancelled || convIdRef.current !== targetId) return;
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [nameSyncTick]);

  React.useLayoutEffect(() => {
    const body = bodyRef.current;
    if (!body) return;
    const previousTailId = tailIdRef.current;
    tailIdRef.current = messagesRef.current[messagesRef.current.length - 1]?.id ?? null;
    const restore = restoreScrollRef.current;
    if (restore) {
      restoreScrollRef.current = null;
      const anchor = body.querySelector<HTMLElement>(`[data-msg-id="${restore.anchorId}"]`);
      if (anchor) {
        body.scrollTop +=
          anchor.getBoundingClientRect().top - body.getBoundingClientRect().top - restore.offset;
      }
      body.style.overflowAnchor = "";
      return;
    }
    if (skipNextAutoScrollRef.current) {
      skipNextAutoScrollRef.current = false;
      return;
    }
    const own = appendedOwnMessage(previousTailId, messagesRef.current, currentUserIdRef.current ?? undefined);
    if (!nearBottomRef.current && !own) return;
    body.scrollTop = body.scrollHeight;
    nearBottomRef.current = true;
  }, [messages.length, loadingInitial, convId]);

  const loadOlder = React.useCallback(async () => {
    const messengerNow = messengerRef.current;
    if (!messengerNow || !convId) return;
    if (loadingOlderRef.current) return;
    if (!hasMore) return;
    if (messages.length === 0) return;
    const oldestId = messages[0].id;
    const targetId = convId;
    loadingOlderRef.current = true;
    setLoadingOlder(true);
    try {
      const res = await messengerNow.listMessages({
        before: oldestId,
        limit: PAGE_LIMIT,
      });
      if (loadedChannelIdRef.current !== targetId) return;
      const older = [...res.messages].reverse();
      if (older.length === 0) {
        setHasMore(false);
      } else {
        const body = bodyRef.current;
        const anchor = body?.querySelector<HTMLElement>(`[data-msg-id="${oldestId}"]`);
        if (body && anchor) {
          body.style.overflowAnchor = "none";
          restoreScrollRef.current = {
            anchorId: oldestId,
            offset: anchor.getBoundingClientRect().top - body.getBoundingClientRect().top,
          };
        } else {
          restoreScrollRef.current = null;
        }
        setMessages((prev) => [...older, ...prev]);
        setHasMore(res.messages.length >= PAGE_LIMIT);
      }
    } catch (err) {
      console.warn("[ChatView] failed to load older messages:", err);
    } finally {
      loadingOlderRef.current = false;
      setLoadingOlder(false);
    }
  }, [convId, hasMore, messages]);

  const handleScroll = React.useCallback(() => {
    const b = bodyRef.current;
    if (!b) return;
    nearBottomRef.current = shouldStickToBottom(
      b.scrollHeight - b.scrollTop - b.clientHeight,
      false,
    );
    if (b.scrollTop < 80) void loadOlder();
  }, [loadOlder]);

  const refreshHistory = React.useCallback(async () => {
    const messengerNow = messengerRef.current;
    if (!messengerNow) return;
    const requestConvId = convIdRef.current;
    reseedChannelReads();
    try {
      const res = await messengerNow.listMessages({ limit: PAGE_LIMIT });
      if (convIdRef.current !== requestConvId) return;
      const ordered = [...res.messages].reverse();
      setMessages((prev) => mergeWithLive(ordered, prev));
      setHasMore(res.messages.length >= PAGE_LIMIT);
    } catch (err) {
      console.warn("[ChatView] failed to refresh history after join:", err);
    }
  }, [reseedChannelReads]);

  const jumpToMessage = React.useCallback(async (target: ChannelMessage) => {
    const messengerNow = messengerRef.current;
    const targetId = convIdRef.current;
    if (!messengerNow || !targetId) return;
    setLoadingInitial(true);
    try {
      const [olderRes, newerRes] = await Promise.all([
        messengerNow.listMessages({
          before: target.id,
          limit: JUMP_CONTEXT,
        }),
        messengerNow.listMessages({
          after: target.id,
          limit: JUMP_CONTEXT,
        }),
      ]);
      if (currentChannelIdRef.current !== targetId) return;
      const older = [...olderRes.messages].reverse();
      const newer = [...newerRes.messages].reverse();
      const combined = [...older, target, ...newer];
      loadedChannelIdRef.current = targetId;
      skipNextAutoScrollRef.current = true;
      setMessages((prev) => mergeWithLive(combined, prev));
      setHasMore(olderRes.messages.length >= JUMP_CONTEXT);
      setHighlightedMessageId(target.id);
    } catch (err) {
      console.warn("[ChatView] failed to jump to message:", err);
    } finally {
      if (currentChannelIdRef.current === targetId) {
        setLoadingInitial(false);
      }
    }
  }, []);

  React.useEffect(() => {
    if (!highlightedMessageId) return;
    const timer = setTimeout(() => setHighlightedMessageId(null), HIGHLIGHT_MS);
    return () => clearTimeout(timer);
  }, [highlightedMessageId]);

  const lastScrolledToRef = React.useRef<string | null>(null);
  React.useEffect(() => {
    if (!highlightedMessageId) {
      lastScrolledToRef.current = null;
      return;
    }
    if (lastScrolledToRef.current === highlightedMessageId) return;
    const el = document.querySelector(
      `[data-msg-id="${highlightedMessageId}"]`,
    ) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
      lastScrolledToRef.current = highlightedMessageId;
    }
  }, [highlightedMessageId, messages]);

  const jumpToMessageId = React.useCallback(
    async (messageId: string) => {
      if (messagesRef.current.some((m) => m.id === messageId)) {
        setHighlightedMessageId(null);
        requestAnimationFrame(() => setHighlightedMessageId(messageId));
        return;
      }
      const messengerNow = messengerRef.current;
      if (!messengerNow) return;
      try {
        const target = await messengerNow.getMessage(messageId);
        await jumpToMessage(target);
      } catch (err) {
        console.warn("[ChatView] failed to open the original message:", err);
      }
    },
    [jumpToMessage],
  );
  return {
    messages,
    setMessages,
    messagesRef,
    loadingInitial,
    loadingOlder,
    accessError,
    setAccessError,
    highlightedMessageId,
    setHighlightedMessageId,
    showInitialLoader,
    bodyRef,
    currentChannelIdRef,
    handleScroll,
    refreshHistory,
    jumpToMessage,
    jumpToMessageId,
  };
}
