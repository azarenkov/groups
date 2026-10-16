import { http } from "../../../shared/api/http";

export interface MediaState {
  micMuted: boolean;
  deafened: boolean;
}

export const mediaAPI = {
  async get(): Promise<MediaState> {
    const response = await http.get("/v1/voice/media");
    return response.data.data;
  },

  async save(state: MediaState): Promise<MediaState> {
    const response = await http.put("/v1/voice/media", state);
    return response.data.data;
  },
};
