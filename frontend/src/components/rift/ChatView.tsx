import { blockGeneration, useBlockStore } from "../../store/blockStore";
import { friendshipAPI } from "../../api/friendship";
import React from "react";
import { bindingFor } from "../../services/hotkeys";
import { eventMatches } from "../../utils/hotkeys";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import CallJoinBanner from "./CallJoinBanner";
import ConfirmModal from "./ConfirmModal";
import { channelAPI } from "../../api/channel";
import { dmAPI } from "../../api/dm";
import { MAX_ATTACHMENTS_PER_MESSAGE } from "../../api/attachment";
import { userAPI } from "../../api/user";
import { useReadReceiptStore } from "../../store/readReceiptStore";
import { Attachment, ChannelMessage, SystemMessageInfo } from "../../types/channel";
import { Server } from "../../types/server";
import { ApiError } from "../../utils/apiError";
import { colorForId } from "../../utils/avatar";
import { UserAvatar } from "./Avatar";
import {
  useWsChannelMessages,
  useWsDmMessages,
} from "../../services/useWebSocketMessages";
import i18n from "../../i18n";
import { useUserNamesStore } from "../../store/userNamesStore";
import { refetchUnread } from "../../store/unreadStore";
import {
  PERM_MANAGE_MESSAGES,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";
import ChannelHeader from "./chat/ChannelHeader";
import { useAttachments } from "../../hooks/chat/useAttachments";
import { useMentions } from "../../hooks/chat/useMentions";
import { MAX_CONTENT, PAGE_LIMIT } from "./chat/constants";
import Message from "./chat/Message";
import type { ChannelMeta, DmMeta } from "./chat/types";
import { formatDayDivider, formatFileSize, mergeWithLive, sameDay, splitForSend } from "../../utils/messageFormat";


interface ChatViewProps {
  channel?: ChannelMeta;
  serverId?: string;
  currentUserId?: string;
  dm?: DmMeta;
  membersOpen?: boolean;
  servers?: Server[];
  onToggleMembers?: () => void;
  onRemoveFriend?: () => void;
  onBlockUser?: () => void;
  onGoToServer?: (serverId: string) => void | Promise<void>;
  topSlot?: React.ReactNode;
  callMode?: boolean;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  serverId,
  currentUserId,
  dm,
  membersOpen,
  servers,
  onToggleMembers,
  onRemoveFriend,
  onBlockUser,
  onGoToServer,
  topSlot,
  callMode,
}) => {
  const { t } = useTranslation("chat");
  const {
    pendingFiles,
    setPendingFiles,
    discardPendingFile,
    pendingFilesRef,
    editFilesRef,
    dragOver,
    setDragOver,
    fileInputRef,
    inFlightSendRef,
    addFiles,
    removePendingFile,
    editFiles,
    setEditFiles,
    editRemovedIds,
    setEditRemovedIds,
    addEditFiles,
    removeEditFile,
    discardEditFiles,
    toggleRemoveAttachment,
  } = useAttachments();
  const [messages, setMessages] = React.useState<ChannelMessage[]>([]);
  const dmBlocked = useBlockStore((s) => (dm ? !!s.blocked[dm.friendId] : false));
  const dmBlockedBy = useBlockStore((s) => (dm ? !!s.blockedBy[dm.friendId] : false));
  const blockedIds = useBlockStore((s) => s.blocked);
  const blocksLoaded = useBlockStore((s) => s.loaded);
  const [revealedResults, setRevealedResults] = React.useState<Set<string>>(() => new Set());
  const blockPeerId = dm?.friendId;
  React.useEffect(() => {
    if (!blockPeerId) return;
    let cancelled = false;
    const gen = blockGeneration(blockPeerId);
    friendshipAPI
      .blockRelation(blockPeerId)
      .then((relation) => {
        if (!cancelled) useBlockStore.getState().applyRelation(blockPeerId, relation, gen);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [blockPeerId]);
  const messagesRef = React.useRef<ChannelMessage[]>([]);
  messagesRef.current = messages;
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);

  const dmFriendId = dm?.friendId;
  const channelId = channel?.id;
  const dmPeerReadAt = useReadReceiptStore((s) =>
    dmFriendId ? s.dmPeerReadAt[dmFriendId] : undefined,
  );
  const channelReads = useReadReceiptStore((s) =>
    channelId ? s.channelReads[channelId] : undefined,
  );

  React.useEffect(() => {
    if (!dmFriendId) return;
    userAPI
      .getDmReadState(dmFriendId)
      .then((r) =>
        useReadReceiptStore.getState().seedDmPeerReadAt(dmFriendId, r.readAt),
      )
      .catch(() => {});
  }, [dmFriendId]);

  React.useEffect(() => {
    if (!channelId) return;
    const seed = () =>
      userAPI
        .getChannelReadStates(channelId)
        .then((r) =>
          useReadReceiptStore.getState().seedChannelReads(channelId, r.members),
        )
        .catch(() => {});
    void seed();
    const interval = setInterval(seed, 60_000);
    return () => clearInterval(interval);
  }, [channelId]);

  const reseedChannelReads = React.useCallback(() => {
    if (!channelId) return;
    userAPI
      .getChannelReadStates(channelId)
      .then((r) =>
        useReadReceiptStore.getState().seedChannelReads(channelId, r.members),
      )
      .catch(() => {});
  }, [channelId]);

  const receiptFor = (
    m: ChannelMessage,
    mine: boolean,
  ): "sent" | "read" | null => {
    if (!mine || m.callStatus || m.inviteServerId) return null;
    const created = new Date(m.createdAt).getTime();
    if (dmFriendId) {
      return dmPeerReadAt && new Date(dmPeerReadAt).getTime() >= created
        ? "read"
        : "sent";
    }
    if (channelId) {
      if (!channelReads) return "sent";
      const members = Object.keys(channelReads);
      if (members.length === 0) return "sent";
      const all = members.every((uid) => {
        const r = channelReads[uid];
        return r != null && new Date(r).getTime() >= created;
      });
      return all ? "read" : "sent";
    }
    return null;
  };

  const channelMsgHandlers = React.useMemo(
    () => ({
      onSent: (m: {
        id: string;
        channelId: string;
        authorId: string;
        authorName?: string;
        content: string;
        occurredAt: string;
        attachments?: Attachment[];
        kind?: "user" | "system";
        system?: SystemMessageInfo;
      }) => {
        setMessages((prev) =>
          prev.some((x) => x.id === m.id)
            ? prev
            : [
                ...prev,
                {
                  id: m.id,
                  channelId: m.channelId,
                  authorId: m.authorId,
                  authorName: m.authorName,
                  content: m.content,
                  attachments: m.attachments,
                  kind: m.kind,
                  system: m.system,
                  createdAt: m.occurredAt,
                  editedAt: null,
                },
              ],
        );
      },
      onEdited: (m: {
        id: string;
        channelId: string;
        newContent: string;
        occurredAt: string;
        attachments?: Attachment[];
      }) => {
        setMessages((prev) =>
          prev.map((x) =>
            x.id === m.id
              ? {
                  ...x,
                  content: m.newContent,
                  editedAt: m.occurredAt,
                  ...(m.attachments ? { attachments: m.attachments } : {}),
                }
              : x,
          ),
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => prev.filter((x) => x.id !== m.id));
      },
    }),
    [currentUserId],
  );
  useWsChannelMessages(channel?.id ?? null, channelMsgHandlers);

  const dmMsgHandlers = React.useMemo(
    () => ({
      onSent: (m: {
        id: string;
        senderId: string;
        senderName?: string;
        recipientId: string;
        content: string;
        occurredAt: string;
        inviteServerId?: string;
        inviteServerName?: string;
        callStatus?: string;
        callDurationSeconds?: number;
        attachments?: Attachment[];
      }) => {
        setMessages((prev) =>
          prev.some((x) => x.id === m.id)
            ? prev
            : [
                ...prev,
                {
                  id: m.id,
                  channelId: m.senderId,
                  authorId: m.senderId,
                  authorName: m.senderName,
                  content: m.content,
                  inviteServerId: m.inviteServerId,
                  inviteServerName: m.inviteServerName,
                  callStatus: m.callStatus,
                  callDurationSeconds: m.callDurationSeconds,
                  attachments: m.attachments,
                  createdAt: m.occurredAt,
                  editedAt: null,
                },
              ],
        );
      },
      onEdited: (m: {
        id: string;
        senderId: string;
        newContent: string;
        occurredAt: string;
        attachments?: Attachment[];
      }) => {
        setMessages((prev) =>
          prev.map((x) =>
            x.id === m.id
              ? {
                  ...x,
                  content: m.newContent,
                  editedAt: m.occurredAt,
                  ...(m.attachments ? { attachments: m.attachments } : {}),
                }
              : x,
          ),
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => prev.filter((x) => x.id !== m.id));
      },
    }),
    [currentUserId],
  );
  useWsDmMessages(dm?.friendId ?? null, currentUserId, dmMsgHandlers);

  const [text, setText] = React.useState("");
  const [loadingInitial, setLoadingInitial] = React.useState(false);
  const [loadingOlder, setLoadingOlder] = React.useState(false);
  const [hasMore, setHasMore] = React.useState(false);
  const [composerError, setComposerError] = React.useState<string | null>(null);
  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [editText, setEditText] = React.useState("");
  const [savingEdit, setSavingEdit] = React.useState(false);
  const [deleteTarget, setDeleteTarget] = React.useState<ChannelMessage | null>(null);
  const [sending, setSending] = React.useState(false);
  const [accessError, setAccessError] = React.useState<string | null>(null);
  const [searchOpen, setSearchOpen] = React.useState(false);
  const [searchQuery, setSearchQuery] = React.useState("");
  const [searchInput, setSearchInput] = React.useState("");
  const [searchResults, setSearchResults] = React.useState<ChannelMessage[]>([]);
  const [searchLoading, setSearchLoading] = React.useState(false);
  const [searchError, setSearchError] = React.useState<string | null>(null);
  const [searchTotal, setSearchTotal] = React.useState(0);
  const [highlightedMessageId, setHighlightedMessageId] = React.useState<string | null>(null);

  const composerRef = React.useRef<HTMLInputElement>(null);
  const wasSendingRef = React.useRef(false);

  const rolesData = useRolesStore((s) =>
    serverId ? s.byServer[serverId] : undefined,
  );
  const fetchRoles = useRolesStore((s) => s.fetch);
  React.useEffect(() => {
    if (serverId) void fetchRoles(serverId).catch(() => {});
  }, [serverId, fetchRoles]);
  const serverOwnerId = React.useMemo(
    () => servers?.find((sv) => sv.id === serverId)?.ownerId,
    [servers, serverId],
  );
  const {
    mentionQuery,
    setMentionQuery,
    mentionIndex,
    setMentionIndex,
    mentionSuggestions,
    resetParticipants,
    updateMentionQuery,
    applyMention,
  } = useMentions({
    channel,
    serverId,
    currentUserId,
    nameOverrides,
    text,
    setText,
    composerRef,
  });

  const canModerateMessages =
    !!channel &&
    !!currentUserId &&
    (serverOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_MANAGE_MESSAGES) !== 0);

  React.useEffect(() => {
    if (wasSendingRef.current && !sending) {
      composerRef.current?.focus();
    }
    wasSendingRef.current = sending;
  }, [sending]);

  const bodyRef = React.useRef<HTMLDivElement>(null);
  const loadingOlderRef = React.useRef(false);
  const skipNextAutoScrollRef = React.useRef(false);
  const lastScrollHeightRef = React.useRef(0);
  const loadedChannelIdRef = React.useRef<string | null>(null);
  const currentChannelIdRef = React.useRef<string | undefined>();
  const convId = channel?.id ?? dm?.friendId;
  const convIdRef = React.useRef(convId);
  convIdRef.current = convId;
  currentChannelIdRef.current = convId;

  const isText = !!channel && !!serverId;
  const isDm = !!dm && !!dm.friendId;
  const isConversation = isText || isDm;
  const conversationNotReady =
    isConversation && loadedChannelIdRef.current !== convId;
  const showInitialLoader = loadingInitial || conversationNotReady;

  const messenger = React.useMemo(() => {
    if (isDm && dm) {
      return {
        listMessages: (opts: {
          before?: string;
          after?: string;
          limit?: number;
        }) => dmAPI.listMessages(dm.friendId, opts),
        sendMessage: (content: string, attachmentIds?: string[]) =>
          dmAPI.sendMessage(dm.friendId, content, attachmentIds),
        editMessage: (
          messageId: string,
          content: string,
          addAttachmentIds?: string[],
          removeAttachmentIds?: string[],
        ) =>
          dmAPI.editMessage(
            messageId,
            content,
            addAttachmentIds,
            removeAttachmentIds,
          ),
        deleteMessage: (messageId: string) => dmAPI.deleteMessage(messageId),
        searchMessages: (query: string, page: number, perPage: number) =>
          dmAPI.searchMessages(dm.friendId, query, page, perPage),
      };
    }
    if (channel && serverId) {
      return {
        listMessages: (opts: {
          before?: string;
          after?: string;
          limit?: number;
        }) => channelAPI.listMessages(serverId, channel.id, opts),
        sendMessage: (content: string, attachmentIds?: string[]) =>
          channelAPI.sendMessage(serverId, channel.id, content, attachmentIds),
        editMessage: (
          messageId: string,
          content: string,
          addAttachmentIds?: string[],
          removeAttachmentIds?: string[],
        ) =>
          channelAPI.editMessage(
            serverId,
            channel.id,
            messageId,
            content,
            addAttachmentIds,
            removeAttachmentIds,
          ),
        deleteMessage: (messageId: string) =>
          channelAPI.deleteMessage(serverId, channel.id, messageId),
        searchMessages: (query: string, page: number, perPage: number) =>
          channelAPI.searchMessages(serverId, channel.id, query, page, perPage),
      };
    }
    return null;
  }, [isDm, dm, channel, serverId]);

  React.useEffect(() => {
    setMessages([]);
    setText("");
    setComposerError(null);
    setMentionQuery(null);
    setMentionIndex(0);
    resetParticipants();
    setPullDistance(0);
    pullStartYRef.current = null;
    wheelPullRef.current = 0;
    setPendingFiles((prev) => {
      for (const p of prev) {
        if (!inFlightSendRef.current.has(p.localId)) discardPendingFile(p);
      }
      return [];
    });
    setEditingId(null);
    setEditText("");
    discardEditFiles();
    setDeleteTarget(null);
    setHasMore(false);
    setAccessError(null);
    setSearchOpen(false);
    setSearchQuery("");
    setSearchInput("");
    setSearchResults([]);
    setSearchError(null);
    setSearchTotal(0);
    if (!messenger || !convId) return;

    let cancelled = false;
    setLoadingInitial(true);
    const targetId = convId;
    messenger
      .listMessages({ limit: PAGE_LIMIT })
      .then((res) => {
        if (cancelled) return;
        const ordered = [...res.messages].reverse();
        setMessages((prev) => mergeWithLive(ordered, prev));
        setHasMore(res.messages.length >= PAGE_LIMIT);
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
          setAccessError(apiErr.message || "You haven't joined this channel yet.");
        } else if (isDm && apiErr?.code === "NOT_FRIENDS") {
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [convId, isText, isDm]);

  React.useEffect(() => {
    if (nameSyncTick === 0 || !messenger || !convId) return;
    let cancelled = false;
    const targetId = convId;
    const oldestLoadedAt = messagesRef.current[0]?.createdAt;
    (async () => {
      for (let attempt = 0; attempt < 2; attempt++) {
        try {
          const acc: ChannelMessage[] = [];
          let before: string | undefined;
          while (!cancelled && convIdRef.current === targetId) {
            const res = await messenger.listMessages({
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nameSyncTick]);

  React.useEffect(() => {
    if (!bodyRef.current) return;
    if (skipNextAutoScrollRef.current) {
      skipNextAutoScrollRef.current = false;
      return;
    }
    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [messages.length, loadingInitial]);

  const loadOlder = React.useCallback(async () => {
    if (!messenger || !convId) return;
    if (loadingOlderRef.current) return;
    if (!hasMore) return;
    if (messages.length === 0) return;
    const oldestId = messages[0].id;
    const targetId = convId;
    loadingOlderRef.current = true;
    setLoadingOlder(true);
    const body = bodyRef.current;
    let prevScrollTop = 0;
    if (body) {
      lastScrollHeightRef.current = body.scrollHeight;
      prevScrollTop = body.scrollTop;
    }
    try {
      const res = await messenger.listMessages({
        before: oldestId,
        limit: PAGE_LIMIT,
      });
      if (loadedChannelIdRef.current !== targetId) return;
      const older = [...res.messages].reverse();
      if (older.length === 0) {
        setHasMore(false);
      } else {
        skipNextAutoScrollRef.current = true;
        setMessages((prev) => [...older, ...prev]);
        setHasMore(res.messages.length >= PAGE_LIMIT);
        requestAnimationFrame(() => {
          const b = bodyRef.current;
          if (!b) return;
          const delta = b.scrollHeight - lastScrollHeightRef.current;
          b.scrollTop = prevScrollTop + delta;
        });
      }
    } catch (err) {
      console.warn("[ChatView] failed to load older messages:", err);
    } finally {
      loadingOlderRef.current = false;
      setLoadingOlder(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [convId, hasMore, messages]);

  const handleScroll = React.useCallback(() => {
    const b = bodyRef.current;
    if (!b) return;
    if (searchOpen && searchInput.trim().length > 0) return;
    if (b.scrollTop < 80) void loadOlder();
  }, [loadOlder, searchOpen, searchInput]);

  const sendOnce = async (
    content: string,
    attachmentIds?: string[],
  ): Promise<ChannelMessage> => {
    if (!messenger) throw new Error("no conversation");
    return messenger.sendMessage(content, attachmentIds);
  };

  const refreshHistoryAfterJoin = async () => {
    if (!messenger) return;
    const requestConvId = convIdRef.current;
    reseedChannelReads();
    try {
      const res = await messenger.listMessages({ limit: PAGE_LIMIT });
      if (convIdRef.current !== requestConvId) return;
      const ordered = [...res.messages].reverse();
      setMessages((prev) => mergeWithLive(ordered, prev));
      setHasMore(res.messages.length >= PAGE_LIMIT);
    } catch (err) {
      console.warn("[ChatView] failed to refresh history after join:", err);
    }
  };

  const PULL_THRESHOLD = 64;
  const PULL_MAX = 96;
  const [pullDistance, setPullDistance] = React.useState(0);
  const [refreshing, setRefreshing] = React.useState(false);
  const pullStartYRef = React.useRef<number | null>(null);
  const wheelPullRef = React.useRef(0);
  const wheelResetTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const refreshingRef = React.useRef(false);
  refreshingRef.current = refreshing;

  const triggerPullRefresh = React.useCallback(async () => {
    if (refreshingRef.current || !messenger) {
      setPullDistance(0);
      return;
    }
    setRefreshing(true);
    setPullDistance(PULL_THRESHOLD);
    const started = Date.now();
    try {
      await refreshHistoryAfterJoin();
      void refetchUnread().catch(() => {});
    } finally {
      const wait = Math.max(0, 650 - (Date.now() - started));
      setTimeout(() => {
        setRefreshing(false);
        setPullDistance(0);
        wheelPullRef.current = 0;
      }, wait);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [messenger]);

  const atBottom = (b: HTMLDivElement) =>
    b.scrollTop + b.clientHeight >= b.scrollHeight - 1;

  const onPullTouchStart = (e: React.TouchEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    pullStartYRef.current = b && atBottom(b) ? e.touches[0].clientY : null;
  };

  const onPullTouchMove = (e: React.TouchEvent) => {
    if (refreshingRef.current || pullStartYRef.current === null) return;
    const b = bodyRef.current;
    if (!b || !atBottom(b)) {
      pullStartYRef.current = null;
      setPullDistance(0);
      return;
    }
    const dy = pullStartYRef.current - e.touches[0].clientY;
    setPullDistance(dy > 0 ? Math.min(dy * 0.45, PULL_MAX) : 0);
  };

  const onPullTouchEnd = () => {
    if (refreshingRef.current) return;
    pullStartYRef.current = null;
    if (pullDistance >= PULL_THRESHOLD) {
      void triggerPullRefresh();
    } else {
      setPullDistance(0);
    }
  };

  const onPullWheel = (e: React.WheelEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    if (!b || !atBottom(b) || e.deltaY <= 0) {
      if (wheelPullRef.current !== 0 && b && !atBottom(b)) {
        wheelPullRef.current = 0;
        setPullDistance(0);
      }
      return;
    }
    wheelPullRef.current = Math.min(
      wheelPullRef.current + e.deltaY * 0.35,
      PULL_MAX,
    );
    setPullDistance(wheelPullRef.current);
    if (wheelResetTimerRef.current) clearTimeout(wheelResetTimerRef.current);
    if (wheelPullRef.current >= PULL_THRESHOLD) {
      void triggerPullRefresh();
      return;
    }
    wheelResetTimerRef.current = setTimeout(() => {
      wheelResetTimerRef.current = null;
      if (!refreshingRef.current) {
        wheelPullRef.current = 0;
        setPullDistance(0);
      }
    }, 400);
  };

  React.useEffect(() => {
    const trimmed = searchInput.trim();
    if (!searchOpen) return;
    if (!messenger) return;
    if (trimmed.length === 0) {
      setSearchQuery("");
      setSearchResults([]);
      setSearchError(null);
      setSearchTotal(0);
      return;
    }
    const timeout = window.setTimeout(() => setSearchQuery(trimmed), 250);
    return () => window.clearTimeout(timeout);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchInput, searchOpen, convId]);

  React.useEffect(() => {
    if (!searchOpen || !messenger) return;
    const q = searchQuery.trim();
    if (q.length === 0) return;
    let cancelled = false;
    setSearchLoading(true);
    setSearchError(null);
    messenger
      .searchMessages(q, 1, 50)
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchQuery, searchOpen, convId, nameSyncTick]);

  const handleJumpToMessage = async (target: ChannelMessage) => {
    if (!messenger || !convId) return;
    const targetId = convId;
    setSearchInput("");
    setSearchQuery("");
    setSearchResults([]);
    setSearchError(null);
    setSearchTotal(0);
    setLoadingInitial(true);
    try {
      const [olderRes, newerRes] = await Promise.all([
        messenger.listMessages({
          before: target.id,
          limit: 25,
        }),
        messenger.listMessages({
          after: target.id,
          limit: 25,
        }),
      ]);
      if (currentChannelIdRef.current !== targetId) return;
      const older = [...olderRes.messages].reverse();
      const newer = [...newerRes.messages].reverse();
      const combined = [...older, target, ...newer];
      loadedChannelIdRef.current = targetId;
      skipNextAutoScrollRef.current = true;
      setMessages((prev) => mergeWithLive(combined, prev));
      setHasMore(olderRes.messages.length >= 25);
      setHighlightedMessageId(target.id);
    } catch (err) {
      console.warn("[ChatView] failed to jump to message:", err);
    } finally {
      if (currentChannelIdRef.current === targetId) {
        setLoadingInitial(false);
      }
    }
  };

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

  const toggleSearch = () => {
    setSearchOpen((prev) => {
      const next = !prev;
      if (!next) {
        setSearchInput("");
        setSearchQuery("");
        setSearchResults([]);
        setSearchError(null);
        setSearchTotal(0);
        setHighlightedMessageId(null);
      }
      return next;
    });
  };

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
  }, [searchOpen]);

  const handleJoinFromAccessError = async () => {
    if (!channel || !serverId) return;
    try {
      await channelAPI.joinText(serverId, channel.id);
      setAccessError(null);
      await refreshHistoryAfterJoin();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "CHANNEL_PASSWORD_REQUIRED" || code === "INVALID_CHANNEL_PASSWORD") {
        const pw = window.prompt(`Enter password for #${channel.name}`);
        if (pw === null) return;
        try {
          await channelAPI.joinText(serverId, channel.id, pw);
          setAccessError(null);
          await refreshHistoryAfterJoin();
        } catch (retryErr) {
          const r = retryErr instanceof ApiError ? retryErr : null;
          setAccessError(r?.message || i18n.t("couldNotJoin", { ns: "chat" }));
        }
      } else {
        setAccessError(apiErr?.message || i18n.t("couldNotJoin", { ns: "chat" }));
      }
    }
  };

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!messenger) return;
    const v = text.trim();
    const readyIds = pendingFiles
      .filter((p) => p.attachment)
      .map((p) => p.attachment?.id ?? "");
    const stillUploading = pendingFiles.some((p) => !p.attachment && !p.error);
    if (stillUploading) {
      setComposerError(i18n.t("attachmentsUploading", { ns: "chat" }));
      return;
    }
    if (!v && readyIds.length === 0) return;
    const chunks = v ? splitForSend(v, MAX_CONTENT) : [""];
    if (chunks.length === 0) return;

    setSending(true);
    setComposerError(null);

    const originConvId = currentChannelIdRef.current;
    const stillHere = () => currentChannelIdRef.current === originConvId;
    const reportComposerError = (message: string) => {
      if (stillHere()) setComposerError(message);
    };

    const remaining = [...chunks];
    const updateComposer = () => {
      if (stillHere()) setText(remaining.join("\n\n"));
    };

    const sentFiles = pendingFiles.filter((p) => p.attachment);
    const sentLocalIds = new Set(sentFiles.map((p) => p.localId));
    const sentAttachments = sentFiles.map((p) => p.attachment as Attachment);
    let idsToAttach: string[] | undefined =
      readyIds.length > 0 ? readyIds : undefined;
    if (idsToAttach) {
      for (const id of sentLocalIds) inFlightSendRef.current.add(id);
    }
    const sendChunk = async (chunk: string) => {
      const msg = await sendOnce(chunk, idsToAttach);
      if (idsToAttach) {
        msg.attachments = msg.attachments ?? sentAttachments;
        idsToAttach = undefined;
        for (const id of sentLocalIds) inFlightSendRef.current.delete(id);
        setPendingFiles((prev) =>
          prev.filter((p) => !sentLocalIds.has(p.localId)),
        );
      }
      if (stillHere()) {
        setMessages((prev) =>
          prev.some((x) => x.id === msg.id) ? prev : [...prev, msg],
        );
      }
    };

    try {
      try {
        await sendChunk(remaining[0]);
        remaining.shift();
        updateComposer();
      } catch (err) {
        const apiErr = err instanceof ApiError ? err : null;
        const code = apiErr?.code;
        if (isText && channel && serverId && code === "CHANNEL_NOT_MEMBER") {
          try {
            await channelAPI.joinText(serverId, channel.id);
            await refreshHistoryAfterJoin();
            await sendChunk(remaining[0]);
            remaining.shift();
            updateComposer();
          } catch (joinErr) {
            const joinApi = joinErr instanceof ApiError ? joinErr : null;
            const jc = joinApi?.code;
            if (jc === "CHANNEL_PASSWORD_REQUIRED" || jc === "INVALID_CHANNEL_PASSWORD") {
              const pw = window.prompt(`Enter password for #${channel.name}`);
              if (pw === null) {
                setSending(false);
                return;
              }
              await channelAPI.joinText(serverId, channel.id, pw);
              await refreshHistoryAfterJoin();
              await sendChunk(remaining[0]);
              remaining.shift();
              updateComposer();
            } else {
              reportComposerError(joinApi?.message || i18n.t("couldNotJoin", { ns: "chat" }));
              return;
            }
          }
        } else {
          throw err;
        }
      }
      while (remaining.length > 0) {
        await sendChunk(remaining[0]);
        remaining.shift();
        updateComposer();
      }
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "MESSAGE_CONTENT_EMPTY") {
        reportComposerError(i18n.t("messageCantBeEmpty", { ns: "chat" }));
      } else if (code === "MESSAGE_CONTENT_TOO_LONG") {
        reportComposerError(i18n.t("chunkTooLong", { ns: "chat" }));
      } else if (code === "NOT_FRIENDS") {
        reportComposerError(i18n.t("notFriends", { ns: "chat" }));
      } else if (code === "USER_BLOCKED") {
        reportComposerError(i18n.t("userBlocked", { ns: "chat" }));
        if (dm) {
          const peerId = dm.friendId;
          const gen = blockGeneration(peerId);
          void friendshipAPI
            .blockRelation(peerId)
            .then((relation) => useBlockStore.getState().applyRelation(peerId, relation, gen))
            .catch(() => {});
        }
      } else if (code === "DMS_CLOSED") {
        reportComposerError(i18n.t("dmsClosed", { ns: "chat" }));
      } else if (code === "RECIPIENT_INACTIVE") {
        reportComposerError(i18n.t("recipientInactive", { ns: "chat" }));
      } else {
        console.warn("[ChatView] send failed:", err);
        reportComposerError(apiErr?.message || i18n.t("couldNotSend", { ns: "chat" }));
      }
    } finally {
      for (const id of sentLocalIds) inFlightSendRef.current.delete(id);
      if (idsToAttach) {
        for (const p of sentFiles) {
          if (
            !pendingFilesRef.current.some((x) => x.localId === p.localId)
          ) {
            discardPendingFile(p);
          }
        }
      }
      setSending(false);
    }
  };

  const myServerIds = React.useMemo(
    () => new Set((servers ?? []).map((s) => s.id)),
    [servers],
  );

  const handleAcceptInvite = async (msg: ChannelMessage) => {
    if (!msg.inviteServerId) return;
    setComposerError(null);
    try {
      await dmAPI.acceptServerInvite(msg.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setComposerError(apiErr?.message || i18n.t("couldNotJoinServer", { ns: "chat" }));
      return;
    }
    try {
      await onGoToServer?.(msg.inviteServerId);
    } catch (err) {
      console.warn("[ChatView] post-accept navigation failed:", err);
    }
  };

  const handleDeclineInvite = async (msg: ChannelMessage) => {
    if (!msg.inviteServerId) return;
    setComposerError(null);
    try {
      await dmAPI.declineServerInvite(msg.id);
      setMessages((prev) => prev.filter((m) => m.id !== msg.id));
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setComposerError(apiErr?.message || "Couldn't decline the invite.");
    }
  };

  const startEdit = (msg: ChannelMessage) => {
    discardEditFiles();
    setEditingId(msg.id);
    setEditText(msg.content);
  };

  const cancelEdit = () => {
    if (savingEdit) return;
    discardEditFiles();
    setEditingId(null);
    setEditText("");
  };

  const saveEdit = async (msg: ChannelMessage) => {
    if (!messenger || savingEdit) return;
    const v = editText.trim();
    const currentIds = (msg.attachments ?? []).map((a) => a.id);
    const removeIds = editRemovedIds.filter((id) => currentIds.includes(id));
    const readyAdds = editFiles.filter((p) => p.attachment);
    const addIds = readyAdds.map((p) => p.attachment!.id);
    if (editFiles.some((p) => !p.attachment && !p.error)) return;
    const finalCount = currentIds.length - removeIds.length + addIds.length;
    if (!v && finalCount === 0) return;
    if (v === msg.content && removeIds.length === 0 && addIds.length === 0) {
      cancelEdit();
      return;
    }
    if (v.length > MAX_CONTENT) return;
    setSavingEdit(true);
    const sentLocalIds = new Set(readyAdds.map((p) => p.localId));
    for (const id of sentLocalIds) inFlightSendRef.current.add(id);
    let linked = false;
    try {
      await messenger.editMessage(msg.id, v, addIds, removeIds);
      linked = true;
      const editedAt = new Date().toISOString();
      const attachments = (msg.attachments ?? [])
        .filter((a) => !removeIds.includes(a.id))
        .concat(readyAdds.map((p) => p.attachment!));
      setMessages((prev) =>
        prev.map((m) =>
          m.id === msg.id ? { ...m, content: v, editedAt, attachments } : m,
        ),
      );
      for (const p of editFilesRef.current) {
        if (!sentLocalIds.has(p.localId)) discardPendingFile(p);
      }
      setEditFiles([]);
      setEditRemovedIds([]);
      setEditingId(null);
      setEditText("");
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (
        apiErr?.code === "MESSAGE_CONTENT_UNCHANGED" ||
        apiErr?.code === "DM_CONTENT_UNCHANGED"
      ) {
        setSavingEdit(false);
        cancelEdit();
        return;
      }
      console.warn("[ChatView] edit failed:", err);
    } finally {
      for (const id of sentLocalIds) inFlightSendRef.current.delete(id);
      if (!linked) {
        for (const p of readyAdds) {
          if (!editFilesRef.current.some((x) => x.localId === p.localId)) {
            discardPendingFile(p);
          }
        }
      }
      setSavingEdit(false);
    }
  };

  const confirmDelete = async () => {
    if (!deleteTarget || !messenger) return;
    const target = deleteTarget;
    try {
      await messenger.deleteMessage(target.id);
      setMessages((prev) => prev.filter((m) => m.id !== target.id));
    } catch (err) {
      console.warn("[ChatView] delete failed:", err);
    } finally {
      setDeleteTarget(null);
    }
  };

  if (!channel && !dm) return null;

  const placeholder = dm
    ? i18n.t("messagePlaceholderDm", { ns: "chat", user: dm.name })
    : i18n.t("messagePlaceholder", { ns: "chat", channel: channel?.name ?? "" });

  const headerName = channel ? channel.name : dm!.name;

  return (
    <>
      <ChannelHeader
        channel={channel}
        dm={dm}
        blocked={dmBlocked}
        blockedBy={dmBlockedBy}
        membersOpen={membersOpen}
        searchActive={searchOpen}
        onToggleSearch={toggleSearch}
        onToggleMembers={onToggleMembers}
        onRemoveFriend={onRemoveFriend}
        onBlockUser={onBlockUser}
        onUnblockUser={() => {
          if (dm) void useBlockStore.getState().unblock(dm.friendId).catch(() => {});
        }}
      />
      {topSlot}
      {dm && !callMode && <CallJoinBanner friendId={dm.friendId} />}
      {searchOpen && (channel || dm) && (
        <div className="search-bar">
          <Icon name="search" size={14} />
          <input
            autoFocus
            type="text"
            placeholder={
              channel
                ? i18n.t("searchInChannel", { ns: "chat", channel: channel.name })
                : i18n.t("searchWithUser", { ns: "chat", user: dm!.name })
            }
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
          />
          <button
            type="button"
            className="search-clear"
            onClick={toggleSearch}
            title={i18n.t("closeSearch", { ns: "chat" })}
          >
            <Icon name="x" size={12} />
          </button>
        </div>
      )}
      <div
        className="chat-body-wrap"
        style={callMode ? { display: "none" } : undefined}
      >
      <div
        className={"pull-refresh" + (refreshing ? " refreshing" : "")}
        style={{
          opacity: refreshing ? 1 : Math.min(pullDistance / PULL_THRESHOLD, 1),
          transform: `translateX(-50%) translateY(${
            44 - (refreshing ? PULL_THRESHOLD : pullDistance)
          }px) scale(${
            refreshing
              ? 1
              : 0.6 + 0.4 * Math.min(pullDistance / PULL_THRESHOLD, 1)
          })`,
        }}
      >
        <span
          className="pull-refresh-icon"
          style={
            refreshing
              ? undefined
              : { transform: `rotate(${pullDistance * 2.5}deg)` }
          }
        >
          <Icon name="refresh" size={15} />
        </span>
      </div>
      <div
        className={"chat-body" + (dragOver ? " drag-over" : "")}
        ref={bodyRef}
        onScroll={handleScroll}
        onTouchStart={onPullTouchStart}
        onTouchMove={onPullTouchMove}
        onTouchEnd={onPullTouchEnd}
        onWheel={onPullWheel}
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes("Files")) {
            e.preventDefault();
            setDragOver(true);
          }
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          if (e.dataTransfer.files.length > 0) {
            e.preventDefault();
            setDragOver(false);
            addFiles(e.dataTransfer.files);
          }
        }}
        style={{
          transform: `translateY(${-pullDistance}px)`,
          transition:
            pullStartYRef.current !== null
              ? "none"
              : "transform 0.3s cubic-bezier(0.22, 1, 0.36, 1)",
        }}
      >
        {searchOpen && searchInput.trim().length > 0 ? (
          <div className="search-results">
            {searchLoading ? (
              <div className="empty-state">
                <span className="loading-spinner" />
              </div>
            ) : searchError ? (
              <div className="empty-state">
                <p>{searchError}</p>
              </div>
            ) : !dm && !blocksLoaded ? (
              <div className="msg-loader">
                <span className="loading-spinner" />
              </div>
            ) : searchResults.length === 0 ? (
              <div className="empty-state">
                <div className="icon-wrap">
                  <Icon name="search" size={22} />
                </div>
                <h3>{i18n.t("noResults", { ns: "chat" })}</h3>
                <p>{i18n.t("noResultsBody", { ns: "chat", query: searchQuery })}</p>
              </div>
            ) : (
              <>
                <div className="search-meta">
                  {i18n.t(searchTotal === 1 ? "resultsFor" : "resultsForPlural", {
                    ns: "chat",
                    count: searchTotal,
                    query: searchQuery,
                  })}
                </div>
                {searchResults.map((m) => {
                  const isMine = !!currentUserId && m.authorId === currentUserId;
                  const hiddenAuthor =
                    !dm && !!blockedIds[m.authorId] && !revealedResults.has(m.id);
                  const displayName = isMine
                    ? i18n.t("you", { ns: "chat" })
                    : (nameOverrides[m.authorId] ?? m.authorName ?? m.authorId);
                  const time = new Date(m.createdAt).toLocaleString([], {
                    month: "short",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                  });
                  return (
                    <button
                      key={m.id}
                      type="button"
                      className="search-result"
                      onClick={() => handleJumpToMessage(m)}
                    >
                      <UserAvatar
                        userId={m.authorId}
                        name={displayName}
                        className="ava"
                      />
                      <div className="body">
                        <div className="head">
                          <b>{displayName}</b>
                          <time>{time}</time>
                        </div>
                        {hiddenAuthor ? (
                          <div className="content msg-system-text">
                            {i18n.t("blockedMessage", { ns: "chat" })}{" "}
                            <span
                              role="button"
                              tabIndex={0}
                              className="link-btn"
                              onClick={(e) => {
                                e.stopPropagation();
                                setRevealedResults((prev) => new Set(prev).add(m.id));
                              }}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                  e.preventDefault();
                                  e.stopPropagation();
                                  setRevealedResults((prev) => new Set(prev).add(m.id));
                                }
                              }}
                            >
                              {i18n.t("showBlockedMessage", { ns: "chat" })}
                            </span>
                          </div>
                        ) : (
                          <div className="content">{m.content}</div>
                        )}
                      </div>
                    </button>
                  );
                })}
              </>
            )}
          </div>
        ) : showInitialLoader ? (
          <div className="empty-state">
            <span className="loading-spinner" />
          </div>
        ) : accessError ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name={isDm ? "user" : "hash"} size={22} />
            </div>
            <h3>
              {isDm ? "Can't open this conversation" : `Join #${headerName}`}
            </h3>
            <p>{accessError}</p>
            {isText && (
              <button
                type="button"
                className="btn primary"
                onClick={handleJoinFromAccessError}
              >
                Join channel
              </button>
            )}
          </div>
        ) : !dm && !blocksLoaded ? (
          <div className="msg-loader">
            <span className="loading-spinner" />
          </div>
        ) : messages.length === 0 ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name="hash" size={22} />
            </div>
            <h3>{t("welcomeTitle", { name: headerName })}</h3>
            <p>{t("welcomeBody")}</p>
          </div>
        ) : (
          <div className="inner">
            {loadingOlder && (
              <div className="msg-loader">
                <span className="loading-spinner" />
              </div>
            )}
            {messages.map((m, i) => {
              const prev = messages[i - 1];
              const curDate = new Date(m.createdAt);
              const prevDate = prev ? new Date(prev.createdAt) : null;
              const newDay = !prevDate || !sameDay(curDate, prevDate);
              const sameAuthor = !!prev && prev.authorId === m.authorId;
              const within = !!prev && curDate.getTime() - new Date(prev.createdAt).getTime() < 5 * 60 * 1000;
              const cont = sameAuthor && within && !newDay && !m.system && !prev?.system;
              const isMine = !!currentUserId && m.authorId === currentUserId;
              const subjectName = nameOverrides[m.authorId] ?? m.authorName ?? m.authorId;
              const displayName = m.system ? subjectName : isMine ? "You" : subjectName;
              return (
                <React.Fragment key={m.id}>
                  {newDay && (
                    <div className="day-divider">
                      <span>{formatDayDivider(curDate)}</span>
                    </div>
                  )}
                <Message
                  blockedAuthor={!dm && !m.system && !!blockedIds[m.authorId]}
                  msg={m}
                  displayName={displayName}
                  continued={cont}
                  isMine={isMine}
                  dmStyle={!!dm}
                  receipt={receiptFor(m, isMine)}
                  editing={editingId === m.id}
                  highlighted={highlightedMessageId === m.id}
                  editText={editText}
                  inviteIsMember={
                    !!m.inviteServerId && myServerIds.has(m.inviteServerId)
                  }
                  canModerate={canModerateMessages}
                  editRemovedIds={editRemovedIds}
                  editFiles={editFiles}
                  editSaving={savingEdit}
                  onEditTextChange={setEditText}
                  onStartEdit={() => startEdit(m)}
                  onCancelEdit={cancelEdit}
                  onSaveEdit={() => saveEdit(m)}
                  onToggleRemoveAttachment={toggleRemoveAttachment}
                  onAddEditFiles={(files) =>
                    addEditFiles(
                      files,
                      MAX_ATTACHMENTS_PER_MESSAGE -
                        ((m.attachments?.length ?? 0) -
                          editRemovedIds.length +
                          editFiles.length),
                    )
                  }
                  onRemoveEditFile={removeEditFile}
                  onDelete={() => setDeleteTarget(m)}
                  onAcceptInvite={() => handleAcceptInvite(m)}
                  onDeclineInvite={() => handleDeclineInvite(m)}
                  onOpenInviteServer={() =>
                    m.inviteServerId && onGoToServer?.(m.inviteServerId)
                  }
                />
                </React.Fragment>
              );
            })}
          </div>
        )}
      </div>
      </div>
      {!callMode && dm && (dmBlocked || dmBlockedBy) ? (
        <div className="composer composer-blocked">
          <Icon name="info" size={15} />
          <span>{dmBlocked ? t("blockedComposer") : t("blockedByComposer")}</span>
          {dmBlocked && (
            <button
              type="button"
              className="btn"
              onClick={() => void useBlockStore.getState().unblock(dm.friendId).catch(() => {})}
            >
              {t("unblockUser")}
            </button>
          )}
        </div>
      ) : !callMode && (
      <form className="composer" onSubmit={handleSend}>
        {pendingFiles.length > 0 && (
          <div className="attach-chips">
            {pendingFiles.map((p) => (
              <div
                key={p.localId}
                className={"attach-chip" + (p.error ? " error" : "")}
              >
                {!p.error && !p.attachment && (
                  <span
                    className="attach-chip-progress"
                    style={{ width: `${Math.round(p.progress * 100)}%` }}
                  />
                )}
                <Icon
                  name={p.file.type.startsWith("image/") ? "camera" : "pin"}
                  size={13}
                />
                <span className="attach-chip-name">{p.file.name}</span>
                <span className="attach-chip-size">
                  {p.error
                    ? i18n.t("attachmentFailed", { ns: "chat" })
                    : formatFileSize(p.file.size)}
                </span>
                <button
                  type="button"
                  className="attach-chip-remove"
                  onClick={() => removePendingFile(p.localId)}
                >
                  <Icon name="x" size={11} />
                </button>
              </div>
            ))}
          </div>
        )}
        <div className="composer-wrap">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            style={{ display: "none" }}
            onChange={(e) => {
              if (e.target.files) addFiles(e.target.files);
              e.target.value = "";
            }}
          />
          <button
            type="button"
            className="attach-btn"
            title={i18n.t("attachFile", { ns: "chat" })}
            disabled={sending}
            onClick={() => fileInputRef.current?.click()}
          >
            <Icon name="paperclip" size={17} />
          </button>
          {mentionQuery && mentionSuggestions.length > 0 && (
            <div className="mention-pop">
              {mentionSuggestions.map((p, i) => (
                <button
                  type="button"
                  key={p.id}
                  className={
                    "mention-pop-item" + (i === mentionIndex ? " active" : "")
                  }
                  onMouseDown={(e) => {
                    e.preventDefault();
                    applyMention(p.name);
                  }}
                  onMouseEnter={() => setMentionIndex(i)}
                >
                  <UserAvatar
                    userId={p.id}
                    name={p.name}
                    style={{
                      width: 18,
                      height: 18,
                      borderRadius: "50%",
                      background: colorForId(p.id),
                      color: "#fff",
                      fontSize: 9,
                      fontWeight: 700,
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  />
                  <span className="mention-pop-name">{p.name}</span>
                </button>
              ))}
            </div>
          )}
          <input
            ref={composerRef}
            placeholder={placeholder}
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              if (composerError) setComposerError(null);
              updateMentionQuery(e.target.value, e.target.selectionStart);
            }}
            onKeyDown={(e) => {
              if (
                text === "" &&
                !mentionQuery &&
                !e.nativeEvent.isComposing &&
                !e.nativeEvent.repeat &&
                eventMatches(e.nativeEvent, bindingFor("editLastMessage") ?? "")
              ) {
                const last = [...messages]
                  .reverse()
                  .find(
                    (m) =>
                      m.authorId === currentUserId &&
                      !m.callStatus &&
                      !m.inviteServerId &&
                      !m.system,
                  );
                if (last) {
                  e.preventDefault();
                  startEdit(last);
                }
                return;
              }
              if (!mentionQuery || mentionSuggestions.length === 0) return;
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setMentionIndex((i) => (i + 1) % mentionSuggestions.length);
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setMentionIndex(
                  (i) =>
                    (i - 1 + mentionSuggestions.length) %
                    mentionSuggestions.length,
                );
              } else if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                const pick =
                  mentionSuggestions[mentionIndex] ?? mentionSuggestions[0];
                applyMention(pick.name);
              } else if (e.key === "Escape") {
                e.preventDefault();
                setMentionQuery(null);
              }
            }}
            onBlur={() => setMentionQuery(null)}
            onSelect={(e) =>
              updateMentionQuery(
                e.currentTarget.value,
                e.currentTarget.selectionStart,
              )
            }
            onPaste={(e) => {
              const files = Array.from(e.clipboardData?.files ?? []);
              if (files.length > 0) {
                e.preventDefault();
                addFiles(files);
              }
            }}
            autoFocus
            disabled={sending}
          />
          <button
            type="submit"
            className={
              "send-btn" +
              (text.trim() || pendingFiles.some((p) => p.attachment)
                ? " enabled"
                : "")
            }
            aria-label={i18n.t("send", { ns: "chat" })}
            disabled={sending}
          >
            <Icon name="send" size={14} />
          </button>
        </div>
        {composerError && <div className="composer-error">{composerError}</div>}
      </form>
      )}
      {deleteTarget && (
        <ConfirmModal
          title={i18n.t("deleteMessage", { ns: "chat" })}
          subtitle={i18n.t("deleteMessageConfirm", { ns: "chat" })}
          confirmLabel={i18n.t("delete", { ns: "common" })}
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={confirmDelete}
        />
      )}
    </>
  );
};

export default ChatView;
