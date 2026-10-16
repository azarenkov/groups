import { http } from "../../../shared/api/http";
import type { MediaRevision } from "../lib/mediaRevision";

export interface MediaState {
  micMuted: boolean;
  deafened: boolean;
}

export const mediaAPI = {
  async get(): Promise<MediaState> {
    const response = await http.get("/v1/voice/media");
    return response.data.data;
  },

  async save(state: MediaState, revision: MediaRevision, options: { keepalive?: boolean } = {}): Promise<MediaState> {
    const response = await http.put(
      "/v1/voice/media",
      { ...state, revision },
      options.keepalive ? { adapter: "fetch", fetchOptions: { keepalive: true } } : undefined,
    );
    return response.data.data;
  },
};
