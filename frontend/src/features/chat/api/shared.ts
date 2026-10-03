import { http } from "../../../shared/api/http";
import type { Attachment, ChannelMessage } from "../../../shared/types/channel";
import { channelAPI } from "../../communities/api/channel";
import { dmAPI } from "../../dm/api/dm";

export type SharedKind = "media" | "files" | "voice" | "links";

export type SharedScope =
  | { kind: "channel"; communityId: string; channelId: string }
  | { kind: "dm"; friendId: string };

export interface SharedItem {
  messageId: string;
  authorId: string;
  authorName: string;
  createdAt: string;
  attachment?: Attachment;
  url?: string;
}

export interface SharedPage {
  items: SharedItem[];
  nextCursor: string | null;
}

export const sharedScopeKey = (scope: SharedScope): string =>
  scope.kind === "dm" ? `dm:${scope.friendId}` : `ch:${scope.communityId}:${scope.channelId}`;

const sharedPath = (scope: SharedScope): string =>
  scope.kind === "dm"
    ? `/v1/users/me/dms/${scope.friendId}/shared`
    : `/v1/communities/${scope.communityId}/channels/${scope.channelId}/shared`;

export const sharedAPI = {
  message(scope: SharedScope, messageId: string): Promise<ChannelMessage> {
    return scope.kind === "dm"
      ? dmAPI.getMessage(scope.friendId, messageId)
      : channelAPI.getMessage(scope.communityId, scope.channelId, messageId);
  },

  async list(
    scope: SharedScope,
    kind: SharedKind,
    opts: { q?: string; before?: string | null; limit?: number } = {},
  ): Promise<SharedPage> {
    const params: Record<string, string | number> = { kind };
    if (opts.q) params.q = opts.q;
    if (opts.before) params.before = opts.before;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await http.get(sharedPath(scope), { params });
    return response.data.data;
  },
};
