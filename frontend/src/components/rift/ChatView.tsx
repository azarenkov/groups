import React from "react";
import { createPortal } from "react-dom";
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
import { colorForId } from "../../utils/avatar";
import { ServerLogo, UserAvatar } from "./Avatar";
import {
  ParsedLink,
  URL_REGEX,
  domainOf,
  extractLinks,
} from "../../utils/links";
import {
  useWsChannelMessages,
  useWsDmMessages,
} from "../../services/useWebSocketMessages";
import i18n from "../../i18n";
import { useUserNamesStore } from "../../store/userNamesStore";
import { useAuthStore } from "../../store/authStore";
import { refetchUnread } from "../../store/unreadStore";
import { useNotificationPrefsStore } from "../../store/notificationPrefsStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { homeAPI } from "../../api/home";
import { profileVersion, useUserProfileStore } from "../../store/userProfileStore";
import { useProfileModalStore } from "../../store/profileModalStore";
import {
  PERM_MANAGE_MESSAGES,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";
import { MENTION_REGEX, activeMentionQuery } from "../../utils/mentions";
import type { ChannelParticipant } from "../../types/channel";

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
  isFriend?: boolean;
  friendship?: "accepted" | "pending_outgoing" | "pending_incoming" | "none";
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
  onRemoveFriend,
  onGoToServer,
  topSlot,
  callMode,
}) => {
  const { t } = useTranslation("chat");
  const [messages, setMessages] = React.useState<ChannelMessage[]>([]);
  const messagesRef = React.useRef<ChannelMessage[]>([]);
  messagesRef.current = messages;
  const [pendingFiles, setPendingFiles] = React.useState<PendingFile[]>([]);
  const [dragOver, setDragOver] = React.useState(false);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);
  const fileInputRef = React.useRef<HTMLInputElement>(null);
  const cancelledUploadsRef = React.useRef<Set<string>>(new Set());
  const pendingFilesRef = React.useRef<PendingFile[]>([]);
  pendingFilesRef.current = pendingFiles;
  const inFlightSendRef = React.useRef<Set<string>>(new Set());

  const addFiles = React.useCallback((incoming: File[] | FileList) => {
    const files = Array.from(incoming);
    if (files.length === 0) return;
    const room =
      MAX_ATTACHMENTS_PER_MESSAGE - pendingFilesRef.current.length;
    const accepted = files.slice(0, Math.max(0, room));
    const additions: PendingFile[] = accepted.map((file) => ({
      localId: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      file,
      progress: 0,
      error: file.size > MAX_ATTACHMENT_BYTES || file.size === 0,
    }));
    if (additions.length === 0) return;
    setPendingFiles((prev) => [...prev, ...additions]);
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
  }, []);

  const discardPendingFile = React.useCallback((target: PendingFile) => {
    if (target.attachment) {
      void attachmentAPI.delete(target.attachment.id).catch(() => {});
    } else if (!target.error) {
      cancelledUploadsRef.current.add(target.localId);
    }
  }, []);

  React.useEffect(() => {
    const inFlight = inFlightSendRef.current;
    return () => {
      for (const p of pendingFilesRef.current) {
        if (!inFlight.has(p.localId)) discardPendingFile(p);
      }
    };
  }, [discardPendingFile]);

  const removePendingFile = (localId: string) =>
    setPendingFiles((prev) => {
      const target = prev.find((p) => p.localId === localId);
      if (target) discardPendingFile(target);
      return prev.filter((p) => p.localId !== localId);
    });

  const [editFiles, setEditFiles] = React.useState<PendingFile[]>([]);
  const [editRemovedIds, setEditRemovedIds] = React.useState<string[]>([]);
  const editFilesRef = React.useRef<PendingFile[]>([]);
  editFilesRef.current = editFiles;

  const addEditFiles = React.useCallback(
    (incoming: File[] | FileList, room: number) => {
      const files = Array.from(incoming);
      if (files.length === 0) return;
      const accepted = files.slice(0, Math.max(0, room));
      const additions: PendingFile[] = accepted.map((file) => ({
        localId: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
        file,
        progress: 0,
        error: file.size > MAX_ATTACHMENT_BYTES || file.size === 0,
      }));
      if (additions.length === 0) return;
      setEditFiles((prev) => [...prev, ...additions]);
      for (const p of additions) {
        if (p.error) continue;
        void attachmentAPI
          .upload(p.file, (fraction) => {
            setEditFiles((cur) =>
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
            setEditFiles((cur) =>
              cur.map((x) =>
                x.localId === p.localId
                  ? { ...x, attachment, progress: 1 }
                  : x,
              ),
            );
          })
          .catch(() => {
            cancelledUploadsRef.current.delete(p.localId);
            setEditFiles((cur) =>
              cur.map((x) =>
                x.localId === p.localId ? { ...x, error: true } : x,
              ),
            );
          });
      }
    },
    [],
  );

  const removeEditFile = React.useCallback(
    (localId: string) =>
      setEditFiles((prev) => {
        const target = prev.find((p) => p.localId === localId);
        if (target) discardPendingFile(target);
        return prev.filter((p) => p.localId !== localId);
      }),
    [discardPendingFile],
  );

  const discardEditFiles = React.useCallback(() => {
    setEditFiles((prev) => {
      for (const p of prev) {
        if (!inFlightSendRef.current.has(p.localId)) discardPendingFile(p);
      }
      return [];
    });
    setEditRemovedIds([]);
  }, [discardPendingFile]);

  const toggleRemoveAttachment = React.useCallback((id: string) => {
    setEditRemovedIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }, []);

  React.useEffect(() => {
    const inFlight = inFlightSendRef.current;
    return () => {
      for (const p of editFilesRef.current) {
        if (!inFlight.has(p.localId)) discardPendingFile(p);
      }
    };
  }, [discardPendingFile]);

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
  const canModerateMessages =
    !!channel &&
    !!currentUserId &&
    (serverOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_MANAGE_MESSAGES) !== 0);

  const [mentionQuery, setMentionQuery] = React.useState<{
    query: string;
    start: number;
  } | null>(null);
  const [mentionIndex, setMentionIndex] = React.useState(0);
  const [participants, setParticipants] = React.useState<ChannelParticipant[]>([]);
  const participantsChannelRef = React.useRef<string | null>(null);

  const channelMemberCount = useReadReceiptStore((s) =>
    channel ? Object.keys(s.channelReads[channel.id] ?? {}).length : 0,
  );
  React.useEffect(() => {
    participantsChannelRef.current = null;
  }, [channelMemberCount]);

  const ensureParticipants = React.useCallback(() => {
    if (!channel || !serverId) return;
    if (participantsChannelRef.current === channel.id) return;
    const channelId = channel.id;
    participantsChannelRef.current = channelId;
    setParticipants([]);
    void (async () => {
      try {
        const all: ChannelParticipant[] = [];
        const seen = new Set<string>();
        let page = 1;
        let totalPages = 1;
        do {
          const res = await channelAPI.listParticipants(
            serverId,
            channelId,
            page,
            50,
          );
          for (const p of res.participants) {
            if (seen.has(p.id)) continue;
            seen.add(p.id);
            all.push(p);
          }
          totalPages = res.pagination?.totalPages ?? 1;
          page += 1;
        } while (
          page <= totalPages &&
          participantsChannelRef.current === channelId
        );
        if (participantsChannelRef.current === channelId) {
          setParticipants(all);
        }
      } catch {
        if (participantsChannelRef.current === channelId) {
          participantsChannelRef.current = null;
        }
      }
    })();
  }, [channel, serverId]);

  const updateMentionQuery = React.useCallback(
    (value: string, caret: number | null) => {
      if (!channel) {
        setMentionQuery(null);
        return;
      }
      const q = activeMentionQuery(value, caret ?? value.length);
      setMentionQuery(q);
      if (q) {
        setMentionIndex(0);
        ensureParticipants();
      }
    },
    [channel, ensureParticipants],
  );

  const mentionSuggestions = React.useMemo(() => {
    if (!mentionQuery) return [];
    const q = mentionQuery.query.toLowerCase();
    return participants
      .filter((p) => p.id !== currentUserId)
      .map((p) => ({ ...p, name: nameOverrides[p.id] ?? p.name }))
      .filter((p) => p.name.toLowerCase().startsWith(q))
      .slice(0, 8);
  }, [mentionQuery, participants, nameOverrides, currentUserId]);

  const applyMention = React.useCallback(
    (name: string) => {
      if (!mentionQuery) return;
      const el = composerRef.current;
      const caret = el?.selectionStart ?? text.length;
      const next =
        text.slice(0, mentionQuery.start) + "@" + name + " " + text.slice(caret);
      setText(next);
      setMentionQuery(null);
      const pos = mentionQuery.start + name.length + 2;
      requestAnimationFrame(() => {
        el?.focus();
        el?.setSelectionRange(pos, pos);
      });
    },
    [mentionQuery, text],
  );

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
    setParticipants([]);
    participantsChannelRef.current = null;
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
        membersOpen={membersOpen}
        searchActive={searchOpen}
        onToggleSearch={toggleSearch}
        onToggleMembers={onToggleMembers}
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
                : (nameOverrides[m.authorId] ?? m.authorName ?? m.authorId);
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
      {!callMode && dm && dm.isFriend === false && (
        <div className="composer-locked">
          <span className="composer-locked-text">
            {dm.friendship === "pending_outgoing"
              ? t("friendRequestPending", { user: dm.name })
              : dm.friendship === "pending_incoming"
                ? t("friendRequestIncoming", { user: dm.name })
                : t("notFriendsNotice", { user: dm.name })}
          </span>
          {dm.friendship === "pending_outgoing" ? (
            <button type="button" className="btn" disabled>
              <Icon name="check" size={13} />
              {t("friendRequestSent")}
            </button>
          ) : dm.friendship === "pending_incoming" ? (
            <button
              type="button"
              className="btn primary"
              onClick={() =>
                useFriendshipStore
                  .getState()
                  .accept(dm.friendId)
                  .catch((e) => setComposerError((e as ApiError)?.message || t("couldNotSend")))
              }
            >
              <Icon name="check" size={13} />
              {t("acceptFriendRequest")}
            </button>
          ) : (
            <button
              type="button"
              className="btn primary"
              onClick={() =>
                useFriendshipStore
                  .getState()
                  .sendRequest(dm.name)
                  .catch((e) => setComposerError((e as ApiError)?.message || t("couldNotSend")))
              }
            >
              <Icon name="user-plus" size={13} />
              {t("addFriend")}
            </button>
          )}
          {composerError && <div className="composer-error">{composerError}</div>}
        </div>
      )}
      {!callMode && !(dm && dm.isFriend === false) && (
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

interface ChannelHeaderProps {
  channel?: ChannelMeta;
  dm?: DmMeta;
  membersOpen?: boolean;
  searchActive?: boolean;
  onToggleSearch?: () => void;
  onToggleMembers?: () => void;
  onRemoveFriend?: () => void;
}

const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  membersOpen,
  searchActive,
  onToggleSearch,
  onToggleMembers,
  onRemoveFriend,
}) => {
  const [bellOpen, setBellOpen] = React.useState(false);
  const dmFriendIdForBio = dm?.friendId ?? null;
  const dmBio = useUserProfileStore((s) => (dmFriendIdForBio ? (s.bios[dmFriendIdForBio]?.bio ?? "") : ""));
  React.useEffect(() => {
    if (!dmFriendIdForBio) return;
    let cancelled = false;
    homeAPI
      .profile(dmFriendIdForBio)
      .then((p) => {
        if (cancelled) return;
        useUserProfileStore.getState().apply(dmFriendIdForBio, p.bio, profileVersion(p.profileUpdatedAt));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [dmFriendIdForBio]);
  const notifLevel = useNotificationPrefsStore((s) =>
    dm
      ? (s.dmLevels[dm.friendId] ?? "all")
      : channel
        ? (s.channelLevels[channel.id] ?? "all")
        : "all",
  );
  const bellRef = React.useRef<HTMLDivElement>(null);
  const bellButtonRef = React.useRef<HTMLButtonElement>(null);

  const applyNotifLevel = (level: "all" | "mentions" | "none") => {
    const prefs = useNotificationPrefsStore.getState();
    const action = dm
      ? level !== "mentions" && prefs.setDmLevel(dm.friendId, level)
      : channel
        ? prefs.setChannelLevel(channel.id, level)
        : null;
    if (action) {
      action.catch((e) =>
        console.warn("[notifications] failed to save preference:", e),
      );
    }
    setBellOpen(false);
  };

  const bellPopover = (
    title: string,
    options: {
      k: "all" | "mentions" | "none";
      ttl: string;
      sub: string;
    }[],
  ) => (
    <div className="popover-wrap" ref={bellRef}>
      <button
        ref={bellButtonRef}
        type="button"
        className={"icon-btn" + (bellOpen ? " on" : "")}
        title={i18n.t("notifications.bell", { ns: "chat" })}
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
          <div className="pop-head">{title}</div>
          {options.map((opt) => (
            <button
              type="button"
              key={opt.k}
              className="pop-item"
              onClick={() => applyNotifLevel(opt.k)}
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
  );

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
        <span
          className="ch-ico clickable"
          role="button"
          tabIndex={0}
          onClick={() => useProfileModalStore.getState().open(dm.friendId)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") useProfileModalStore.getState().open(dm.friendId);
          }}
        >
          <UserAvatar
            userId={dm.friendId}
            name={dm.name}
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
          />
        </span>
        <span className="ch-name">{dm.name}</span>
        <span className="sep" />
        <span className="topic" title={dmBio || undefined}>
          {dm.online ? i18n.t("activeNow", { ns: "chat" }) : i18n.t("offline", { ns: "chat" })}
          {dmBio && <span className="topic-bio"> · {dmBio}</span>}
        </span>
        <div className="right">
          {dm.isFriend !== false && (
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
          )}
          {dm.isFriend !== false && bellPopover(
            i18n.t("notifications.dmTitle", { ns: "chat", name: dm.name }),
            [
              { k: "all", ttl: i18n.t("notifications.all", { ns: "chat" }), sub: i18n.t("notifications.allHint", { ns: "chat" }) },
              { k: "none", ttl: i18n.t("notifications.nothing", { ns: "chat" }), sub: i18n.t("notifications.dmNothingHint", { ns: "chat" }) },
            ],
          )}
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
          {onRemoveFriend && dm.isFriend !== false && (
            <button
              type="button"
              className="icon-btn"
              title={i18n.t("removeFriend", { ns: "chat" })}
              onClick={onRemoveFriend}
            >
              <Icon name="user-check" size={16} />
            </button>
          )}
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
        {bellPopover(
          i18n.t("notifications.title", { ns: "chat", channel: channel.name }),
          [
            { k: "all", ttl: i18n.t("notifications.all", { ns: "chat" }), sub: i18n.t("notifications.allHint", { ns: "chat" }) },
            { k: "mentions", ttl: i18n.t("notifications.mentions", { ns: "chat" }), sub: i18n.t("notifications.mentionsHint", { ns: "chat" }) },
            { k: "none", ttl: i18n.t("notifications.nothing", { ns: "chat" }), sub: i18n.t("notifications.nothingHint", { ns: "chat" }) },
          ],
        )}
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
      </div>
    </header>
  );
};

const copyTextToClipboard = async (text: string): Promise<void> => {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return;
    } catch {
      // fall through to execCommand
    }
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.style.position = "fixed";
  ta.style.opacity = "0";
  document.body.appendChild(ta);
  ta.focus();
  ta.select();
  try {
    if (!document.execCommand("copy")) {
      throw new Error("execCommand copy failed");
    }
  } finally {
    ta.remove();
  }
};

const renderMentions = (
  text: string,
  keyPrefix: string,
): React.ReactNode[] => {
  const selfName = useAuthStore.getState().user?.name?.toLowerCase();
  const parts: React.ReactNode[] = [];
  let last = 0;
  let i = 0;
  for (const match of text.matchAll(MENTION_REGEX)) {
    const start = (match.index ?? 0) + match[1].length;
    const name = match[2];
    if (start > last) parts.push(text.slice(last, start));
    const isSelf = !!selfName && name.toLowerCase() === selfName;
    parts.push(
      <span
        key={`${keyPrefix}m${i++}`}
        className={"msg-mention" + (isSelf ? " self" : "")}
      >
        @{name}
      </span>,
    );
    last = start + name.length + 1;
  }
  if (parts.length === 0) return [text];
  if (last < text.length) parts.push(text.slice(last));
  return parts;
};

const renderWithLinks = (text: string): React.ReactNode => {
  const parts: React.ReactNode[] = [];
  let last = 0;
  let i = 0;
  for (const match of text.matchAll(URL_REGEX)) {
    const idx = match.index ?? 0;
    if (idx > last) parts.push(...renderMentions(text.slice(last, idx), `s${i}`));
    parts.push(
      <a
        key={`l${i++}`}
        href={match[0]}
        target="_blank"
        rel="noopener noreferrer"
        className="msg-link"
      >
        {match[0]}
      </a>,
    );
    last = idx + match[0].length;
  }
  if (parts.length === 0) return renderMentions(text, "t");
  if (last < text.length) parts.push(...renderMentions(text.slice(last), "e"));
  return parts;
};

const LinkPreview: React.FC<{ link: ParsedLink }> = ({ link }) => {
  const [hidden, setHidden] = React.useState(false);
  const [revealed, setRevealed] = React.useState(false);
  if (hidden) return null;

  if (link.kind === "image") {
    if (!revealed) {
      return (
        <button
          type="button"
          className="link-preview-card link-preview-reveal"
          onClick={() => setRevealed(true)}
        >
          <span className="link-preview-favicon">
            <Icon name="eye" size={16} />
          </span>
          <span className="link-preview-info">
            <span className="link-preview-domain">
              {i18n.t("linkPreviewShow", { ns: "chat" })}
            </span>
            <span className="link-preview-url">{domainOf(link.url)}</span>
          </span>
        </button>
      );
    }
    return (
      <a
        href={link.url}
        target="_blank"
        rel="noopener noreferrer"
        className="link-preview-image"
      >
        <img
          src={link.url}
          alt={domainOf(link.url)}
          loading="lazy"
          onError={() => setHidden(true)}
        />
      </a>
    );
  }

  if (link.kind === "youtube" && link.youtubeId) {
    if (!revealed) {
      return (
        <button
          type="button"
          className="link-preview-card link-preview-reveal"
          onClick={() => setRevealed(true)}
        >
          <span className="link-preview-favicon">
            <Icon name="eye" size={16} />
          </span>
          <span className="link-preview-info">
            <span className="link-preview-domain">
              {i18n.t("linkPreviewShowVideo", { ns: "chat" })}
            </span>
            <span className="link-preview-url">{domainOf(link.url)}</span>
          </span>
        </button>
      );
    }
    return (
      <a
        href={link.url}
        target="_blank"
        rel="noopener noreferrer"
        className="link-preview-video"
      >
        <img
          src={`https://img.youtube.com/vi/${link.youtubeId}/hqdefault.jpg`}
          alt="YouTube"
          loading="lazy"
          onError={() => setHidden(true)}
        />
        <span className="link-preview-play">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        </span>
        <span className="link-preview-video-domain">youtube.com</span>
      </a>
    );
  }

  const domain = domainOf(link.url);
  return (
    <a
      href={link.url}
      target="_blank"
      rel="noopener noreferrer"
      className="link-preview-card"
    >
      <span className="link-preview-favicon">
        <Icon name="globe" size={16} />
      </span>
      <span className="link-preview-info">
        <span className="link-preview-domain">{domain}</span>
        <span className="link-preview-url">{link.url}</span>
      </span>
      <Icon name="chev-left" size={14} className="link-preview-arrow" />
    </a>
  );
};

const IMAGE_CACHE_LIMIT = 40;
interface CachedImage {
  url: string;
  refs: number;
}
const imageUrlCache = new Map<string, CachedImage>();

const evictImageUrls = (protectedId?: string) => {
  if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) return;
  for (const [id, entry] of imageUrlCache) {
    if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) break;
    if (entry.refs > 0 || id === protectedId) continue;
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
  evictImageUrls(id);
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
  const viewerOpenRef = React.useRef(viewerOpen);
  viewerOpenRef.current = viewerOpen;

  React.useEffect(() => {
    return () => releaseImageUrl(attachment.id, urlRef.current);
  }, [attachment.id]);

  React.useEffect(() => {
    if (!isImage) return;
    const node = containerRef.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const intersecting = entries.some((e) => e.isIntersecting);
        if (intersecting) {
          setVisible(true);
        } else {
          setVisible(false);
          if (!viewerOpenRef.current && urlRef.current) {
            releaseImageUrl(attachment.id, urlRef.current);
            setUrl(null);
          }
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [attachment.id, isImage]);

  React.useEffect(() => {
    if (!isImage || url || !visible) return;
    const cached = acquireImageUrl(attachment.id);
    if (cached) {
      setUrl(cached);
      return;
    }
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
        {viewerOpen &&
          url &&
          createPortal(
            <div
              className="image-viewer-overlay"
              onClick={() => {
                setViewerOpen(false);
                if (!visible && urlRef.current) {
                  releaseImageUrl(attachment.id, urlRef.current);
                  setUrl(null);
                }
              }}
            >
              <img src={url} alt={attachment.fileName} />
            </div>,
            document.body,
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
  canModerate?: boolean;
  msg: ChannelMessage;
  displayName: string;
  continued: boolean;
  isMine: boolean;
  dmStyle: boolean;
  receipt: "sent" | "read" | null;
  editing: boolean;
  highlighted: boolean;
  editText: string;
  editRemovedIds: string[];
  editFiles: PendingFile[];
  editSaving: boolean;
  inviteIsMember: boolean;
  onEditTextChange: (v: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onToggleRemoveAttachment: (id: string) => void;
  onAddEditFiles: (files: FileList | File[]) => void;
  onRemoveEditFile: (localId: string) => void;
  onDelete: () => void;
  onAcceptInvite: () => void;
  onDeclineInvite: () => void;
  onOpenInviteServer: () => void;
}

const Message: React.FC<MessageProps> = ({
  canModerate,
  msg,
  displayName,
  continued,
  isMine,
  dmStyle,
  receipt,
  editing,
  highlighted,
  editText,
  editRemovedIds,
  editFiles,
  editSaving,
  inviteIsMember,
  onEditTextChange,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onToggleRemoveAttachment,
  onAddEditFiles,
  onRemoveEditFile,
  onDelete,
  onAcceptInvite,
  onDeclineInvite,
  onOpenInviteServer,
}) => {
  const editFileInputRef = React.useRef<HTMLInputElement>(null);
  const [copied, setCopied] = React.useState(false);
  const copiedTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  React.useEffect(
    () => () => {
      if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    },
    [],
  );
  const handleCopy = () => {
    void copyTextToClipboard(msg.content)
      .then(() => {
        setCopied(true);
        if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
        copiedTimerRef.current = setTimeout(() => setCopied(false), 1200);
      })
      .catch((e) => console.warn("[ChatView] copy failed:", e));
  };
  const created = new Date(msg.createdAt);
  const time = created.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  return (
    <div
      data-msg-id={msg.id}
      className={
        "msg bubble " +
        (isMine ? "mine" : "theirs") +
        (dmStyle ? " dm" : "") +
        (continued ? " cont" : "") +
        (highlighted ? " highlight" : "") +
        (editing ? " editing" : "")
      }
    >
      <UserAvatar
        userId={msg.authorId}
        name={displayName}
        className="ava clickable"
        onClick={() => useProfileModalStore.getState().open(msg.authorId)}
      />
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
              <ServerLogo
                serverId={msg.inviteServerId}
                name={msg.inviteServerName ?? "?"}
                className="dm-invite-card-glyph"
                style={{ display: "inline-flex" }}
              />
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
            <div className="msg-attachments msg-edit-attachments">
              {(msg.attachments ?? [])
                .filter((a) => !editRemovedIds.includes(a.id))
                .map((a) => (
                  <div key={a.id} className="msg-edit-att">
                    <AttachmentItem attachment={a} />
                    <button
                      type="button"
                      className="msg-edit-att-toggle"
                      title={i18n.t("editRemoveAttachment", { ns: "chat" })}
                      disabled={editSaving}
                      onClick={() => onToggleRemoveAttachment(a.id)}
                    >
                      <Icon name="x" size={12} />
                    </button>
                  </div>
                ))}
              <input
                ref={editFileInputRef}
                type="file"
                multiple
                hidden
                onChange={(e) => {
                  if (e.target.files) onAddEditFiles(e.target.files);
                  e.target.value = "";
                }}
              />
              <button
                type="button"
                className="msg-edit-add"
                title={i18n.t("attachFile", { ns: "chat" })}
                disabled={editSaving}
                onClick={() => editFileInputRef.current?.click()}
              >
                <Icon name="plus" size={16} />
              </button>
            </div>
            {editFiles.length > 0 && (
              <div className="attach-chips msg-edit-chips">
                {editFiles.map((p) => (
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
                      disabled={editSaving}
                      onClick={() => onRemoveEditFile(p.localId)}
                    >
                      <Icon name="x" size={11} />
                    </button>
                  </div>
                ))}
              </div>
            )}
            <div className="msg-edit-input-row">
              <input
                autoFocus
                value={editText}
                maxLength={MAX_CONTENT}
                placeholder={
                  (msg.attachments?.length ?? 0) > 0
                    ? i18n.t("editCaptionPlaceholder", { ns: "chat" })
                    : undefined
                }
                readOnly={editSaving}
                onChange={(e) => onEditTextChange(e.target.value)}
                onKeyDown={(e) => {
                  if (editSaving) return;
                  if (e.key === "Escape") {
                    e.preventDefault();
                    onCancelEdit();
                  } else if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    onSaveEdit();
                  }
                }}
              />
              <button
                type="button"
                className="msg-edit-confirm"
                title={i18n.t("editConfirm", { ns: "chat" })}
                disabled={editSaving}
                onClick={onSaveEdit}
              >
                <Icon name="check" size={16} />
              </button>
            </div>
            <div className="msg-edit-hint">
              {i18n.t("editHint", { ns: "chat" })}
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
            {renderWithLinks(msg.content)}
            {(msg.attachments?.length ?? 0) > 0 && (
              <div className="msg-attachments">
                {(msg.attachments ?? []).map((a) => (
                  <AttachmentItem key={a.id} attachment={a} />
                ))}
              </div>
            )}
            {msg.editedAt && <span className="msg-edited"> ({i18n.t("edited", { ns: "chat" })})</span>}
            {(() => {
              const links = extractLinks(msg.content).slice(0, 2);
              if (links.length === 0) return null;
              return (
                <div className="msg-link-previews">
                  {links.map((l) => (
                    <LinkPreview key={l.url} link={l} />
                  ))}
                </div>
              );
            })()}
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
        {isMine && !editing && !msg.inviteServerId && (
          <div className="msg-actions">
            {!msg.callStatus && (
              <button
                type="button"
                className="icon-btn"
                title="Edit"
                onClick={onStartEdit}
              >
                <Icon name="edit" size={13} />
              </button>
            )}
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
        {!isMine &&
          !editing &&
          !msg.inviteServerId &&
          ((msg.content.length > 0 && !msg.callStatus) || canModerate) && (
            <div className="msg-actions">
              {msg.content.length > 0 && !msg.callStatus && (
                <button
                  type="button"
                  className="icon-btn"
                  title={i18n.t("copyMessage", { ns: "chat" })}
                  onClick={handleCopy}
                >
                  <Icon name={copied ? "check" : "copy"} size={13} />
                </button>
              )}
              {canModerate && (
                <button
                  type="button"
                  className="icon-btn"
                  title="Delete"
                  onClick={onDelete}
                >
                  <Icon name="trash" size={13} />
                </button>
              )}
            </div>
          )}
      </div>
    </div>
  );
};

export default ChatView;
