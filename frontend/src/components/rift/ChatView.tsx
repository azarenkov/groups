import React from "react";
import Icon from "./Icon";

const AVA_COLORS = ["#0071e3", "#ff6b35", "#30a46c", "#bf5af2", "#ff9f0a"];

const colorFor = (id: string): string => {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return AVA_COLORS[h % AVA_COLORS.length];
};

interface ChannelMeta {
  name: string;
  topic?: string;
  type?: "text" | "voice";
}

interface DmMeta {
  name: string;
  initials: string;
  color: string;
  online: boolean;
}

interface ChatMessage {
  id: string;
  authorId: string;
  author: string;
  ts: number;
  text: string;
}

interface ChatViewProps {
  channel?: ChannelMeta;
  dm?: DmMeta;
  membersOpen?: boolean;
  onToggleMembers?: () => void;
  onOpenChannelSettings?: () => void;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  dm,
  membersOpen,
  onToggleMembers,
  onOpenChannelSettings,
}) => {
  const [messages, setMessages] = React.useState<ChatMessage[]>([]);
  const [text, setText] = React.useState("");
  const bodyRef = React.useRef<HTMLDivElement>(null);

  const conversationKey = channel?.name || dm?.name || "";

  React.useEffect(() => {
    setMessages([]);
    setText("");
  }, [conversationKey]);

  React.useEffect(() => {
    if (bodyRef.current) bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [messages.length]);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const v = text.trim();
    if (!v) return;
    setMessages((prev) => [
      ...prev,
      {
        id: "m" + Date.now(),
        authorId: "u-you",
        author: "You",
        ts: Date.now(),
        text: v,
      },
    ]);
    setText("");
  };

  if (!channel && !dm) return null;

  const placeholder = dm
    ? `Message ${dm.name}`
    : channel?.type === "voice"
      ? `Chat in ${channel.name}`
      : `Message #${channel?.name ?? ""}`;

  const headerName = channel ? channel.name : dm!.name;

  return (
    <>
      <ChannelHeader
        channel={channel}
        dm={dm}
        membersOpen={membersOpen}
        onToggleMembers={onToggleMembers}
        onOpenChannelSettings={onOpenChannelSettings}
      />
      <div className="chat-body" ref={bodyRef}>
        {messages.length === 0 ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon
                name={dm ? "user" : channel?.type === "voice" ? "volume" : "hash"}
                size={22}
              />
            </div>
            <h3>Welcome to {dm ? dm.name : `#${headerName}`}</h3>
            <p>
              This is the beginning of the conversation. Say hi — messages
              appear here in real-time.
            </p>
          </div>
        ) : (
          <div className="inner">
            {messages.map((m, i) => {
              const prev = messages[i - 1];
              const cont =
                !!prev && prev.authorId === m.authorId && m.ts - prev.ts < 5 * 60 * 1000;
              return <Message key={m.id} msg={m} continued={cont} />;
            })}
          </div>
        )}
      </div>
      <form className="composer" onSubmit={submit}>
        <div className="composer-wrap">
          <input
            placeholder={placeholder}
            value={text}
            onChange={(e) => setText(e.target.value)}
            autoFocus
          />
          <button
            type="submit"
            className={"send-btn" + (text.trim() ? " enabled" : "")}
            aria-label="Send"
          >
            <Icon name="send" size={14} />
          </button>
        </div>
      </form>
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
        <Icon name={channel.type === "voice" ? "volume" : "hash"} size={16} />
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

const Message: React.FC<{ msg: ChatMessage; continued: boolean }> = ({
  msg,
  continued,
}) => {
  const time = new Date(msg.ts).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  return (
    <div className={"msg" + (continued ? " cont" : "")}>
      <div className="ava" style={{ background: colorFor(msg.authorId) }}>
        {msg.author.slice(0, 1).toUpperCase()}
      </div>
      <div className="body">
        {!continued && (
          <div className="head">
            <b>{msg.author}</b>
            <time>{time}</time>
          </div>
        )}
        <div className="txt">{msg.text}</div>
      </div>
    </div>
  );
};

export default ChatView;
