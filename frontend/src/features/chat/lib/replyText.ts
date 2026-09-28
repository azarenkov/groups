import i18n from "../../../i18n";
import type { ReplyPreview } from "../../../shared/types/channel";
import { formatVoiceDuration } from "./voice";

export function replyText(reply: ReplyPreview): string {
  if (reply.content) return reply.content;
  if (typeof reply.videoNoteDurationMs === "number") {
    return i18n.t("videoNoteWithDuration", {
      ns: "chat",
      duration: formatVoiceDuration(reply.videoNoteDurationMs),
    });
  }
  if (typeof reply.voiceDurationMs === "number") {
    return i18n.t("voiceMessageWithDuration", {
      ns: "chat",
      duration: formatVoiceDuration(reply.voiceDurationMs),
    });
  }
  return reply.hasAttachments ? i18n.t("replyAttachment", { ns: "chat" }) : "";
}
