import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import CallJoinBanner from "./CallJoinBanner";
import { useCallStore } from "../../store/callStore";
import ConfirmModal from "./ConfirmModal";
import { channelAPI } from "../../api/channel";
import { dmAPI } from "../../api/dm";
import {
  MAX_ATTACHMENTS_PER_MESSAGE,
  MAX_ATTACHMENT_BYTES,
  attachmentAPI,
} from "../../api/attachment";
import { userAPI } from "../../api/user";
import { useReadReceiptStore } from "../../store/readReceiptStore";
import { Attachment, ChannelMessage } from "../../types/channel";
import { Server } from "../../types/server";
import { ApiError } from "../../utils/apiError";
import { colorForId, glyphFor } from "../../utils/avatar";
import {
  useWsChannelMessages,
  useWsDmMessages,
} from "../../services/useWebSocketMessages";
import i18n from "../../i18n";

const mergeWithLive = (
  history: ChannelMessage[],
  prev: ChannelMessage[],
): ChannelMessage[] => {
  const ids = new Set(history.map((m) => m.id));
  const extras = prev.filter((m) => !ids.has(m.id));
  if (extras.length === 0) return history;
  return [...history, ...extras].sort((a, b) =>
    a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0,
  );
};

const sameDay = (a: Date, b: Date): boolean =>
  a.getFullYear() === b.getFullYear() &&
  a.getMonth() === b.getMonth() &&
  a.getDate() === b.getDate();

const formatDayDivider = (date: Date): string => {
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  if (sameDay(date, today)) return i18n.t("today", { ns: "chat", defaultValue: "Today" });
  if (sameDay(date, yesterday)) return i18n.t("yesterday", { ns: "chat", defaultValue: "Yesterday" });
  return date.toLocaleDateString(i18n.language, {
    month: "long",
    day: "numeric",
    year: "numeric",
  });
};

const PAGE_LIMIT = 50;
const MAX_CONTENT = 2000;

const splitForSend = (text: string, maxLen: number): string[] => {
  const chunks: string[] = [];
  let remaining = text;
  while (remaining.length > maxLen) {
    const slice = remaining.slice(0, maxLen);
    const nl = slice.lastIndexOf("\n");
    let cut = maxLen;
    if (nl >= maxLen - 200 && nl > 0) {
      cut = nl + 1;
    } else {
      const sp = slice.lastIndexOf(" ");
      if (sp >= maxLen - 100 && sp > 0) cut = sp + 1;
    }
    const piece = remaining.slice(0, cut).trimEnd();
    if (piece.length > 0) chunks.push(piece);
    remaining = remaining.slice(cut).trimStart();
  }
  const tail = remaining.trim();
  if (tail.length > 0) chunks.push(tail);
  return chunks;
};

interface ChannelMeta {
  id: string;
  name: string;
  topic?: string;
  type: "text";
}

interface DmMeta {
  friendId: string;
  name: string;
  initials: string;
  color: string;
  online: boolean;
}

interface PendingFile {
  localId: string;
  file: File;
  progress: number;
  attachment?: Attachment;
  error?: boolean;
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const formatCallDuration = (seconds: number): string => {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
};

interface ChatViewProps {
  channel?: ChannelMeta;
  serverId?: string;
  currentUserId?: string;
  dm?: DmMeta;
  membersOpen?: boolean;
  servers?: Server[];
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
  onRemoveFriend?: () => void;
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
  onOpenChannelSettings,
  onRemoveFriend,
  onGoToServer,
  topSlot,
  callMode,
}) => {
  const { t } = useTranslation("chat");
  const [messages, setMessages] = React.useState<ChannelMessage[]>([]);
  const [pendingFiles, setPendingFiles] = React.useState<PendingFile[]>([]);
  const [dragOver, setDragOver] = React.useState(false);
  const fileInputRef = React.useRef<HTMLInputElement>(null);
  const cancelledUploadsRef = React.useRef<Set<string>>(new Set());

  const addFiles = React.useCallback((incoming: File[] | FileList) => {
    const files = Array.from(incoming);
    if (files.length === 0) return;
    setPendingFiles((prev) => {
      const room = MAX_ATTACHMENTS_PER_MESSAGE - prev.length;
      const accepted = files.slice(0, Math.max(0, room));
      const additions: PendingFile[] = accepted.map((file) => ({
        localId: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
        file,
        progress: 0,
        error: file.size > MAX_ATTACHMENT_BYTES || file.size === 0,
      }));
      for (const p of additions) {
        if (p.error) continue;
        void attachmentAPI
          .upload(p.file, (fraction) => {
            setPendingFiles((cur) =>
              cur.map((x) =>
                x.localId === p.localId ? { ...x, progress: fraction } : x,
              ),
            );
          })
          .then((attachment) => {
            if (cancelledUploadsRef.current.delete(p.localId)) {
              void attachmentAPI.delete(attachment.id).catch(() => {});
              return;
            }
            setPendingFiles((cur) =>
              cur.map((x) =>
                x.localId === p.localId
                  ? { ...x, attachment, progress: 1 }
                  : x,
              ),
            );
          })
          .catch(() => {
            cancelledUploadsRef.current.delete(p.localId);
            setPendingFiles((cur) =>
              cur.map((x) =>
                x.localId === p.localId ? { ...x, error: true } : x,
              ),
            );
          });
      }
      return [...prev, ...additions];
    });
  }, []);

  const discardPendingFile = React.useCallback((target: PendingFile) => {
    if (target.attachment) {
      void attachmentAPI.delete(target.attachment.id).catch(() => {});
    } else if (!target.error) {
      cancelledUploadsRef.current.add(target.localId);
    }
  }, []);

  const pendingFilesRef = React.useRef<PendingFile[]>([]);
  pendingFilesRef.current = pendingFiles;

  React.useEffect(() => {
    return () => {
      for (const p of pendingFilesRef.current) discardPendingFile(p);
    };
  }, [discardPendingFile]);

  const removePendingFile = (localId: string) =>
    setPendingFiles((prev) => {
      const target = prev.find((p) => p.localId === localId);
      if (target) discardPendingFile(target);
      return prev.filter((p) => p.localId !== localId);
    });

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
      }) => {
        setMessages((prev) =>
          prev.map((x) =>
            x.id === m.id
              ? { ...x, content: m.newContent, editedAt: m.occurredAt }
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
      }) => {
        setMessages((prev) =>
          prev.map((x) =>
            x.id === m.id
              ? { ...x, content: m.newContent, editedAt: m.occurredAt }
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
        editMessage: (messageId: string, content: string) =>
          dmAPI.editMessage(messageId, content),
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
        editMessage: (messageId: string, content: string) =>
          channelAPI.editMessage(serverId, channel.id, messageId, content),
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
    setPendingFiles((prev) => {
      for (const p of prev) discardPendingFile(p);
      return [];
    });
    setEditingId(null);
    setEditText("");
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
          setAccessError(
            apiErr.message || "You are no longer friends with this user.",
          );
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
    reseedChannelReads();
    try {
      const res = await messenger.listMessages({ limit: PAGE_LIMIT });
      const ordered = [...res.messages].reverse();
      setMessages((prev) => mergeWithLive(ordered, prev));
      setHasMore(res.messages.length >= PAGE_LIMIT);
    } catch (err) {
      console.warn("[ChatView] failed to refresh history after join:", err);
    }
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
  }, [searchQuery, searchOpen, convId]);

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

    const remaining = [...chunks];
    const updateComposer = () => setText(remaining.join("\n\n"));

    const sentFiles = pendingFiles.filter((p) => p.attachment);
    const sentLocalIds = new Set(sentFiles.map((p) => p.localId));
    const sentAttachments = sentFiles.map((p) => p.attachment as Attachment);
    let idsToAttach: string[] | undefined =
      readyIds.length > 0 ? readyIds : undefined;
    const sendChunk = async (chunk: string) => {
      const msg = await sendOnce(chunk, idsToAttach);
      if (idsToAttach) {
        msg.attachments = msg.attachments ?? sentAttachments;
        idsToAttach = undefined;
        setPendingFiles((prev) =>
          prev.filter((p) => !sentLocalIds.has(p.localId)),
        );
      }
      setMessages((prev) =>
        prev.some((x) => x.id === msg.id) ? prev : [...prev, msg],
      );
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
              setComposerError(joinApi?.message || i18n.t("couldNotJoin", { ns: "chat" }));
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
        setComposerError(i18n.t("messageCantBeEmpty", { ns: "chat" }));
      } else if (code === "MESSAGE_CONTENT_TOO_LONG") {
        setComposerError(i18n.t("chunkTooLong", { ns: "chat" }));
      } else {
        console.warn("[ChatView] send failed:", err);
        setComposerError(apiErr?.message || i18n.t("couldNotSend", { ns: "chat" }));
      }
    } finally {
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
    setEditingId(msg.id);
    setEditText(msg.content);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditText("");
  };

  const saveEdit = async (msg: ChannelMessage) => {
    if (!messenger) return;
    const v = editText.trim();
    if (!v) {
      cancelEdit();
      return;
    }
    if (v === msg.content) {
      cancelEdit();
      return;
    }
    if (v.length > MAX_CONTENT) return;
    try {
      await messenger.editMessage(msg.id, v);
      const editedAt = new Date().toISOString();
      setMessages((prev) =>
        prev.map((m) => (m.id === msg.id ? { ...m, content: v, editedAt } : m)),
      );
      cancelEdit();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (
        apiErr?.code === "MESSAGE_CONTENT_UNCHANGED" ||
        apiErr?.code === "DM_CONTENT_UNCHANGED"
      ) {
        cancelEdit();
        return;
      }
      console.warn("[ChatView] edit failed:", err);
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
        membersOpen={membersOpen}
        searchActive={searchOpen}
        onToggleSearch={toggleSearch}
        onToggleMembers={onToggleMembers}
        onOpenChannelSettings={onOpenChannelSettings}
        onRemoveFriend={onRemoveFriend}
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
        className={"chat-body" + (dragOver ? " drag-over" : "")}
        ref={bodyRef}
        onScroll={handleScroll}
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
        style={callMode ? { display: "none" } : undefined}
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
                  const displayName = isMine
                    ? i18n.t("you", { ns: "chat" })
                    : (m.authorName ?? m.authorId);
                  const time = new Date(m.createdAt).toLocaleString([], {
                    month: "short",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                  });
                  const initial = displayName.slice(0, 1).toUpperCase();
                  return (
                    <button
                      key={m.id}
                      type="button"
                      className="search-result"
                      onClick={() => handleJumpToMessage(m)}
                    >
                      <div
                        className="ava"
                        style={{ background: colorForId(m.authorId) }}
                      >
                        {initial}
                      </div>
                      <div className="body">
                        <div className="head">
                          <b>{displayName}</b>
                          <time>{time}</time>
                        </div>
                        <div className="content">{m.content}</div>
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
              const cont = sameAuthor && within && !newDay;
              const isMine = !!currentUserId && m.authorId === currentUserId;
              const displayName = isMine
                ? "You"
                : (m.authorName ?? m.authorId);
              return (
                <React.Fragment key={m.id}>
                  {newDay && (
                    <div className="day-divider">
                      <span>{formatDayDivider(curDate)}</span>
                    </div>
                  )}
                <Message
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
                  onEditTextChange={setEditText}
                  onStartEdit={() => startEdit(m)}
                  onCancelEdit={cancelEdit}
                  onSaveEdit={() => saveEdit(m)}
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
      {!callMode && (
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
            <Icon name="plus" size={16} />
          </button>
          <input
            ref={composerRef}
            placeholder={placeholder}
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              if (composerError) setComposerError(null);
            }}
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

interface ChannelHeaderProps {
  channel?: ChannelMeta;
  dm?: DmMeta;
  membersOpen?: boolean;
  searchActive?: boolean;
  onToggleSearch?: () => void;
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
  onRemoveFriend?: () => void;
}

const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  membersOpen,
  searchActive,
  onToggleSearch,
  onToggleMembers,
  onOpenChannelSettings,
  onRemoveFriend,
}) => {
  const [bellOpen, setBellOpen] = React.useState(false);
  const [notifLevel, setNotifLevel] = React.useState<"all" | "mentions" | "none">("all");
  const bellRef = React.useRef<HTMLDivElement>(null);
  const bellButtonRef = React.useRef<HTMLButtonElement>(null);

  React.useEffect(() => {
    if (!bellOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (bellRef.current && !bellRef.current.contains(e.target as Node)) {
        setBellOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [bellOpen]);

  React.useEffect(() => {
    if (!bellOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setBellOpen(false);
        bellButtonRef.current?.focus();
        return;
      }
      if (e.key !== "Tab" || !bellRef.current) return;
      const focusables = bellRef.current.querySelectorAll<HTMLElement>(
        'button:not([tabindex="-1"])',
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bellOpen]);

  if (dm) {
    return (
      <header className="main-header">
        <span className="ch-ico">
          <span
            style={{
              width: 20,
              height: 20,
              borderRadius: "50%",
              background: dm.color,
              color: "#fff",
              fontSize: 10,
              fontWeight: 700,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            {dm.initials}
          </span>
        </span>
        <span className="ch-name">{dm.name}</span>
        <span className="sep" />
        <span className="topic">{dm.online ? i18n.t("activeNow", { ns: "chat" }) : i18n.t("offline", { ns: "chat" })}</span>
        <div className="right">
          <button
            type="button"
            className="icon-btn"
            title={i18n.t("voiceCallInvite", { ns: "chat" })}
            onClick={() => {
              if (!dm?.friendId) return;
              const cs = useCallStore.getState();
              const inCallWithFriend =
                !!cs.activeCallId &&
                (cs.peerId === dm.friendId ||
                  dm.friendId in cs.participants ||
                  cs.ringing.includes(dm.friendId));
              if (inCallWithFriend) {
                cs.setFocused(true);
                return;
              }
              cs.start([dm.friendId]).catch((e) =>
                console.warn("[call] start failed:", e),
              );
            }}
          >
            <Icon name="phone" size={16} />
          </button>
          {onToggleSearch && (
            <button
              type="button"
              className={"icon-btn" + (searchActive ? " on" : "")}
              title={searchActive ? i18n.t("closeSearch", { ns: "chat" }) : i18n.t("searchWithUser", { ns: "chat", user: dm.name })}
              onClick={onToggleSearch}
            >
              <Icon name="search" size={15} />
            </button>
          )}
          {onRemoveFriend && (
            <button
              type="button"
              className="icon-btn"
              title={i18n.t("removeFriend", { ns: "chat" })}
              onClick={onRemoveFriend}
            >
              <Icon name="user-check" size={16} />
            </button>
          )}
          <button type="button" className="icon-btn" title={i18n.t("settings", { ns: "sidebar" })}>
            <Icon name="settings" size={16} />
          </button>
        </div>
      </header>
    );
  }

  if (!channel) return null;

  return (
    <header className="main-header">
      <span className="ch-ico">
        <Icon name="hash" size={16} />
      </span>
      <span className="ch-name">{channel.name}</span>
      {channel.topic && (
        <>
          <span className="sep" />
          <span className="topic">{channel.topic}</span>
        </>
      )}
      <div className="right">
        <div className="popover-wrap" ref={bellRef}>
          <button
            ref={bellButtonRef}
            type="button"
            className={"icon-btn" + (bellOpen ? " on" : "")}
            title={i18n.t("settings", { ns: "sidebar" })}
            onClick={() => setBellOpen((o) => !o)}
          >
            <Icon name="bell" size={15} />
            {notifLevel !== "all" && (
              <span
                className="nb-dot"
                style={{ background: notifLevel === "none" ? "#ff453a" : "#ff9f0a" }}
              />
            )}
          </button>
          {bellOpen && (
            <div className="popover" onMouseDown={(e) => e.stopPropagation()}>
              <div className="pop-head">{i18n.t("notifications.title", { ns: "chat", channel: channel.name })}</div>
              {(
                [
                  { k: "all", ttl: i18n.t("notifications.all", { ns: "chat" }), sub: i18n.t("notifications.allHint", { ns: "chat" }) },
                  { k: "mentions", ttl: i18n.t("notifications.mentions", { ns: "chat" }), sub: i18n.t("notifications.mentionsHint", { ns: "chat" }) },
                  { k: "none", ttl: i18n.t("notifications.nothing", { ns: "chat" }), sub: i18n.t("notifications.nothingHint", { ns: "chat" }) },
                ] as const
              ).map((opt) => (
                <button
                  type="button"
                  key={opt.k}
                  className="pop-item"
                  onClick={() => {
                    setNotifLevel(opt.k);
                    setBellOpen(false);
                  }}
                >
                  <span className="check">
                    {notifLevel === opt.k && <Icon name="check" size={14} />}
                  </span>
                  <span className="body">
                    <div className="ttl">{opt.ttl}</div>
                    <div className="sub">{opt.sub}</div>
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
        {onToggleMembers && (
          <button
            type="button"
            className={"icon-btn" + (membersOpen ? " on" : "")}
            title={membersOpen ? "Hide members" : "Show members"}
            onClick={onToggleMembers}
          >
            <Icon name="users" size={15} />
          </button>
        )}
        {onToggleSearch && (
          <button
            type="button"
            className={"icon-btn" + (searchActive ? " on" : "")}
            title={searchActive ? i18n.t("closeSearch", { ns: "chat" }) : i18n.t("search", { ns: "common" })}
            onClick={onToggleSearch}
          >
            <Icon name="search" size={15} />
          </button>
        )}
        {onOpenChannelSettings && (
          <button
            type="button"
            className="icon-btn"
            title={i18n.t("channelSettings.title", { ns: "modals" })}
            onClick={onOpenChannelSettings}
          >
            <Icon name="settings" size={15} />
          </button>
        )}
      </div>
    </header>
  );
};

const IMAGE_CACHE_LIMIT = 40;
interface CachedImage {
  url: string;
  refs: number;
}
const imageUrlCache = new Map<string, CachedImage>();

const evictImageUrls = () => {
  if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) return;
  for (const [id, entry] of imageUrlCache) {
    if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) break;
    if (entry.refs > 0) continue;
    imageUrlCache.delete(id);
    URL.revokeObjectURL(entry.url);
  }
};

const cacheImageUrl = (id: string, url: string): string => {
  const entry = imageUrlCache.get(id);
  if (entry) {
    if (entry.url === url) return url;
    if (entry.refs > 0) {
      URL.revokeObjectURL(url);
      return entry.url;
    }
    imageUrlCache.delete(id);
    URL.revokeObjectURL(entry.url);
  }
  imageUrlCache.set(id, { url, refs: 0 });
  evictImageUrls();
  return url;
};

const pendingImageDownloads = new Map<string, Promise<string>>();

const downloadImageUrl = (id: string): Promise<string> => {
  const existing = pendingImageDownloads.get(id);
  if (existing) return existing;
  const promise = attachmentAPI
    .download(id)
    .then((blob) => cacheImageUrl(id, URL.createObjectURL(blob)))
    .finally(() => {
      pendingImageDownloads.delete(id);
    });
  pendingImageDownloads.set(id, promise);
  return promise;
};

const acquireImageUrl = (id: string): string | null => {
  const entry = imageUrlCache.get(id);
  if (!entry) return null;
  imageUrlCache.delete(id);
  imageUrlCache.set(id, entry);
  entry.refs += 1;
  return entry.url;
};

const retainImageUrl = (id: string, url: string) => {
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url) entry.refs += 1;
};

const releaseImageUrl = (id: string, url: string | null) => {
  if (!url) return;
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url && entry.refs > 0) {
    entry.refs -= 1;
    if (entry.refs === 0) evictImageUrls();
  }
};

const discardBrokenImageUrl = (id: string, url: string) => {
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url) {
    if (entry.refs > 0) entry.refs -= 1;
    imageUrlCache.delete(id);
  }
  URL.revokeObjectURL(url);
};

const AttachmentItem: React.FC<{ attachment: Attachment }> = ({
  attachment,
}) => {
  const isImage = attachment.contentType.startsWith("image/");
  const [url, setUrl] = React.useState<string | null>(() =>
    acquireImageUrl(attachment.id),
  );
  const [failed, setFailed] = React.useState(false);
  const [viewerOpen, setViewerOpen] = React.useState(false);
  const [visible, setVisible] = React.useState(false);
  const [retryTick, setRetryTick] = React.useState(0);
  const attemptsRef = React.useRef(0);
  const containerRef = React.useRef<HTMLButtonElement>(null);
  const urlRef = React.useRef(url);
  urlRef.current = url;

  React.useEffect(() => {
    return () => releaseImageUrl(attachment.id, urlRef.current);
  }, [attachment.id]);

  React.useEffect(() => {
    if (!isImage || url || visible) return;
    const node = containerRef.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "300px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [attachment.id, isImage, url, visible]);

  React.useEffect(() => {
    if (!isImage || url || !visible) return;
    let cancelled = false;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;
    downloadImageUrl(attachment.id)
      .then((objectUrl) => {
        if (!cancelled) {
          attemptsRef.current = 0;
          retainImageUrl(attachment.id, objectUrl);
          setUrl(objectUrl);
        }
      })
      .catch(() => {
        if (cancelled) return;
        if (attemptsRef.current < 3) {
          const delay = 1500 * 2 ** attemptsRef.current;
          attemptsRef.current += 1;
          retryTimer = setTimeout(() => setRetryTick((t) => t + 1), delay);
        } else {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
      if (retryTimer) clearTimeout(retryTimer);
    };
  }, [attachment.id, isImage, url, visible, retryTick]);

  const handleDownload = () => {
    void attachmentAPI
      .download(attachment.id)
      .then((blob) => {
        const objectUrl = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = objectUrl;
        a.download = attachment.fileName;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        setTimeout(() => URL.revokeObjectURL(objectUrl), 10_000);
      })
      .catch(() => {});
  };

  if (isImage && !failed) {
    return (
      <>
        <button
          ref={containerRef}
          type="button"
          className="msg-attach-image"
          onClick={() => url && setViewerOpen(true)}
        >
          {url ? (
            <img
              src={url}
              alt={attachment.fileName}
              loading="lazy"
              onError={() => {
                if (url) discardBrokenImageUrl(attachment.id, url);
                setUrl(null);
                setFailed(true);
              }}
            />
          ) : (
            <span className="msg-attach-image-loading">
              <span className="loading-spinner" />
            </span>
          )}
        </button>
        {viewerOpen && url && (
          <div
            className="image-viewer-overlay"
            onClick={() => setViewerOpen(false)}
          >
            <img src={url} alt={attachment.fileName} />
          </div>
        )}
      </>
    );
  }

  return (
    <button type="button" className="msg-attach-file" onClick={handleDownload}>
      <span className="msg-attach-file-icon">
        <Icon name="download" size={16} />
      </span>
      <span className="msg-attach-file-info">
        <span className="msg-attach-file-name">{attachment.fileName}</span>
        <span className="msg-attach-file-size">
          {formatFileSize(attachment.sizeBytes)}
        </span>
      </span>
    </button>
  );
};

const ReadTicks: React.FC<{ read: boolean }> = ({ read }) => (
  <svg
    className="rc-icon"
    viewBox="0 0 15 11"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {read ? (
      <>
        <path d="M1 6.2 L3.5 8.7 L7.8 3" />
        <path d="M5.4 6.2 L7.9 8.7 L12.2 3" />
      </>
    ) : (
      <path d="M2.4 6.2 L5 8.7 L9.6 3" />
    )}
  </svg>
);

interface MessageProps {
  msg: ChannelMessage;
  displayName: string;
  continued: boolean;
  isMine: boolean;
  dmStyle: boolean;
  receipt: "sent" | "read" | null;
  editing: boolean;
  highlighted: boolean;
  editText: string;
  inviteIsMember: boolean;
  onEditTextChange: (v: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onDelete: () => void;
  onAcceptInvite: () => void;
  onDeclineInvite: () => void;
  onOpenInviteServer: () => void;
}

const Message: React.FC<MessageProps> = ({
  msg,
  displayName,
  continued,
  isMine,
  dmStyle,
  receipt,
  editing,
  highlighted,
  editText,
  inviteIsMember,
  onEditTextChange,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onDelete,
  onAcceptInvite,
  onDeclineInvite,
  onOpenInviteServer,
}) => {
  const created = new Date(msg.createdAt);
  const time = created.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  const initial = displayName.slice(0, 1).toUpperCase();
  return (
    <div
      data-msg-id={msg.id}
      className={
        "msg bubble " +
        (isMine ? "mine" : "theirs") +
        (dmStyle ? " dm" : "") +
        (continued ? " cont" : "") +
        (highlighted ? " highlight" : "")
      }
    >
      <div className="ava" style={{ background: colorForId(msg.authorId) }}>
        {initial}
      </div>
      <div className="body">
        {!continued && (
          <div className="head">
            <b>{displayName}</b>
            <time>{time}</time>
          </div>
        )}
        {msg.callStatus ? (
          <div className={"dm-call-card " + msg.callStatus}>
            <span className="dm-call-icon">
              <Icon name="phone" size={15} />
            </span>
            <div className="dm-call-info">
              <div className="dm-call-title">
                {i18n.t("voiceCall", { ns: "call" })}
              </div>
              <div className="dm-call-sub">
                {msg.callStatus === "completed"
                  ? formatCallDuration(msg.callDurationSeconds ?? 0)
                  : msg.callStatus === "declined"
                    ? i18n.t("callDeclined", { ns: "call" })
                    : isMine
                      ? i18n.t("callNoAnswer", { ns: "call" })
                      : i18n.t("callMissed", { ns: "call" })}
              </div>
            </div>
          </div>
        ) : msg.inviteServerId ? (
          <div className="dm-invite-card">
            <div className="dm-invite-card-label">
              {i18n.t("invite.label", { ns: "chat" })}
            </div>
            <div className="dm-invite-card-row">
              <span
                className="dm-invite-card-glyph"
                style={{ background: colorForId(msg.inviteServerId) }}
              >
                {glyphFor(msg.inviteServerName ?? "?")}
              </span>
              <div className="dm-invite-card-info">
                <div className="dm-invite-card-name">
                  {msg.inviteServerName ??
                    i18n.t("invite.serverFallback", { ns: "chat" })}
                </div>
                <div className="dm-invite-card-sub">
                  {isMine
                    ? i18n.t("invite.youSent", { ns: "chat" })
                    : i18n.t("invite.invitedYou", {
                        ns: "chat",
                        name: displayName,
                      })}
                </div>
              </div>
              {inviteIsMember ? (
                <button
                  type="button"
                  className="dm-invite-card-btn joined"
                  onClick={onOpenInviteServer}
                >
                  {i18n.t("invite.open", { ns: "chat" })}
                </button>
              ) : isMine ? null : (
                <div className="dm-invite-card-actions">
                  <button
                    type="button"
                    className="dm-invite-card-btn"
                    onClick={onAcceptInvite}
                  >
                    {i18n.t("invite.join", { ns: "chat" })}
                  </button>
                  <button
                    type="button"
                    className="dm-invite-card-btn decline"
                    onClick={onDeclineInvite}
                  >
                    {i18n.t("invite.decline", { ns: "chat" })}
                  </button>
                </div>
              )}
            </div>
          </div>
        ) : editing ? (
          <div className="msg-edit">
            <input
              autoFocus
              value={editText}
              maxLength={MAX_CONTENT}
              onChange={(e) => onEditTextChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  onCancelEdit();
                } else if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSaveEdit();
                }
              }}
            />
            <div className="msg-edit-hint">
              escape to cancel · enter to save
            </div>
          </div>
        ) : (
          <div className="txt">
            {!dmStyle && !isMine && !continued && (
              <div
                className="msg-author"
                style={{ color: colorForId(msg.authorId) }}
              >
                {displayName}
              </div>
            )}
            {msg.content}
            {(msg.attachments?.length ?? 0) > 0 && (
              <div className="msg-attachments">
                {(msg.attachments ?? []).map((a) => (
                  <AttachmentItem key={a.id} attachment={a} />
                ))}
              </div>
            )}
            {msg.editedAt && <span className="msg-edited"> ({i18n.t("edited", { ns: "chat" })})</span>}
            <span className="msg-meta">
              <time className="msg-time">{time}</time>
                {receipt && (
                  <span
                    className={"msg-receipt" + (receipt === "read" ? " read" : "")}
                    title={i18n.t(receipt === "read" ? "seen" : "delivered", {
                      ns: "chat",
                    })}
                    aria-hidden="true"
                  >
                    <ReadTicks read={receipt === "read"} />
                  </span>
              )}
            </span>
          </div>
        )}
      </div>
      {isMine && !editing && !msg.inviteServerId && (
        <div className="msg-actions">
          <button
            type="button"
            className="icon-btn"
            title="Edit"
            onClick={onStartEdit}
          >
            <Icon name="edit" size={13} />
          </button>
          <button
            type="button"
            className="icon-btn"
            title="Delete"
            onClick={onDelete}
          >
            <Icon name="trash" size={13} />
          </button>
        </div>
      )}
    </div>
  );
};

export default ChatView;
