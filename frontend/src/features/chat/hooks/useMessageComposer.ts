import React from "react";
import type { PackSticker } from "../../stickers/api/stickers";
import { useStickerStore } from "../../stickers/store/stickerStore";
import i18n from "../../../i18n";
import { ApiError } from "../../../shared/lib/apiError";
import { channelAPI } from "../../communities/api/channel";
import { blockGeneration, useBlockStore } from "../../social/store/blockStore";
import { friendshipAPI } from "../../social/api/friendship";
import type { Attachment, ChannelMessage, ReplyPreview } from "../../../shared/types/channel";
import { MAX_CONTENT } from "../constants";
import { splitForSend } from "../lib/messageFormat";
import type { ChannelMeta, DmMeta } from "../types";
import type { useAttachments } from "./useAttachments";
import type { Messenger } from "./useMessenger";
import { attachmentAPI } from "../api/attachment";
import type { MediaRecording } from "../lib/recording";

export function useMessageComposer(params: {
  messenger: Messenger | null;
  channel?: ChannelMeta;
  communityId?: string;
  dm?: DmMeta;
  isText: boolean;
  convId?: string;
  attachments: ReturnType<typeof useAttachments>;
  setMessages: React.Dispatch<React.SetStateAction<ChannelMessage[]>>;
  currentChannelIdRef: React.MutableRefObject<string | undefined>;
  refreshHistory: () => Promise<void>;
}) {
  const {
    messenger,
    channel,
    communityId,
    dm,
    isText,
    convId,
    attachments,
    setMessages,
    currentChannelIdRef,
    refreshHistory,
  } = params;
  const [text, setText] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [composerError, setComposerError] = React.useState<string | null>(null);
  const [replyTarget, setReplyTarget] = React.useState<ReplyPreview | null>(null);
  const composerRef = React.useRef<HTMLTextAreaElement>(null);
  const wasSendingRef = React.useRef(false);

  React.useEffect(() => {
    setText("");
    setComposerError(null);
    setReplyTarget(null);
  }, [convId]);

  React.useEffect(() => {
    if (wasSendingRef.current && !sending) {
      composerRef.current?.focus();
    }
    wasSendingRef.current = sending;
  }, [sending]);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!messenger) return;
    const {
      pendingFiles,
      pendingFilesRef,
      setPendingFiles,
      discardPendingFile,
      inFlightSendRef,
    } = attachments;
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
    let replyId: string | undefined = replyTarget?.id;
    if (idsToAttach) {
      for (const id of sentLocalIds) inFlightSendRef.current.add(id);
    }
    const sendChunk = async (chunk: string) => {
      const msg = await messenger.sendMessage(chunk, idsToAttach, replyId);
      if (replyId) {
        replyId = undefined;
        if (stillHere()) setReplyTarget(null);
      }
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
        if (isText && channel && communityId && code === "CHANNEL_NOT_MEMBER") {
          try {
            await channelAPI.joinText(communityId, channel.id);
            await refreshHistory();
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
              await channelAPI.joinText(communityId, channel.id, pw);
              await refreshHistory();
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
      if (code === "REPLY_INVALID") {
        if (stillHere()) setReplyTarget(null);
        reportComposerError(i18n.t("replyUnavailable", { ns: "chat" }));
      } else if (code === "MESSAGE_CONTENT_EMPTY") {
        reportComposerError(i18n.t("messageCantBeEmpty", { ns: "chat" }));
      } else if (code === "MESSAGE_CONTENT_TOO_LONG") {
        reportComposerError(i18n.t("chunkTooLong", { ns: "chat" }));
      } else if (code === "NOT_FRIENDS") {
        reportComposerError(i18n.t("notFriends", { ns: "chat" }));
      } else if (code === "USER_BLOCKED") {
        reportComposerError(i18n.t("userBlocked", { ns: "chat" }));
        if (dm) {
          const peerId = dm.friendId;
          const gen = blockGeneration(peerId);
          void friendshipAPI
            .blockRelation(peerId)
            .then((relation) => useBlockStore.getState().applyRelation(peerId, relation, gen))
            .catch(() => {});
        }
      } else if (code === "DMS_CLOSED") {
        reportComposerError(i18n.t("dmsClosed", { ns: "chat" }));
      } else if (code === "RECIPIENT_INACTIVE") {
        reportComposerError(i18n.t("recipientInactive", { ns: "chat" }));
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

  const sendVoice = async (recording: MediaRecording): Promise<boolean> => {
    if (!messenger) return false;
    const originConvId = currentChannelIdRef.current;
    const stillHere = () => currentChannelIdRef.current === originConvId;
    const reply = replyTarget;
    setSending(true);
    setComposerError(null);
    let pending: Attachment | null = null;
    try {
      const uploaded =
        recording.kind === "video"
          ? await attachmentAPI.uploadVideoNote(recording.blob, recording.mimeType, {
              durationMs: recording.durationMs,
              size: recording.size,
              poster: recording.poster,
            })
          : await attachmentAPI.uploadVoice(recording.blob, recording.mimeType, {
              durationMs: recording.durationMs,
              waveform: recording.waveform,
            });
      pending = uploaded;
      const msg = await messenger.sendMessage("", [uploaded.id], reply?.id);
      pending = null;
      msg.attachments = msg.attachments?.length ? msg.attachments : [uploaded];
      if (stillHere()) {
        if (reply) setReplyTarget(null);
        setMessages((prev) => (prev.some((x) => x.id === msg.id) ? prev : [...prev, msg]));
      }
      return true;
    } catch (err) {
      if (pending) void attachmentAPI.delete(pending.id).catch(() => {});
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      const message =
        code === "VOICE_TOO_LONG"
          ? i18n.t("voiceTooLong", { ns: "chat" })
          : code === "VIDEO_NOTE_TOO_LONG"
            ? i18n.t("videoNoteTooLong", { ns: "chat" })
          : code === "VIDEO_NOTE_INVALID" || (recording.kind === "video" && code === "ATTACHMENTS_INVALID")
            ? i18n.t("videoNoteInvalid", { ns: "chat" })
          : code === "VOICE_INVALID" || code === "ATTACHMENTS_INVALID"
            ? i18n.t("voiceInvalid", { ns: "chat" })
            : code === "REPLY_INVALID"
              ? i18n.t("replyUnavailable", { ns: "chat" })
              : code === "NOT_FRIENDS"
                ? i18n.t("notFriends", { ns: "chat" })
                : code === "USER_BLOCKED"
                  ? i18n.t("userBlocked", { ns: "chat" })
                  : code === "DMS_CLOSED"
                    ? i18n.t("dmsClosed", { ns: "chat" })
                    : apiErr?.message || i18n.t("couldNotSend", { ns: "chat" });
      if (code === "REPLY_INVALID" && stillHere()) setReplyTarget(null);
      if (stillHere()) setComposerError(message);
      return false;
    } finally {
      setSending(false);
    }
  };

  const sendSticker = async (sticker: PackSticker): Promise<boolean> => {
    if (!messenger) return false;
    const originConvId = currentChannelIdRef.current;
    const stillHere = () => currentChannelIdRef.current === originConvId;
    const reply = replyTarget;
    setComposerError(null);
    const send = () => messenger.sendMessage("", undefined, reply?.id, sticker.id);
    try {
      let msg;
      try {
        msg = await send();
      } catch (err) {
        const code = err instanceof ApiError ? err.code : undefined;
        if (!(isText && channel && communityId && code === "CHANNEL_NOT_MEMBER")) throw err;
        await channelAPI.joinText(communityId, channel.id);
        await refreshHistory();
        msg = await send();
      }
      msg.sticker = msg.sticker ?? sticker;
      useStickerStore.getState().rememberUsed(sticker);
      if (stillHere()) {
        if (reply) setReplyTarget(null);
        setMessages((prev) => (prev.some((x) => x.id === msg.id) ? prev : [...prev, msg]));
      }
      return true;
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const message =
        apiErr?.code === "STICKER_INVALID"
          ? i18n.t("stickerUnavailable", { ns: "chat" })
          : apiErr?.code === "REPLY_INVALID"
            ? i18n.t("replyUnavailable", { ns: "chat" })
            : apiErr?.message || i18n.t("couldNotSend", { ns: "chat" });
      if (stillHere()) setComposerError(message);
      return false;
    }
  };

  return {
    text,
    setText,
    sending,
    sendVoice,
    sendSticker,
    composerError,
    setComposerError,
    composerRef,
    handleSend,
    replyTarget,
    setReplyTarget,
  };
}
