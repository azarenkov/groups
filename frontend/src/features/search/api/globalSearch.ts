import { http } from "../../../shared/api/http";
import type { Attachment } from "../../../shared/types/channel";

export type GlobalSearchKind = "messages" | "files" | "links";

export const GLOBAL_SEARCH_KINDS: GlobalSearchKind[] = ["messages", "files", "links"];

export type GlobalSearchPlace =
  | { type: "channel"; communityId: string; communityName: string; channelId: string; channelName: string }
  | { type: "dm"; peerId: string; peerName: string };

export interface GlobalSearchHit {
  messageId: string;
  authorId: string;
  authorName: string;
  forwardedAuthorId?: string;
  createdAt: string;
  place: GlobalSearchPlace;
  snippet: string;
  attachment?: Attachment;
  url?: string;
  position?: number;
}

export interface GlobalSearchPage {
  items: GlobalSearchHit[];
  nextCursor: string | null;
}

export const globalSearchAPI = {
  async search(
    q: string,
    kind: GlobalSearchKind,
    opts: { before?: string | null; limit?: number } = {},
  ): Promise<GlobalSearchPage> {
    const params: Record<string, string | number> = { q, kind };
    if (opts.before) params.before = opts.before;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await http.get("/v1/search", { params });
    return response.data.data;
  },
};
