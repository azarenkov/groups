import type { ChannelMessage } from "../../../shared/types/channel";
import { isVoiceMessage } from "./voice";

export type MessageAction = "reply" | "forward" | "copy" | "link" | "pin" | "edit" | "delete" | "report";

export interface MessageMenuContext {
  msg: ChannelMessage;
  isMine: boolean;
  hasSelection: boolean;
  canPin?: boolean;
}

export function messageActions(ctx: MessageMenuContext): MessageAction[] {
  const { msg, isMine, hasSelection, canPin } = ctx;
  const card = !!msg.callStatus || !!msg.inviteCommunityId;
  const actions: MessageAction[] = [];
  if (!card && !msg.system) actions.push("reply");
  if (!card && !msg.system) actions.push("forward");
  const voice = isVoiceMessage(msg) || !!msg.sticker;
  const hasText = !voice && (msg.content.length > 0 || (msg.attachments?.length ?? 0) > 0);
  if (!card && (hasText || hasSelection)) actions.push("copy");
  if (!msg.system) actions.push("link");
  if (canPin && !card && !msg.system) actions.push("pin");
  if (isMine && !card && !voice && !msg.system && !msg.forwardedFrom) actions.push("edit");
  if (isMine || !msg.inviteCommunityId) actions.push("delete");
  if (!isMine && !card && !msg.system) actions.push("report");
  return actions;
}

export function canDeleteForEveryone(
  msg: ChannelMessage,
  isMine: boolean,
  canModerate: boolean,
): boolean {
  return (isMine && !msg.inviteCommunityId) || canModerate;
}

export function messageCopyText(msg: ChannelMessage, selection: string): string {
  if (selection.trim().length > 0) return selection;
  const names = (msg.attachments ?? []).map((a) => a.fileName);
  return [msg.content, ...names].filter((part) => part.length > 0).join("\n");
}

export function selectionInside(el: Element | null): string {
  const selection = window.getSelection();
  if (!el || !selection || selection.isCollapsed || selection.rangeCount === 0) return "";
  const range = selection.getRangeAt(0);
  if (!el.contains(range.commonAncestorContainer)) return "";
  return selection.toString();
}
