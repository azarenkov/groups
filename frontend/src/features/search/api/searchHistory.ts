import { http } from "../../../shared/api/http";
import type { GlobalSearchPlace } from "./globalSearch";

export type HistoryTarget =
  | { type: "query"; text: string }
  | { type: "person"; userId: string; name: string }
  | { type: "dm"; userId: string; name: string }
  | { type: "community"; communityId: string; name: string }
  | { type: "channel"; communityId: string; communityName: string; channelId: string; name: string; voice: boolean }
  | {
      type: "message";
      messageId: string;
      authorId: string;
      authorName: string;
      forwardedAuthorId?: string;
      snippet: string;
      createdAt: string;
      place: GlobalSearchPlace;
    };

export type HistoryItem = HistoryTarget & { id: string; usedAt: string };

export const historyTargetId = (target: HistoryTarget): string | null => {
  switch (target.type) {
    case "query":
      return null;
    case "person":
    case "dm":
      return target.userId;
    case "community":
      return target.communityId;
    case "channel":
      return target.channelId;
    case "message":
      return target.messageId;
  }
};

export const searchHistoryAPI = {
  async list(): Promise<HistoryItem[]> {
    const response = await http.get("/v1/search/history");
    return response.data.data.items;
  },

  async record(target: HistoryTarget): Promise<{ id: string; usedAt: string }> {
    const response = await http.post("/v1/search/history", {
      kind: target.type,
      query: target.type === "query" ? target.text : undefined,
      targetId: historyTargetId(target) ?? undefined,
    });
    return response.data.data;
  },

  async remove(id: string): Promise<void> {
    await http.delete(`/v1/search/history/${encodeURIComponent(id)}`);
  },

  async clear(): Promise<void> {
    await http.delete("/v1/search/history");
  },
};
