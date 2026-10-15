import i18n from "../../../i18n";
import { callCardLabel } from "../../voice/lib/callCard";
import { formatVoiceDuration, videoNoteDurationFromPayload, voiceDurationFromPayload } from "./voice";

export function notificationBody(
  content: string | undefined,
  attachments: unknown,
  sticker?: unknown,
  callStatus?: string,
): string {
  if (callStatus) return callCardLabel(callStatus, null, false);
  if (content) return content;
  const emoji = (sticker as { emoji?: unknown } | null | undefined)?.emoji;
  if (typeof emoji === "string") {
    return `${emoji} ${i18n.t("replySticker", { ns: "chat" })}`;
  }
  const videoMs = videoNoteDurationFromPayload(attachments);
  if (videoMs !== null) {
    return i18n.t("videoNoteWithDuration", { ns: "chat", duration: formatVoiceDuration(videoMs) });
  }
  const voiceMs = voiceDurationFromPayload(attachments);
  if (voiceMs !== null) {
    return i18n.t("voiceMessageWithDuration", { ns: "chat", duration: formatVoiceDuration(voiceMs) });
  }
  if (Array.isArray(attachments) && attachments.length > 0) {
    return i18n.t("replyAttachment", { ns: "chat" });
  }
  return "";
}
