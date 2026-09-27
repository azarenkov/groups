import React from "react";

import type { ChannelMessage } from "../../../shared/types/channel";
import { maxAttachmentsPerMessage } from "../api/attachment";
import { formatDayDivider, sameDay } from "../lib/messageFormat";
import Message from "./Message";
import type { PendingFile } from "../types";

interface MessageListProps {
  messages: ChannelMessage[];
  loadingOlder: boolean;
  currentUserId?: string;
  isDm: boolean;
  blockedIds: Record<string, unknown>;
  nameOverrides: Record<string, string>;
  editingId: string | null;
  editText: string;
  editRemovedIds: string[];
  editFiles: PendingFile[];
  savingEdit: boolean;
  highlightedMessageId: string | null;
  myCommunityIds: Set<string>;
  receiptFor: (message: ChannelMessage, mine: boolean) => "sent" | "read" | null;
  onEditTextChange: (value: string) => void;
  onStartEdit: (message: ChannelMessage) => void;
  onCancelEdit: () => void;
  onSaveEdit: (message: ChannelMessage) => void;
  onToggleRemoveAttachment: (attachmentId: string) => void;
  onAddEditFiles: (files: FileList | File[], room: number) => void;
  onRemoveEditFile: (localId: string) => void;
  onDelete: (message: ChannelMessage) => void;
  onReply: (message: ChannelMessage, authorName: string) => void;
  onJumpToMessage: (messageId: string) => void;
  onCopyLink: (message: ChannelMessage) => void;
  onForward: (message: ChannelMessage) => void;
  onOpenForwardSource: (message: ChannelMessage) => void;
  onAcceptInvite: (message: ChannelMessage) => void;
  onDeclineInvite: (message: ChannelMessage) => void;
  onOpenInviteCommunity?: (communityId: string) => void;
  onVoiceListened?: (message: ChannelMessage) => Promise<void>;
}

const MessageList: React.FC<MessageListProps> = ({
  messages,
  loadingOlder,
  currentUserId,
  isDm,
  blockedIds,
  nameOverrides,
  editingId,
  editText,
  editRemovedIds,
  editFiles,
  savingEdit,
  highlightedMessageId,
  myCommunityIds,
  receiptFor,
  onEditTextChange,
  onStartEdit,
  onCancelEdit,
  onSaveEdit,
  onToggleRemoveAttachment,
  onAddEditFiles,
  onRemoveEditFile,
  onDelete,
  onReply,
  onJumpToMessage,
  onCopyLink,
  onForward,
  onOpenForwardSource,
  onAcceptInvite,
  onDeclineInvite,
  onOpenInviteCommunity,
  onVoiceListened,
}) => {
  let lastBubbleIndex = -1;
  for (let k = messages.length - 1; k >= 0; k -= 1) {
    const candidate = messages[k];
    const blocked =
      !!blockedIds[candidate.authorId] ||
      (!!candidate.forwardedFrom && !!blockedIds[candidate.forwardedFrom.authorId]);
    if (!candidate.system && (isDm || !blocked)) {
      lastBubbleIndex = k;
      break;
    }
  }
  return (
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
                  tabbable={i === lastBubbleIndex}
                  blockedAuthor={
                    !isDm &&
                    !m.system &&
                    (!!blockedIds[m.authorId] ||
                      (!!m.forwardedFrom && !!blockedIds[m.forwardedFrom.authorId]))
                  }
                  blockedReplyAuthor={!isDm && !!m.replyTo?.authorId && !!blockedIds[m.replyTo.authorId]}
                  msg={m}
                  displayName={displayName}
                  continued={cont}
                  isMine={isMine}
                  dmStyle={isDm}
                  receipt={receiptFor(m, isMine)}
                  editing={editingId === m.id}
                  highlighted={highlightedMessageId === m.id}
                  editText={editText}
                  inviteIsMember={
                    !!m.inviteCommunityId && myCommunityIds.has(m.inviteCommunityId)
                  }
                  editRemovedIds={editRemovedIds}
                  editFiles={editFiles}
                  editSaving={savingEdit}
                  onEditTextChange={onEditTextChange}
                  onStartEdit={() => onStartEdit(m)}
                  onCancelEdit={onCancelEdit}
                  onSaveEdit={() => onSaveEdit(m)}
                  onToggleRemoveAttachment={onToggleRemoveAttachment}
                  onAddEditFiles={(files) =>
                    onAddEditFiles(
                      files,
                      maxAttachmentsPerMessage() -
                        ((m.attachments?.length ?? 0) -
                          editRemovedIds.length +
                          editFiles.length),
                    )
                  }
                  onRemoveEditFile={onRemoveEditFile}
                  onDelete={() => onDelete(m)}
                  onReply={() => onReply(m, subjectName)}
                  onJumpToReply={onJumpToMessage}
                  onCopyLink={() => onCopyLink(m)}
                  onForward={() => onForward(m)}
                  onOpenForwardSource={() => onOpenForwardSource(m)}
                  onAcceptInvite={() => onAcceptInvite(m)}
                  onDeclineInvite={() => onDeclineInvite(m)}
                  onOpenInviteCommunity={() =>
                    m.inviteCommunityId && onOpenInviteCommunity?.(m.inviteCommunityId)
                  }
                  onVoiceListened={onVoiceListened ? () => onVoiceListened(m) : undefined}
                />
                </React.Fragment>
              );
            })}
  </div>
  );
};

export default MessageList;
