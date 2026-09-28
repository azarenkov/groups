import React from "react";

import i18n from "../../../i18n";
import { bindingFor } from "../../settings/lib/hotkeys";
import type { ChannelParticipant, ReplyPreview } from "../../../shared/types/channel";
import { colorForId } from "../../../shared/lib/avatar";
import { eventMatches } from "../../settings/lib/hotkeyMatching";
import { formatFileSize } from "../lib/messageFormat";
import { enterAction, insertNewline } from "../lib/composerKeys";
import { useAutoGrow } from "../hooks/useAutoGrow";
import { UserAvatar } from "../../../shared/ui/Avatar";
import Icon from "../../../shared/ui/Icon";
import type { PendingFile } from "../types";
import { replyText } from "../lib/replyText";
import VoiceRecordButton from "./VoiceRecordButton";
import { useVoiceDraftStore } from "../store/voiceDraftStore";
import type { MediaRecording } from "../lib/recording";

interface ComposerProps {
  composerRef: React.RefObject<HTMLTextAreaElement>;
  fileInputRef: React.RefObject<HTMLInputElement>;
  text: string;
  placeholder: string;
  sending: boolean;
  composerError: string | null;
  pendingFiles: PendingFile[];
  mentionQuery: { query: string; start: number } | null;
  mentionSuggestions: ChannelParticipant[];
  mentionIndex: number;
  onTextChange: (value: string, caret: number | null) => void;
  onCaretChange: (value: string, caret: number | null) => void;
  onSubmit: (e: React.FormEvent) => void;
  onAddFiles: (files: File[] | FileList) => void;
  onRemovePendingFile: (localId: string) => void;
  onMentionIndexChange: (update: number | ((i: number) => number)) => void;
  onApplyMention: (name: string) => void;
  onCloseMentions: () => void;
  onEditLastMessage: () => boolean;
  replyTarget?: ReplyPreview | null;
  onCancelReply?: () => void;
  onSendVoice?: (recording: MediaRecording) => Promise<boolean>;
  onVoiceError?: (message: string | null) => void;
  conversationKey?: string;
}

const Composer: React.FC<ComposerProps> = ({
  composerRef,
  fileInputRef,
  text,
  placeholder,
  sending,
  composerError,
  pendingFiles,
  mentionQuery,
  mentionSuggestions,
  mentionIndex,
  onTextChange,
  onCaretChange,
  onSubmit,
  onAddFiles,
  onRemovePendingFile,
  onMentionIndexChange,
  onApplyMention,
  onCloseMentions,
  onEditLastMessage,
  replyTarget,
  onCancelReply,
  onSendVoice,
  onVoiceError,
  conversationKey,
}) => {
  useAutoGrow(composerRef, text);
  const [voiceActive, setVoiceActive] = React.useState(false);
  const hasVoiceDraft = useVoiceDraftStore((s) => !!s.failed[conversationKey ?? ""]);
  const showVoice =
    !!onSendVoice &&
    (voiceActive || hasVoiceDraft || (!text.trim() && pendingFiles.length === 0));
  return (
    <form className="composer" onSubmit={onSubmit}>
        {replyTarget && (
          <div className="composer-reply">
            <Icon name="reply" size={15} />
            {replyTarget.videoNotePoster && (
              <img
                className="msg-reply-thumb"
                src={`data:image/jpeg;base64,${replyTarget.videoNotePoster}`}
                alt=""
              />
            )}
            <div className="composer-reply-body">
              <div className="composer-reply-who">
                {i18n.t("replyingTo", { ns: "chat", name: replyTarget.authorName ?? "" })}
              </div>
              <div className="composer-reply-text">
                {replyText(replyTarget)}
              </div>
            </div>
            <button
              type="button"
              className="composer-reply-close"
              title={i18n.t("cancelReply", { ns: "chat" })}
              aria-label={i18n.t("cancelReply", { ns: "chat" })}
              onClick={onCancelReply}
            >
              <Icon name="x" size={14} />
            </button>
          </div>
        )}
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
                  onClick={() => onRemovePendingFile(p.localId)}
                >
                  <Icon name="x" size={11} />
                </button>
              </div>
            ))}
          </div>
        )}
        <div className={"composer-wrap" + (voiceActive ? " voice-recording" : "")}>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            style={{ display: "none" }}
            onChange={(e) => {
              if (e.target.files) onAddFiles(e.target.files);
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
                    onApplyMention(p.name);
                  }}
                  onMouseEnter={() => onMentionIndexChange(i)}
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
          <textarea
            ref={composerRef}
            rows={1}
            className="composer-input"
            placeholder={placeholder}
            value={text}
            onChange={(e) => onTextChange(e.target.value, e.target.selectionStart)}
            onKeyDown={(e) => {
              if (
                e.key === "Escape" &&
                replyTarget &&
                !(mentionQuery && mentionSuggestions.length > 0)
              ) {
                e.preventDefault();
                onCancelReply?.();
                return;
              }
              if (
                text === "" &&
                !mentionQuery &&
                !e.nativeEvent.isComposing &&
                !e.nativeEvent.repeat &&
                eventMatches(e.nativeEvent, bindingFor("editLastMessage") ?? "")
              ) {
                if (onEditLastMessage()) e.preventDefault();
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
              if (mentionQuery && mentionSuggestions.length > 0) {
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  onMentionIndexChange((i) => (i + 1) % mentionSuggestions.length);
                  return;
                }
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  onMentionIndexChange(
                    (i) =>
                      (i - 1 + mentionSuggestions.length) %
                      mentionSuggestions.length,
                  );
                  return;
                }
                if (e.key === "Tab" || action === "send") {
                  e.preventDefault();
                  const pick =
                    mentionSuggestions[mentionIndex] ?? mentionSuggestions[0];
                  onApplyMention(pick.name);
                  return;
                }
                if (e.key === "Escape") {
                  e.preventDefault();
                  onCloseMentions();
                  return;
                }
              }
              if (action === "send") {
                e.preventDefault();
                e.currentTarget.form?.requestSubmit();
              } else if (action === "newline") {
                e.preventDefault();
                const el = e.currentTarget;
                const next = insertNewline(el.value, el.selectionStart, el.selectionEnd, el.maxLength);
                if (!next) return;
                onTextChange(next.value, next.caret);
                requestAnimationFrame(() => {
                  el.selectionStart = next.caret;
                  el.selectionEnd = next.caret;
                });
              }
            }}
            onBlur={onCloseMentions}
            onSelect={(e) =>
              onCaretChange(e.currentTarget.value, e.currentTarget.selectionStart)
            }
            onPaste={(e) => {
              const files = Array.from(e.clipboardData?.files ?? []);
              if (files.length > 0) {
                e.preventDefault();
                onAddFiles(files);
              }
            }}
            autoFocus
            disabled={sending}
          />
          {showVoice && onSendVoice ? (
            <VoiceRecordButton
              key={conversationKey}
              draftKey={conversationKey ?? ""}
              disabled={sending}
              onSend={onSendVoice}
              onError={(message) => onVoiceError?.(message)}
              onActiveChange={setVoiceActive}
            />
          ) : (
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
              <Icon name="send" size={15} />
            </button>
          )}
        </div>
        {composerError && <div className="composer-error">{composerError}</div>}
      </form>
  );
};

export default Composer;
