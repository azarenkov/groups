import React from "react";

import type { ChannelMessage } from "../../../shared/types/channel";
import { MAX_ATTACHMENTS_PER_MESSAGE } from "../api/attachment";
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
  canModerateMessages: boolean;
  myServerIds: Set<string>;
  receiptFor: (message: ChannelMessage, mine: boolean) => "sent" | "read" | null;
  onEditTextChange: (value: string) => void;
  onStartEdit: (message: ChannelMessage) => void;
  onCancelEdit: () => void;
  onSaveEdit: (message: ChannelMessage) => void;
  onToggleRemoveAttachment: (attachmentId: string) => void;
  onAddEditFiles: (files: FileList | File[], room: number) => void;
  onRemoveEditFile: (localId: string) => void;
  onDelete: (message: ChannelMessage) => void;
  onAcceptInvite: (message: ChannelMessage) => void;
  onDeclineInvite: (message: ChannelMessage) => void;
  onOpenInviteServer?: (serverId: string) => void;
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
  canModerateMessages,
  myServerIds,
  receiptFor,
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
}) => (
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
                  blockedAuthor={!isDm && !m.system && !!blockedIds[m.authorId]}
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
                    !!m.inviteServerId && myServerIds.has(m.inviteServerId)
                  }
                  canModerate={canModerateMessages}
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
                      MAX_ATTACHMENTS_PER_MESSAGE -
                        ((m.attachments?.length ?? 0) -
                          editRemovedIds.length +
                          editFiles.length),
                    )
                  }
                  onRemoveEditFile={onRemoveEditFile}
                  onDelete={() => onDelete(m)}
                  onAcceptInvite={() => onAcceptInvite(m)}
                  onDeclineInvite={() => onDeclineInvite(m)}
                  onOpenInviteServer={() =>
                    m.inviteServerId && onOpenInviteServer?.(m.inviteServerId)
                  }
                />
                </React.Fragment>
              );
            })}
  </div>
);

export default MessageList;
