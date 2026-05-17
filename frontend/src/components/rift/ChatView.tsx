import React from "react";
import Icon from "./Icon";
import ConfirmModal from "./ConfirmModal";
import { channelAPI } from "../../api/channel";
import { ChannelMessage } from "../../types/channel";
import { ApiError } from "../../utils/apiError";

const AVA_COLORS = ["#0071e3", "#ff6b35", "#30a46c", "#bf5af2", "#ff9f0a"];
const PAGE_LIMIT = 50;
const MAX_CONTENT = 4000;

const colorFor = (id: string): string => {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return AVA_COLORS[h % AVA_COLORS.length];
};

interface ChannelMeta {
  id: string;
  name: string;
  topic?: string;
  type: "text";
}

interface DmMeta {
  name: string;
  initials: string;
  color: string;
  online: boolean;
}

interface ChatViewProps {
  channel?: ChannelMeta;
  serverId?: string;
  currentUserId?: string;
  currentUserName?: string;
  dm?: DmMeta;
  membersOpen?: boolean;
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  serverId,
  currentUserId,
  currentUserName,
  dm,
  membersOpen,
  onToggleMembers,
  onOpenChannelSettings,
}) => {
  const [messages, setMessages] = React.useState<ChannelMessage[]>([]);
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

  const isText = !!channel && !!serverId;
  const channelNotReady = isText && loadedChannelIdRef.current !== channel!.id;
  const showInitialLoader = loadingInitial || channelNotReady;

  React.useEffect(() => {
    setMessages([]);
    setText("");
    setComposerError(null);
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
    if (!isText || !channel || !serverId) return;

    let cancelled = false;
    setLoadingInitial(true);
    const targetChannelId = channel.id;
    channelAPI
      .listMessages(serverId, channel.id, { limit: PAGE_LIMIT })
      .then((res) => {
        if (cancelled) return;
        const ordered = [...res.messages].reverse();
        setMessages(ordered);
        setHasMore(res.messages.length >= PAGE_LIMIT);
      })
      .catch((err) => {
        if (cancelled) return;
        const apiErr = err instanceof ApiError ? err : null;
        if (
          apiErr?.code === "CHANNEL_NOT_MEMBER" ||
          apiErr?.code === "CHANNEL_PASSWORD_REQUIRED" ||
          apiErr?.code === "INVALID_CHANNEL_PASSWORD"
        ) {
          setAccessError(apiErr.message || "You haven't joined this channel yet.");
        } else {
          console.warn("[ChatView] failed to load messages:", err);
        }
      })
      .finally(() => {
        if (cancelled) return;
        loadedChannelIdRef.current = targetChannelId;
        setLoadingInitial(false);
      });
    return () => {
      cancelled = true;
    };
  }, [channel?.id, serverId, isText]);

  React.useEffect(() => {
    if (!bodyRef.current) return;
    if (skipNextAutoScrollRef.current) {
      skipNextAutoScrollRef.current = false;
      return;
    }
    bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [messages.length, loadingInitial]);

  const loadOlder = React.useCallback(async () => {
    if (!channel || !serverId) return;
    if (loadingOlderRef.current) return;
    if (!hasMore) return;
    if (messages.length === 0) return;
    const oldestId = messages[0].id;
    const targetChannelId = channel.id;
    loadingOlderRef.current = true;
    setLoadingOlder(true);
    const body = bodyRef.current;
    let prevScrollTop = 0;
    if (body) {
      lastScrollHeightRef.current = body.scrollHeight;
      prevScrollTop = body.scrollTop;
    }
    try {
      const res = await channelAPI.listMessages(serverId, channel.id, {
        before: oldestId,
        limit: PAGE_LIMIT,
      });
      if (loadedChannelIdRef.current !== targetChannelId) return;
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
  }, [channel?.id, serverId, hasMore, messages]);

  const handleScroll = React.useCallback(() => {
    const b = bodyRef.current;
    if (!b) return;
    if (b.scrollTop < 80) void loadOlder();
  }, [loadOlder]);

  const sendOnce = async (content: string): Promise<ChannelMessage> => {
    if (!channel || !serverId) throw new Error("no channel");
    return channelAPI.sendMessage(serverId, channel.id, content);
  };

  const refreshHistoryAfterJoin = async () => {
    if (!channel || !serverId) return;
    try {
      const res = await channelAPI.listMessages(serverId, channel.id, {
        limit: PAGE_LIMIT,
      });
      const ordered = [...res.messages].reverse();
      setMessages(ordered);
      setHasMore(res.messages.length >= PAGE_LIMIT);
    } catch (err) {
      console.warn("[ChatView] failed to refresh history after join:", err);
    }
  };

  React.useEffect(() => {
    const trimmed = searchInput.trim();
    if (!searchOpen) return;
    if (!channel || !serverId) return;
    if (trimmed.length === 0) {
      setSearchQuery("");
      setSearchResults([]);
      setSearchError(null);
      setSearchTotal(0);
      return;
    }
    const timeout = window.setTimeout(() => setSearchQuery(trimmed), 250);
    return () => window.clearTimeout(timeout);
  }, [searchInput, searchOpen, channel, serverId]);

  React.useEffect(() => {
    if (!searchOpen || !channel || !serverId) return;
    const q = searchQuery.trim();
    if (q.length === 0) return;
    let cancelled = false;
    setSearchLoading(true);
    setSearchError(null);
    channelAPI
      .searchMessages(serverId, channel.id, q, 1, 50)
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
  }, [searchQuery, searchOpen, channel, serverId]);

  const handleJumpToMessage = async (target: ChannelMessage) => {
    if (!channel || !serverId) return;
    setSearchOpen(false);
    setSearchInput("");
    setSearchQuery("");
    setSearchResults([]);
    setSearchError(null);
    setSearchTotal(0);
    setLoadingInitial(true);
    try {
      const [olderRes, newerRes] = await Promise.all([
        channelAPI.listMessages(serverId, channel.id, { before: target.id, limit: 25 }),
        channelAPI.listMessages(serverId, channel.id, { after: target.id, limit: 25 }),
      ]);
      const older = [...olderRes.messages].reverse();
      const newer = [...newerRes.messages].reverse();
      const combined = [...older, target, ...newer];
      loadedChannelIdRef.current = channel.id;
      skipNextAutoScrollRef.current = true;
      setMessages(combined);
      setHasMore(olderRes.messages.length >= 25);
      setHighlightedMessageId(target.id);
    } catch (err) {
      console.warn("[ChatView] failed to jump to message:", err);
    } finally {
      setLoadingInitial(false);
    }
  };

  React.useEffect(() => {
    if (!highlightedMessageId) return;
    const el = document.querySelector(
      `[data-msg-id="${highlightedMessageId}"]`,
    ) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
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
      }
      return next;
    });
  };

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
          setAccessError(r?.message || "Couldn't join channel.");
        }
      } else {
        setAccessError(apiErr?.message || "Couldn't join channel.");
      }
    }
  };

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!channel || !serverId) return;
    const v = text.trim();
    if (!v) return;
    if (v.length > MAX_CONTENT) {
      setComposerError(`Message too long (max ${MAX_CONTENT} chars).`);
      return;
    }
    setSending(true);
    setComposerError(null);
    try {
      const msg = await sendOnce(v);
      setMessages((prev) => [...prev, msg]);
      setText("");
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "CHANNEL_NOT_MEMBER") {
        try {
          await channelAPI.joinText(serverId, channel.id);
          await refreshHistoryAfterJoin();
          const msg = await sendOnce(v);
          setMessages((prev) => [...prev, msg]);
          setText("");
        } catch (joinErr) {
          const joinApi = joinErr instanceof ApiError ? joinErr : null;
          const jc = joinApi?.code;
          if (jc === "CHANNEL_PASSWORD_REQUIRED" || jc === "INVALID_CHANNEL_PASSWORD") {
            const pw = window.prompt(`Enter password for #${channel.name}`);
            if (pw === null) {
              setSending(false);
              return;
            }
            try {
              await channelAPI.joinText(serverId, channel.id, pw);
              await refreshHistoryAfterJoin();
              const msg = await sendOnce(v);
              setMessages((prev) => [...prev, msg]);
              setText("");
            } catch (retryErr) {
              const r = retryErr instanceof ApiError ? retryErr : null;
              setComposerError(r?.message || "Couldn't join channel.");
            }
          } else {
            setComposerError(joinApi?.message || "Couldn't join channel.");
          }
        }
      } else if (code === "MESSAGE_CONTENT_EMPTY") {
        setComposerError("Message can't be empty.");
      } else if (code === "MESSAGE_CONTENT_TOO_LONG") {
        setComposerError(`Message too long (max ${MAX_CONTENT} chars).`);
      } else {
        console.warn("[ChatView] send failed:", err);
        setComposerError(apiErr?.message || "Couldn't send message.");
      }
    } finally {
      setSending(false);
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
    if (!channel || !serverId) return;
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
      await channelAPI.editMessage(serverId, channel.id, msg.id, v);
      const editedAt = new Date().toISOString();
      setMessages((prev) =>
        prev.map((m) => (m.id === msg.id ? { ...m, content: v, editedAt } : m)),
      );
      cancelEdit();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "MESSAGE_CONTENT_UNCHANGED") {
        cancelEdit();
        return;
      }
      console.warn("[ChatView] edit failed:", err);
    }
  };

  const confirmDelete = async () => {
    if (!deleteTarget || !channel || !serverId) return;
    const target = deleteTarget;
    try {
      await channelAPI.deleteMessage(serverId, channel.id, target.id);
      setMessages((prev) => prev.filter((m) => m.id !== target.id));
    } catch (err) {
      console.warn("[ChatView] delete failed:", err);
    } finally {
      setDeleteTarget(null);
    }
  };

  if (!channel && !dm) return null;

  const placeholder = dm
    ? `Message ${dm.name}`
    : `Message #${channel?.name ?? ""}`;

  const headerName = channel ? channel.name : dm!.name;

  if (dm) {
    return (
      <>
        <ChannelHeader
          channel={undefined}
          dm={dm}
          membersOpen={membersOpen}
          onToggleMembers={onToggleMembers}
          onOpenChannelSettings={onOpenChannelSettings}
        />
        <div className="chat-body">
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name="user" size={22} />
            </div>
            <h3>Welcome to {dm.name}</h3>
            <p>Direct messages aren't available yet.</p>
          </div>
        </div>
        <form
          className="composer"
          onSubmit={(e) => {
            e.preventDefault();
          }}
        >
          <div className="composer-wrap">
            <input placeholder={placeholder} value="" onChange={() => {}} disabled />
          </div>
        </form>
      </>
    );
  }

  return (
    <>
      <ChannelHeader
        channel={channel}
        membersOpen={membersOpen}
        searchActive={searchOpen}
        onToggleSearch={toggleSearch}
        onToggleMembers={onToggleMembers}
        onOpenChannelSettings={onOpenChannelSettings}
      />
      {searchOpen && channel && (
        <div className="search-bar">
          <Icon name="search" size={14} />
          <input
            autoFocus
            type="text"
            placeholder={`Search in #${channel.name}`}
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                toggleSearch();
              }
            }}
          />
          {searchInput && (
            <button
              type="button"
              className="search-clear"
              onClick={() => setSearchInput("")}
              title="Clear"
            >
              <Icon name="x" size={12} />
            </button>
          )}
        </div>
      )}
      <div
        className="chat-body"
        ref={bodyRef}
        onScroll={handleScroll}
        onClick={() => {
          if (highlightedMessageId) setHighlightedMessageId(null);
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
                <h3>No results</h3>
                <p>Nothing matched "{searchQuery}".</p>
              </div>
            ) : (
              <>
                <div className="search-meta">
                  {searchTotal} {searchTotal === 1 ? "result" : "results"} for "{searchQuery}"
                </div>
                {searchResults.map((m) => {
                  const isMine = !!currentUserId && m.authorId === currentUserId;
                  const displayName =
                    m.authorName ?? (isMine ? currentUserName : undefined) ?? m.authorId;
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
                        style={{ background: colorFor(m.authorId) }}
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
              <Icon name="hash" size={22} />
            </div>
            <h3>Join #{headerName}</h3>
            <p>{accessError}</p>
            <button
              type="button"
              className="btn primary"
              onClick={handleJoinFromAccessError}
            >
              Join channel
            </button>
          </div>
        ) : messages.length === 0 ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name="hash" size={22} />
            </div>
            <h3>Welcome to #{headerName}</h3>
            <p>
              This is the beginning of the conversation. Say hi — messages
              appear here in real-time.
            </p>
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
              const sameAuthor = !!prev && prev.authorId === m.authorId;
              const within = !!prev && new Date(m.createdAt).getTime() - new Date(prev.createdAt).getTime() < 5 * 60 * 1000;
              const cont = sameAuthor && within;
              const isMine = !!currentUserId && m.authorId === currentUserId;
              const displayName = m.authorName ?? (isMine ? currentUserName : undefined) ?? m.authorId;
              return (
                <Message
                  key={m.id}
                  msg={m}
                  displayName={displayName}
                  continued={cont}
                  isMine={isMine}
                  editing={editingId === m.id}
                  highlighted={highlightedMessageId === m.id}
                  editText={editText}
                  onEditTextChange={setEditText}
                  onStartEdit={() => startEdit(m)}
                  onCancelEdit={cancelEdit}
                  onSaveEdit={() => saveEdit(m)}
                  onDelete={() => setDeleteTarget(m)}
                />
              );
            })}
          </div>
        )}
      </div>
      <form className="composer" onSubmit={handleSend}>
        <div className="composer-wrap">
          <input
            ref={composerRef}
            placeholder={placeholder}
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              if (composerError) setComposerError(null);
            }}
            maxLength={MAX_CONTENT}
            autoFocus
            disabled={sending}
          />
          <button
            type="submit"
            className={"send-btn" + (text.trim() ? " enabled" : "")}
            aria-label="Send"
            disabled={sending}
          >
            <Icon name="send" size={14} />
          </button>
        </div>
        {composerError && <div className="composer-error">{composerError}</div>}
      </form>
      {deleteTarget && (
        <ConfirmModal
          title="Delete message?"
          subtitle="This can't be undone."
          confirmLabel="Delete"
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
}

const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  membersOpen,
  searchActive,
  onToggleSearch,
  onToggleMembers,
  onOpenChannelSettings,
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
        <span className="topic">{dm.online ? "Active now" : "Offline"}</span>
        <div className="right">
          <button type="button" className="icon-btn" title="Start a voice call">
            <Icon name="mic" size={16} />
          </button>
          <button type="button" className="icon-btn" title="Conversation settings">
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
            title="Notification settings"
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
              <div className="pop-head">Notifications for #{channel.name}</div>
              {(
                [
                  { k: "all", ttl: "All messages", sub: "Notify on every message" },
                  { k: "mentions", ttl: "@mentions only", sub: "Only when someone @mentions you" },
                  { k: "none", ttl: "Nothing", sub: "Channel is muted" },
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
            title={searchActive ? "Close search" : "Search messages"}
            onClick={onToggleSearch}
          >
            <Icon name="search" size={15} />
          </button>
        )}
        {onOpenChannelSettings && (
          <button
            type="button"
            className="icon-btn"
            title="Channel settings"
            onClick={onOpenChannelSettings}
          >
            <Icon name="settings" size={15} />
          </button>
        )}
      </div>
    </header>
  );
};

interface MessageProps {
  msg: ChannelMessage;
  displayName: string;
  continued: boolean;
  isMine: boolean;
  editing: boolean;
  highlighted: boolean;
  editText: string;
  onEditTextChange: (v: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onDelete: () => void;
}

const Message: React.FC<MessageProps> = ({
  msg,
  displayName,
  continued,
  isMine,
  editing,
  highlighted,
  editText,
  onEditTextChange,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onDelete,
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
        "msg" + (continued ? " cont" : "") + (highlighted ? " highlight" : "")
      }
    >
      <div className="ava" style={{ background: colorFor(msg.authorId) }}>
        {initial}
      </div>
      <div className="body">
        {!continued && (
          <div className="head">
            <b>{displayName}</b>
            <time>{time}</time>
          </div>
        )}
        {editing ? (
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
            {msg.content}
            {msg.editedAt && <span className="msg-edited"> (edited)</span>}
          </div>
        )}
      </div>
      {isMine && !editing && (
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
