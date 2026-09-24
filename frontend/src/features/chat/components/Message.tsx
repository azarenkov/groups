import React from "react";
import { Trans } from "react-i18next";

import i18n from "../../../i18n";
import type { ChannelMessage } from "../../../shared/types/channel";
import { colorForId } from "../../../shared/lib/avatar";
import { extractLinks } from "../../../shared/lib/links";
import { formatCallDuration, formatFileSize } from "../lib/messageFormat";
import { copyTextToClipboard } from "../../../shared/lib/clipboard";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import ProfileLink from "../../social/components/ProfileLink";
import { rememberMentionTarget } from "../lib/mentionTargets";
import { UserAvatar, CommunityLogo } from "../../../shared/ui/Avatar";
import Icon from "../../../shared/ui/Icon";
import { openReport } from "../../moderation/store/reportModalStore";
import AttachmentItem from "./AttachmentItem";
import LinkPreview from "./LinkPreview";
import ReadTicks from "./ReadTicks";
import { renderWithLinks } from "./MessageText";
import { enterAction, insertNewline } from "../lib/composerKeys";
import { useAutoGrow } from "../hooks/useAutoGrow";
import type { PendingFile } from "../types";
import { MAX_CONTENT } from "../constants";

export interface MessageProps {
  canModerate?: boolean;
  blockedAuthor?: boolean;
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
  onOpenInviteCommunity: () => void;
}

export const Message: React.FC<MessageProps> = ({
  canModerate,
  blockedAuthor,
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
  onOpenInviteCommunity,
}) => {
  const editFileInputRef = React.useRef<HTMLInputElement>(null);
  React.useEffect(() => {
    rememberMentionTarget(displayName, msg.authorId);
  }, [displayName, msg.authorId]);
  const editRef = React.useRef<HTMLTextAreaElement>(null);
  useAutoGrow(editRef, editing ? editText : "");
  const [copied, setCopied] = React.useState(false);
  const [revealed, setRevealed] = React.useState(false);
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
  if (blockedAuthor && !revealed) {
    return (
      <div data-msg-id={msg.id} className="msg blocked-collapsed">
        <span className="msg-system-ico">
          <Icon name="shield" size={13} />
        </span>
        <span className="msg-system-text">{i18n.t("blockedMessage", { ns: "chat" })}</span>
        <button type="button" className="link-btn" onClick={() => setRevealed(true)}>
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
        ) : msg.inviteCommunityId ? (
          <div className="dm-invite-card">
            <div className="dm-invite-card-label">
              {i18n.t("invite.label", { ns: "chat" })}
            </div>
            <div className="dm-invite-card-row">
              <CommunityLogo
                communityId={msg.inviteCommunityId}
                name={msg.inviteCommunityName ?? "?"}
                className="dm-invite-card-glyph"
                style={{ display: "inline-flex" }}
              />
              <div className="dm-invite-card-info">
                <div className="dm-invite-card-name">
                  {msg.inviteCommunityName ??
                    i18n.t("invite.communityFallback", { ns: "chat" })}
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
                  onClick={onOpenInviteCommunity}
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
                    const next = insertNewline(el.value, el.selectionStart, el.selectionEnd);
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
          <div className="txt">
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
            <span className="msg-text">{renderWithLinks(msg.content)}</span>
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
        {isMine && !editing && !msg.inviteCommunityId && (
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
          !msg.inviteCommunityId &&
          ((msg.content.length > 0 && !msg.callStatus) ||
            (msg.attachments?.length ?? 0) > 0 ||
            canModerate) && (
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
              {!msg.callStatus && !msg.system && (
                <button
                  type="button"
                  className="icon-btn"
                  title={i18n.t("action", { ns: "report" })}
                  onClick={() =>
                    openReport({
                      type: "user",
                      id: msg.authorId,
                      name: displayName,
                      messageId: msg.id,
                    })
                  }
                >
                  <Icon name="flag" size={13} />
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

export default Message;
