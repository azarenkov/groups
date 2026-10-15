import i18n from "../../../i18n";
import { formatCallDuration } from "../../chat/lib/messageFormat";

const UNREAD_CALL_STATUSES: readonly string[] = ["missed"];

export const countsAsUnread = (callStatus: string | null | undefined): boolean =>
  !callStatus || UNREAD_CALL_STATUSES.includes(callStatus);

export const canCallBack = (callStatus: string | null | undefined): boolean => callStatus === "missed";

export function callCardLabel(callStatus: string, durationSeconds: number | null | undefined, isMine: boolean): string {
  if (callStatus === "completed") return formatCallDuration(durationSeconds ?? 0);
  if (callStatus === "declined") return i18n.t("callDeclined", { ns: "call" });
  return i18n.t(isMine ? "callNoAnswer" : "callMissed", { ns: "call" });
}
