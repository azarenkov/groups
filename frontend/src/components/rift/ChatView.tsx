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
  dm?: DmMeta;
  membersOpen?: boolean;
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  serverId,
  currentUserId,
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

  const bodyRef = React.useRef<HTMLDivElement>(null);
  const loadingOlderRef = React.useRef(false);
  const skipNextAutoScrollRef = React.useRef(false);
  const lastScrollHeightRef = React.useRef(0);

  const isText = !!channel && !!serverId;

  React.useEffect(() => {
    setMessages([]);
    setText("");
    setComposerError(null);
    setEditingId(null);
    setEditText("");
    setDeleteTarget(null);
    setHasMore(false);
    if (!isText || !channel || !serverId) return;

    let cancelled = false;
    setLoadingInitial(true);
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
        console.warn("[ChatView] failed to load messages:", err);
      })
      .finally(() => {
        if (!cancelled) setLoadingInitial(false);
      });
    return () => {
      cancelled = true;
    };
  }, [channel, serverId, isText]);

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
    loadingOlderRef.current = true;
    setLoadingOlder(true);
    const body = bodyRef.current;
    if (body) lastScrollHeightRef.current = body.scrollHeight;
    try {
      const res = await channelAPI.listMessages(serverId, channel.id, {
        before: oldestId,
        limit: PAGE_LIMIT,
      });
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
          b.scrollTop = delta;
        });
      }
    } catch (err) {
      console.warn("[ChatView] failed to load older messages:", err);
    } finally {
      loadingOlderRef.current = false;
      setLoadingOlder(false);
    }
  }, [channel, serverId, hasMore, messages]);

  const handleScroll = React.useCallback(() => {
    const b = bodyRef.current;
    if (!b) return;
    if (b.scrollTop < 80) void loadOlder();
  }, [loadOlder]);

  const sendOnce = async (content: string): Promise<ChannelMessage> => {
    if (!channel || !serverId) throw new Error("no channel");
    return channelAPI.sendMessage(serverId, channel.id, content);
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
        onToggleMembers={onToggleMembers}
        onOpenChannelSettings={onOpenChannelSettings}
      />
      <div className="chat-body" ref={bodyRef} onScroll={handleScroll}>
        {loadingInitial ? (
          <div className="empty-state">
            <span className="loading-spinner" />
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
              return (
                <Message
                  key={m.id}
                  msg={m}
                  continued={cont}
                  isMine={isMine}
                  editing={editingId === m.id}
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
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
}

const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  membersOpen,
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
  continued: boolean;
  isMine: boolean;
  editing: boolean;
  editText: string;
  onEditTextChange: (v: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onDelete: () => void;
}

const Message: React.FC<MessageProps> = ({
  msg,
  continued,
  isMine,
  editing,
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
  const author = msg.authorId.slice(0, 1).toUpperCase();
  return (
    <div className={"msg" + (continued ? " cont" : "")}>
      <div className="ava" style={{ background: colorFor(msg.authorId) }}>
        {author}
      </div>
      <div className="body">
        {!continued && (
          <div className="head">
            <b>{msg.authorId}</b>
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
