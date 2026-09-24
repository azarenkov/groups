import type { ChannelMessage } from "../../../shared/types/channel";

export type MessageAction = "copy" | "edit" | "delete" | "report";

export interface MessageMenuContext {
  msg: ChannelMessage;
  isMine: boolean;
  canModerate: boolean;
  hasSelection: boolean;
}

export function messageActions(ctx: MessageMenuContext): MessageAction[] {
  const { msg, isMine, canModerate, hasSelection } = ctx;
  const card = !!msg.callStatus || !!msg.inviteCommunityId;
  const actions: MessageAction[] = [];
  const hasText = msg.content.length > 0 || (msg.attachments?.length ?? 0) > 0;
  if (!card && (hasText || hasSelection)) actions.push("copy");
  if (isMine && !card && !msg.system) actions.push("edit");
  if ((isMine && !msg.inviteCommunityId) || canModerate) actions.push("delete");
  if (!isMine && !card && !msg.system) actions.push("report");
  return actions;
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
