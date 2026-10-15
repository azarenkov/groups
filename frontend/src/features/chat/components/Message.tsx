import React from "react";
import { Trans } from "react-i18next";

import i18n from "../../../i18n";
import type { ChannelMessage } from "../../../shared/types/channel";
import { colorForId } from "../../../shared/lib/avatar";
import { extractLinks } from "../../../shared/lib/links";
import { formatFileSize } from "../lib/messageFormat";
import { callCardLabel, canCallBack } from "../../voice/lib/callCard";
import { useCallStore } from "../../voice/store/callStore";
import { copyTextToClipboard } from "../../../shared/lib/clipboard";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import ProfileLink from "../../social/components/ProfileLink";
import { rememberMentionTarget } from "../lib/mentionTargets";
import { UserAvatar } from "../../../shared/ui/Avatar";
import Icon from "../../../shared/ui/Icon";
import { openReport } from "../../moderation/store/reportModalStore";
import AttachmentItem from "./AttachmentItem";
import VoicePlayer from "./VoicePlayer";
import VideoNotePlayer from "./VideoNotePlayer";
import { videoNoteAttachmentOf } from "../lib/voice";
import { jumboEmojiCount } from "../lib/emoji";
import StickerImage from "../../stickers/components/StickerImage";
import StickerPacksModal from "../../stickers/components/StickerPacksModal";
import StickerViewer from "../../stickers/components/StickerViewer";
import LinkPreview from "./LinkPreview";
import ReadTicks from "./ReadTicks";
import { renderWithLinks } from "./MessageText";
import { enterAction, insertNewline } from "../lib/composerKeys";
import { useAutoGrow } from "../hooks/useAutoGrow";
import { messageActions, messageCopyText, selectionInside } from "../lib/messageMenu";
import type { MessageAction } from "../lib/messageMenu";
import {
  openContextMenu,
  openContextMenuAt,
} from "../../../shared/ui/contextMenu/contextMenuStore";
import type { MenuEntry } from "../../../shared/ui/contextMenu/contextMenuStore";
import { useLongPress } from "../../../shared/ui/contextMenu/useLongPress";
import {
  IDLE_WHEEL_SWIPE,
  SWIPE_TRIGGER_PX,
  nextWheelSwipe,
  touchSwipeOffset,
  wheelSwipeOffset,
} from "../lib/swipe";
import type { PendingFile } from "../types";
import { MAX_CONTENT } from "../constants";
import { replyText } from "../lib/replyText";
import { displayUserName } from "../../../shared/lib/displayName";

export interface MessageProps {
  tabbable?: boolean;
  blockedAuthor?: boolean;
  blockedReplyAuthor?: boolean;
  msg: ChannelMessage;
  displayName: string;
  continued: boolean;
  hideMeta?: boolean;
  hideTime?: boolean;
  groupPrev?: boolean;
  groupNext?: boolean;
  isMine: boolean;
  dmStyle: boolean;
  receipt: "sent" | "read" | null;
  editing: boolean;
  highlighted: boolean;
  editText: string;
  editRemovedIds: string[];
  editFiles: PendingFile[];
  editSaving: boolean;
  onEditTextChange: (v: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSaveEdit: () => void;
  onToggleRemoveAttachment: (id: string) => void;
  onAddEditFiles: (files: FileList | File[]) => void;
  onRemoveEditFile: (localId: string) => void;
  onDelete: () => void;
  onReply?: () => void;
  onJumpToReply?: (messageId: string) => void;
  onCopyLink?: () => void;
  onForward?: () => void;
  onOpenForwardSource?: () => void;
  onVoiceListened?: () => Promise<void>;
  pinned?: boolean;
  onTogglePin?: () => void;
}

const MESSAGE_STICKER_SIZE = 220;

export const Message: React.FC<MessageProps> = ({
  tabbable,
  blockedAuthor,
  blockedReplyAuthor,
  msg,
  displayName,
  continued,
  hideMeta,
  hideTime,
  groupPrev,
  groupNext,
  isMine,
  dmStyle,
  receipt,
  editing,
  highlighted,
  editText,
  editRemovedIds,
  editFiles,
  editSaving,
  onEditTextChange,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onToggleRemoveAttachment,
  onAddEditFiles,
  onRemoveEditFile,
  onDelete,
  onReply,
  onJumpToReply,
  onCopyLink,
  onForward,
  onOpenForwardSource,
  onVoiceListened,
  pinned,
  onTogglePin,
}) => {
  const jumboCount = !msg.sticker && !(msg.attachments?.length ?? 0) ? jumboEmojiCount(msg.content) : 0;
  const jumbo = jumboCount > 0;
  const [stickerPackOpen, setStickerPackOpen] = React.useState<string | null>(null);
  const [stickerViewerOpen, setStickerViewerOpen] = React.useState(false);
  const editFileInputRef = React.useRef<HTMLInputElement>(null);
  React.useEffect(() => {
    rememberMentionTarget(displayName, msg.authorId);
  }, [displayName, msg.authorId]);
  const editRef = React.useRef<HTMLTextAreaElement>(null);
  useAutoGrow(editRef, editing ? editText : "");
  const [revealed, setRevealed] = React.useState(false);
  const [quoteRevealed, setQuoteRevealed] = React.useState(false);
  const focusOnRevealRef = React.useRef(false);
  const rootRef = React.useRef<HTMLDivElement>(null);
  const wasEditingRef = React.useRef(editing);
  React.useEffect(() => {
    const wasEditing = wasEditingRef.current;
    wasEditingRef.current = editing;
    if (!wasEditing || editing) return;
    const active = document.activeElement;
    const row = rootRef.current;
    if (!row) return;
    if (active && active !== document.body && !row.contains(active)) return;
    row.focus({ preventScroll: true });
  }, [editing]);

  React.useEffect(() => {
    if (!revealed || !focusOnRevealRef.current) return;
    focusOnRevealRef.current = false;
    rootRef.current?.focus();
  }, [revealed]);
  const menuEntries = (): MenuEntry[] => {
    const selection = selectionInside(rootRef.current);
    const actions = messageActions({
      msg,
      isMine,
      hasSelection: selection.trim().length > 0,
      canPin: !!onTogglePin,
    });
    const entry: Record<MessageAction, MenuEntry> = {
      reply: {
        kind: "item",
        id: "reply",
        label: i18n.t("reply", { ns: "chat" }),
        icon: "reply",
        disabled: !onReply,
        onSelect: () => onReply?.(),
      },
      forward: {
        kind: "item",
        id: "forward",
        label: i18n.t("forward", { ns: "chat" }),
        icon: "forward",
        disabled: !onForward,
        onSelect: () => onForward?.(),
      },
      copy: {
        kind: "item",
        id: "copy",
        label: i18n.t("copyText", { ns: "chat" }),
        icon: "copy",
        onSelect: () => {
          void copyTextToClipboard(messageCopyText(msg, selection)).catch((e) =>
            console.warn("[ChatView] copy failed:", e),
          );
        },
      },
      link: {
        kind: "item",
        id: "link",
        label: i18n.t("copyMessageLink", { ns: "chat" }),
        icon: "link",
        disabled: !onCopyLink,
        onSelect: () => onCopyLink?.(),
      },
      pin: {
        kind: "item",
        id: "pin",
        label: i18n.t(pinned ? "pins.unpin" : "pins.pin", { ns: "chat" }),
        icon: "pin",
        onSelect: () => onTogglePin?.(),
      },
      edit: {
        kind: "item",
        id: "edit",
        label: i18n.t("edit", { ns: "chat" }),
        icon: "edit",
        onSelect: onStartEdit,
      },
      report: {
        kind: "item",
        id: "report",
        label: i18n.t("action", { ns: "report" }),
        icon: "flag",
        onSelect: () =>
          openReport({ type: "user", id: msg.authorId, name: displayName, messageId: msg.id }),
      },
      delete: {
        kind: "item",
        id: "delete",
        label: i18n.t("delete", { ns: "chat" }),
        icon: "trash",
        danger: true,
        onSelect: onDelete,
      },
    };
    const pick = (action: MessageAction) => (actions.includes(action) ? [entry[action]] : []);
    return [
      ...pick("reply"),
      ...pick("forward"),
      ...pick("copy"),
      ...pick("link"),
      ...pick("pin"),
      { kind: "separator", id: "s1" },
      ...pick("edit"),
      ...pick("report"),
      { kind: "separator", id: "s2" },
      ...pick("delete"),
    ];
  };
  const longPress = useLongPress((point) => !editing && openContextMenu(point, menuEntries()));
  const canSwipe = !!onReply && !editing && !msg.system && !msg.callStatus;
  const [swipeOffset, setSwipeOffset] = React.useState(0);
  const touchStartRef = React.useRef<{ x: number; y: number } | null>(null);
  const wheelSwipeRef = React.useRef(IDLE_WHEEL_SWIPE);
  const wheelResetRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  React.useEffect(
    () => () => {
      if (wheelResetRef.current) clearTimeout(wheelResetRef.current);
    },
    [],
  );
  const selectingText = () => {
    const selection = window.getSelection();
    return !!selection && !selection.isCollapsed;
  };
  const swipeHandlers = {
    onTouchStart: (e: React.TouchEvent) => {
      if (!editing) longPress.onTouchStart(e);
      if (!canSwipe || e.touches.length !== 1) return;
      touchStartRef.current = { x: e.touches[0].clientX, y: e.touches[0].clientY };
    },
    onTouchMove: (e: React.TouchEvent) => {
      if (!editing) longPress.onTouchMove(e);
      const start = touchStartRef.current;
      if (!start) return;
      const offset = touchSwipeOffset(
        start.x - e.touches[0].clientX,
        start.y - e.touches[0].clientY,
      );
      if (offset === null) {
        touchStartRef.current = null;
        setSwipeOffset(0);
        return;
      }
      setSwipeOffset(offset);
    },
    onTouchEnd: (e: React.TouchEvent) => {
      if (!editing) longPress.onTouchEnd(e);
      if (touchStartRef.current && swipeOffset >= SWIPE_TRIGGER_PX) onReply?.();
      touchStartRef.current = null;
      setSwipeOffset(0);
    },
    onTouchCancel: () => {
      if (!editing) longPress.onTouchCancel();
      touchStartRef.current = null;
      setSwipeOffset(0);
    },
    onClickCapture: (e: React.MouseEvent) => {
      if (!editing) longPress.onClickCapture(e);
    },
    onWheel: (e: React.WheelEvent) => {
      if (!canSwipe || selectingText()) return;
      const next = nextWheelSwipe(wheelSwipeRef.current, e.deltaX, e.deltaY, e.timeStamp);
      wheelSwipeRef.current = next.state;
      if (next.state.accum === 0) return;
      setSwipeOffset(wheelSwipeOffset(next.state));
      if (next.fire) onReply?.();
      if (wheelResetRef.current) clearTimeout(wheelResetRef.current);
      if (!next.state.fired) {
        wheelResetRef.current = setTimeout(() => setSwipeOffset(0), 180);
      }
    },
  };
  const created = new Date(msg.createdAt);
  const time = created.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
  if (blockedAuthor && !revealed) {
    return (
      <div data-msg-id={msg.id} className="msg blocked-collapsed">
        <span className="msg-system-ico">
          <Icon name="shield" size={13} />
        </span>
        <span className="msg-system-text">{i18n.t("blockedMessage", { ns: "chat" })}</span>
        <button
          type="button"
          className="link-btn"
          onClick={() => {
            focusOnRevealRef.current = true;
            setRevealed(true);
          }}
        >
          {i18n.t("showBlockedMessage", { ns: "chat" })}
        </button>
      </div>
    );
  }
  if (msg.system) {
    const joined = msg.system.type === "member_joined";
    return (
      <div data-msg-id={msg.id} className={"msg system" + (highlighted ? " highlight" : "")}>
        <span className="msg-system-ico">
          <Icon name={joined ? "user-plus" : "logout"} size={13} />
        </span>
        <span className="msg-system-text">
          <Trans
            i18n={i18n}
            ns="chat"
            i18nKey={joined ? "system.memberJoined" : "system.memberLeft"}
            values={{ name: displayName }}
            components={[<ProfileLink key="n" userId={msg.system.userId} />]}
          />
        </span>
        <time>{time}</time>
      </div>
    );
  }
  return (
    <div
      ref={rootRef}
      data-msg-id={msg.id}
      tabIndex={tabbable || (blockedAuthor && revealed) ? 0 : -1}
      onContextMenu={(e) => {
        if (editing) return;
        openContextMenu(e, menuEntries());
      }}
      onKeyDown={(e) => {
        if (editing || e.target !== e.currentTarget) return;
        if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) {
          e.preventDefault();
          openContextMenuAt(e.currentTarget, menuEntries());
          return;
        }
        if (e.key === "ArrowUp" || e.key === "ArrowDown") {
          const rows = Array.from(
            e.currentTarget.parentElement?.closest(".chat-body")?.querySelectorAll<HTMLElement>(".msg.bubble[tabindex]") ?? [],
          );
          const next = rows[rows.indexOf(e.currentTarget) + (e.key === "ArrowUp" ? -1 : 1)];
          if (!next) return;
          e.preventDefault();
          next.focus();
          next.scrollIntoView({ block: "nearest" });
        }
      }}
      {...swipeHandlers}
      style={
        swipeOffset > 0
          ? { transform: `translateX(${-swipeOffset}px)`, transition: "none" }
          : undefined
      }
      className={
        "msg bubble " +
        (isMine ? "mine" : "theirs") +
        (dmStyle ? " dm" : "") +
        (continued ? " cont" : "") +
        (groupPrev ? " grp-prev" : "") +
        (groupNext ? " grp-next" : "") +
        (highlighted ? " highlight" : "") +
        (editing ? " editing" : "")
      }
    >
      {swipeOffset > 0 && (
        <span
          className="msg-swipe-hint"
          style={{
            right: `${8 - swipeOffset}px`,
            opacity: Math.min(1, swipeOffset / SWIPE_TRIGGER_PX),
          }}
        >
          <Icon name="reply" size={16} />
        </span>
      )}
      <UserAvatar
        userId={msg.authorId}
        name={displayName}
        className="ava clickable"
        title={i18n.t("viewProfile", { ns: "profile" })}
        onClick={() => useProfileModalStore.getState().open(msg.authorId)}
      />
      <div className="body">
        {!continued && (
          <div className="head">
            <ProfileLink userId={msg.authorId}>
              <b>{displayName}</b>
            </ProfileLink>
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
                {callCardLabel(msg.callStatus, msg.callDurationSeconds, isMine)}
              </div>
              {canCallBack(msg.callStatus) && !isMine && (
                <button
                  type="button"
                  className="link-btn dm-call-back"
                  onClick={() => useCallStore.getState().callFriend(msg.authorId)}
                >
                  {i18n.t("callBack", { ns: "call" })}
                </button>
              )}
            </div>
            <time className="dm-call-time" dateTime={msg.createdAt}>
              {time}
            </time>
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
              <textarea
                ref={editRef}
                rows={1}
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
                    return;
                  }
                  const action = enterAction({
                    key: e.key,
                    shiftKey: e.shiftKey,
                    ctrlKey: e.ctrlKey,
                    metaKey: e.metaKey,
                    altKey: e.altKey,
                    isComposing: e.nativeEvent.isComposing,
                  });
                  if (action === "send") {
                    e.preventDefault();
                    onSaveEdit();
                  } else if (action === "newline") {
                    e.preventDefault();
                    const el = e.currentTarget;
                    const next = insertNewline(el.value, el.selectionStart, el.selectionEnd, el.maxLength);
                    if (!next) return;
                    onEditTextChange(next.value);
                    requestAnimationFrame(() => {
                      el.selectionStart = next.caret;
                      el.selectionEnd = next.caret;
                    });
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
          <div
            className={
              "txt" +
              (videoNoteAttachmentOf(msg) ? " vnote-only" : "") +
              (jumbo && !msg.replyTo && !msg.forwardedFrom ? " jumbo-only" : "") +
              (msg.sticker ? " sticker-only" : "")
            }
          >
            {!dmStyle && !isMine && !continued && (
              <div className="msg-author">
                <ProfileLink
                  userId={msg.authorId}
                  style={{ color: colorForId(msg.authorId) }}
                >
                  {displayName}
                </ProfileLink>
              </div>
            )}
            {(() => {
              const origin = msg.forwardedFrom;
              if (!origin) return null;
              const openable = origin.source.kind === "channel" && !!onOpenForwardSource;
              return (
                <div
                  className={"msg-forwarded" + (openable ? " openable" : "")}
                  role={openable ? "button" : undefined}
                  tabIndex={openable ? 0 : undefined}
                  title={openable ? i18n.t("forwardOpenSource", { ns: "chat" }) : undefined}
                  onClick={(e) => {
                    if (!openable) return;
                    e.stopPropagation();
                    onOpenForwardSource?.();
                  }}
                  onKeyDown={(e) => {
                    if (!openable || (e.key !== "Enter" && e.key !== " ")) return;
                    e.preventDefault();
                    onOpenForwardSource?.();
                  }}
                >
                  <Icon name="forward" size={12} />
                  <span>
                    {i18n.t("forwardedFrom", { ns: "chat", name: displayUserName(origin.authorName) })}
                  </span>
                </div>
              );
            })()}
            {msg.replyTo && (() => {
              const reply = msg.replyTo;
              const masked = !reply.deleted && !!blockedReplyAuthor && !quoteRevealed;
              const activate = () => {
                if (reply.deleted) return;
                if (masked) {
                  setQuoteRevealed(true);
                  return;
                }
                onJumpToReply?.(reply.id);
              };
              return (
                <div
                  className={"msg-reply" + (reply.deleted ? " deleted" : "")}
                  role={reply.deleted ? undefined : "button"}
                  tabIndex={reply.deleted ? undefined : 0}
                  title={
                    reply.deleted
                      ? undefined
                      : masked
                        ? i18n.t("showBlockedMessage", { ns: "chat" })
                        : i18n.t("jumpToReply", { ns: "chat" })
                  }
                  onClick={(e) => {
                    e.stopPropagation();
                    activate();
                  }}
                  onKeyDown={(e) => {
                    if (e.key !== "Enter" && e.key !== " ") return;
                    e.preventDefault();
                    activate();
                  }}
                >
                  {reply.deleted ? (
                    <span className="msg-reply-text">{i18n.t("replyDeleted", { ns: "chat" })}</span>
                  ) : masked ? (
                    <span className="msg-reply-text">{i18n.t("blockedMessage", { ns: "chat" })}</span>
                  ) : (
                    <>
                      {reply.sticker && (
                        <StickerImage
                          id={reply.sticker.id}
                          emoji={reply.sticker.emoji}
                          size={28}
                          className="msg-reply-thumb"
                          animated={reply.sticker.animated}
                        />
                      )}
                      {reply.videoNotePoster && (
                        <img
                          className="msg-reply-thumb"
                          src={`data:image/jpeg;base64,${reply.videoNotePoster}`}
                          alt=""
                        />
                      )}
                      <span
                        className="msg-reply-author"
                        style={{ color: colorForId(reply.authorId ?? "") }}
                      >
                        {reply.authorName && displayUserName(reply.authorName)}
                      </span>
                      <span className="msg-reply-text">
                        {replyText(reply)}
                      </span>
                    </>
                  )}
                </div>
              );
            })()}
            {msg.sticker ? (
              <button
                type="button"
                className="msg-sticker"
                title={msg.sticker.emoji}
                aria-label={`${msg.sticker.emoji} ${i18n.t("replySticker", { ns: "chat" })}`}
                onClick={(e) => {
                  e.stopPropagation();
                  setStickerViewerOpen(true);
                }}
              >
                <StickerImage
                  id={msg.sticker.id}
                  emoji={msg.sticker.emoji}
                  size={MESSAGE_STICKER_SIZE}
                  width={msg.sticker.width}
                  height={msg.sticker.height}
                  animated={msg.sticker.animated}
                />
              </button>
            ) : (
              <span className={"msg-text" + (jumbo ? " jumbo" : "") + (jumboCount > 1 ? " jumbo-few" : "")}>{renderWithLinks(msg.content)}</span>
            )}
            {stickerViewerOpen && msg.sticker && (
              <StickerViewer
                sticker={msg.sticker}
                messageId={msg.id}
                onClose={() => setStickerViewerOpen(false)}
                onOpenPack={(packId) => {
                  setStickerViewerOpen(false);
                  setStickerPackOpen(packId);
                }}
              />
            )}
            {stickerPackOpen && (
              <StickerPacksModal packId={stickerPackOpen} onClose={() => setStickerPackOpen(null)} />
            )}
            {(msg.attachments?.length ?? 0) > 0 && (
              <div className="msg-attachments">
                {(msg.attachments ?? []).map((a) =>
                  a.videoNote ? (
                    <VideoNotePlayer
                      key={a.id}
                      messageId={msg.id}
                      attachment={a}
                      isMine={isMine}
                      listened={!!msg.voiceListened}
                      showListenDot={!isMine || dmStyle}
                      onListened={onVoiceListened}
                    />
                  ) : a.voice ? (
                    <VoicePlayer
                      key={a.id}
                      messageId={msg.id}
                      attachment={a}
                      isMine={isMine}
                      listened={!!msg.voiceListened}
                      showListenDot={!isMine || dmStyle}
                      onListened={onVoiceListened}
                    />
                  ) : (
                    <AttachmentItem key={a.id} attachment={a} />
                  ),
                )}
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
          </div>
        )}
        {pinned && !editing && hideMeta && (
          <div className="msg-meta msg-meta-pin">
            <span className="msg-pinned" title={i18n.t("pins.pinned", { ns: "chat" })} aria-label={i18n.t("pins.pinned", { ns: "chat" })}>
              <Icon name="pin" size={11} />
            </span>
          </div>
        )}
        {!editing && !hideMeta && !msg.callStatus && (
          <div className="msg-meta">
            {pinned && (
              <span className="msg-pinned" title={i18n.t("pins.pinned", { ns: "chat" })} aria-label={i18n.t("pins.pinned", { ns: "chat" })}>
                <Icon name="pin" size={11} />
              </span>
            )}
            {!hideTime && <time className="msg-time">{time}</time>}
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
          </div>
        )}
      </div>
    </div>
  );
};

export default Message;
