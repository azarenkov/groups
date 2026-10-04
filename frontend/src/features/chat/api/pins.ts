import { http } from "../../../shared/api/http";
import type { SharedScope } from "./shared";

export interface PinnedMessage {
  messageId: string;
  authorId: string;
  authorName: string;
  forwardedAuthorId?: string;
  content: string;
  stickerEmoji?: string;
  hasAttachments: boolean;
  createdAt: string;
  pinnedBy: string;
  pinnedAt: string;
}

const conversationPath = (scope: SharedScope): string =>
  scope.kind === "dm"
    ? `/v1/users/me/dms/${scope.friendId}`
    : `/v1/communities/${scope.communityId}/channels/${scope.channelId}`;

export const pinsAPI = {
  async list(scope: SharedScope): Promise<PinnedMessage[]> {
    const response = await http.get(`${conversationPath(scope)}/pins`);
    return response.data.data.items;
  },
  async pin(scope: SharedScope, messageId: string): Promise<void> {
    await http.put(`${conversationPath(scope)}/messages/${messageId}/pin`);
  },
  async unpin(scope: SharedScope, messageId: string): Promise<void> {
    await http.delete(`${conversationPath(scope)}/messages/${messageId}/pin`);
  },
};
