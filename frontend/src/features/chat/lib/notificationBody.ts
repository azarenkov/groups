import i18n from "../../../i18n";
import { formatVoiceDuration, voiceDurationFromPayload } from "./voice";

export function notificationBody(content: string | undefined, attachments: unknown): string {
  if (content) return content;
  const voiceMs = voiceDurationFromPayload(attachments);
  if (voiceMs !== null) {
    return i18n.t("voiceMessageWithDuration", { ns: "chat", duration: formatVoiceDuration(voiceMs) });
  }
  if (Array.isArray(attachments) && attachments.length > 0) {
    return i18n.t("replyAttachment", { ns: "chat" });
  }
  return "";
}
