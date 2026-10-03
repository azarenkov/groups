import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { domainOf } from "../../../shared/lib/links";
import { currentLocale } from "../../../i18n";
import { announceVoiceListened, sharedAPI, type SharedItem, type SharedKind, type SharedScope } from "../api/shared";
import { itemKey, useSharedContent } from "../hooks/useSharedContent";
import { useWsChannelMessages, useWsDmMessages } from "../hooks/useWebSocketMessages";
import { ws } from "../../../shared/ws/websocket";
import { useBlockStore } from "../../social/store/blockStore";
import AttachmentItem from "./AttachmentItem";
import VideoNotePlayer from "./VideoNotePlayer";
import VoicePlayer from "./VoicePlayer";

const TABS: SharedKind[] = ["media", "files", "voice", "links"];

interface SharedPanelProps {
  scope: SharedScope;
  currentUserId?: string;
  onClose: () => void;
  onShowMessage: (messageId: string) => void;
}

const formatDate = (iso: string) =>
  new Intl.DateTimeFormat(currentLocale(), { day: "numeric", month: "short", year: "numeric" }).format(new Date(iso));

const SharedEntry: React.FC<{
  item: SharedItem;
  kind: SharedKind;
  mine: boolean;
  blocked: boolean;
  scope: SharedScope;
  onShowMessage: (messageId: string) => void;
}> = ({ item, kind, mine, blocked, scope, onShowMessage }) => {
  const { t } = useTranslation("chat");
  const [revealed, setRevealed] = React.useState(false);
  if (blocked && !revealed) {
    return (
      <li className="shared-entry shared-blocked">
        <span>{t("blockedMessage")}</span>
        <button type="button" className="shared-jump" onClick={() => setRevealed(true)}>
          {t("showBlockedMessage")}
        </button>
      </li>
    );
  }
  const author = mine ? t("shared.you") : item.authorName || t("shared.unknownAuthor");
  const meta = (
    <div className="shared-meta">
      <span className="shared-author">{author}</span>
      <span className="shared-date">{formatDate(item.createdAt)}</span>
      <button type="button" className="shared-jump" onClick={() => onShowMessage(item.messageId)}>
        {t("shared.showInChat")}
      </button>
    </div>
  );
  if (kind === "links" && item.url) {
    return (
      <li className="shared-entry shared-link">
        <a href={item.url} target="_blank" rel="noopener noreferrer" className="shared-link-body">
          <span className="shared-link-icon" aria-hidden="true">
            <Icon name="link" size={14} />
          </span>
          <span className="shared-link-text">
            <span className="shared-link-domain">{domainOf(item.url)}</span>
            <span className="shared-link-url">{item.url}</span>
          </span>
        </a>
        {meta}
      </li>
    );
  }
  const attachment = item.attachment;
  if (!attachment) return null;
  const markListened = async () => {
    await sharedAPI.markListened(scope, item.messageId);
    announceVoiceListened(item.messageId);
  };
  let body: React.ReactNode;
  if (attachment.videoNote) {
    body = (
      <VideoNotePlayer
        messageId={item.messageId}
        playbackId={`shared:${item.messageId}:${attachment.id}`}
        attachment={attachment}
        isMine={mine}
        listened={false}
        autoplay={false}
        showListenDot={false}
        onListened={markListened}
      />
    );
  } else if (attachment.voice) {
    body = (
      <VoicePlayer
        messageId={item.messageId}
        playbackId={`shared:${item.messageId}:${attachment.id}`}
        attachment={attachment}
        isMine={mine}
        listened={false}
        autoplay={false}
        showListenDot={false}
        onListened={markListened}
      />
    );
  } else {
    body = <AttachmentItem attachment={attachment} />;
  }
  return (
    <li className={"shared-entry shared-" + kind}>
      <div className="shared-body">{body}</div>
      {meta}
    </li>
  );
};

const LINK_HINT = /https?:\/\//i;

const SharedPanel: React.FC<SharedPanelProps> = ({ scope, currentUserId, onClose, onShowMessage }) => {
  const { t } = useTranslation("chat");
  const [kind, setKind] = React.useState<SharedKind>("media");
  const [query, setQuery] = React.useState("");
  const content = useSharedContent(scope, kind, query);
  const blockedIds = useBlockStore((s) => s.blocked);
  const sentinelRef = React.useRef<HTMLDivElement>(null);
  const { loadMore, refreshTop, dropMessage, applyEdit } = content;

  const affects = (message: { content?: string; newContent?: string; attachments?: unknown[] }) =>
    kind === "links"
      ? LINK_HINT.test(message.content ?? message.newContent ?? "")
      : (message.attachments?.length ?? 0) > 0;
  const handlers = {
    onSent: (m: { content: string; attachments?: unknown[] }) => {
      if (affects(m)) refreshTop();
    },
    onEdited: (m: { id: string; newContent?: string; attachments?: { id: string }[] }) => {
      applyEdit({
        messageId: m.id,
        content: m.newContent,
        attachmentIds: m.attachments ? m.attachments.map((a) => a.id) : undefined,
      });
      refreshTop();
    },
    onDeleted: (m: { id: string }) => dropMessage(m.id),
  };
  useWsChannelMessages(scope.kind === "channel" ? scope.channelId : null, handlers);
  useWsDmMessages(scope.kind === "dm" ? scope.friendId : null, currentUserId, handlers);

  React.useEffect(
    () =>
      ws.on((env) => {
        if (env.type !== "message.hidden") return;
        const id = (env.data as { messageId?: unknown }).messageId;
        if (typeof id === "string") dropMessage(id);
      }),
    [dropMessage],
  );

  React.useEffect(() => {
    const node = sentinelRef.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) loadMore();
      },
      { rootMargin: "300px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [loadMore, content.items.length]);

  const empty = content.loaded && !content.loading && !content.failed && content.items.length === 0;

  return (
    <aside className="shared-panel" aria-label={t("shared.title")}>
      <div className="shared-head">
        <h3>{t("shared.title")}</h3>
        <button type="button" className="icon-btn" title={t("shared.close")} aria-label={t("shared.close")} onClick={onClose}>
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className="shared-tabs" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={kind === tab}
            className={"shared-tab" + (kind === tab ? " on" : "")}
            onClick={() => setKind(tab)}
          >
            {t(`shared.tabs.${tab}`)}
          </button>
        ))}
      </div>
      <input
        className="s-input shared-search"
        value={query}
        placeholder={t(kind === "links" ? "shared.searchLinks" : "shared.searchFiles")}
        aria-label={t(kind === "links" ? "shared.searchLinks" : "shared.searchFiles")}
        onChange={(e) => setQuery(e.target.value)}
      />
      <div className="shared-scroll">
        <ul className={"shared-list shared-list-" + kind}>
          {content.items.map((item) => (
            <SharedEntry
              key={itemKey(item)}
              item={item}
              kind={kind}
              mine={!!currentUserId && item.authorId === currentUserId}
              blocked={scope.kind === "channel" && !!blockedIds[item.authorId]}
              scope={scope}
              onShowMessage={onShowMessage}
            />
          ))}
        </ul>
        {empty && (
          <div className="shared-empty">{query.trim() ? t("shared.nothingFound") : t(`shared.empty.${kind}`)}</div>
        )}
        {content.failed && (
          <div className="shared-error" role="alert">
            <span>{t("shared.loadFailed")}</span>
            <button type="button" className="s-btn secondary" onClick={content.retry}>
              {t("shared.retry")}
            </button>
          </div>
        )}
        {content.loading && (
          <div className="shared-loading">
            <span className="loading-spinner" />
          </div>
        )}
        <div ref={sentinelRef} className="shared-sentinel" />
      </div>
    </aside>
  );
};

export default SharedPanel;
