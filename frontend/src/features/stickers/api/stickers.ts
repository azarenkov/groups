import { http } from "../../../shared/api/http";

export interface PackSticker {
  id: string;
  packId: string;
  emoji: string;
  animated: boolean;
  width: number;
  height: number;
}

export interface StickerPack {
  id: string;
  ownerId: string;
  name: string;
  slug: string;
  installCount: number;
  stickerCount: number;
  stickers: PackSticker[];
}

class StickersAPI {
  private client = http;

  async search(query: string, page = 0): Promise<StickerPack[]> {
    const response = await this.client.get("/v1/sticker-packs", { params: { query, page } });
    return response.data.data;
  }

  async mine(): Promise<StickerPack[]> {
    const response = await this.client.get("/v1/sticker-packs/mine");
    return response.data.data;
  }

  async pack(idOrSlug: string): Promise<StickerPack> {
    const response = await this.client.get(`/v1/sticker-packs/${encodeURIComponent(idOrSlug)}`);
    return response.data.data;
  }

  async create(name: string, slug: string): Promise<StickerPack> {
    const response = await this.client.post("/v1/sticker-packs", { name, slug });
    return response.data.data;
  }

  async rename(packId: string, name: string): Promise<void> {
    await this.client.patch(`/v1/sticker-packs/${packId}`, { name });
  }

  async remove(packId: string): Promise<void> {
    await this.client.delete(`/v1/sticker-packs/${packId}`);
  }

  async addSticker(packId: string, file: File, emoji: string): Promise<StickerPack> {
    const form = new FormData();
    form.append("emoji", emoji);
    form.append("file", file, file.name);
    const response = await this.client.post(`/v1/sticker-packs/${packId}/stickers`, form, {
      timeout: 60000,
    });
    return response.data.data;
  }

  async removeSticker(packId: string, stickerId: string): Promise<void> {
    await this.client.delete(`/v1/sticker-packs/${packId}/stickers/${stickerId}`);
  }

  async reorder(packId: string, stickerIds: string[]): Promise<void> {
    await this.client.put(`/v1/sticker-packs/${packId}/order`, { stickerIds });
  }

  async install(packId: string): Promise<void> {
    await this.client.put(`/v1/sticker-packs/${packId}/install`);
  }

  async uninstall(packId: string): Promise<void> {
    await this.client.delete(`/v1/sticker-packs/${packId}/install`);
  }

  async download(stickerId: string): Promise<Blob> {
    const response = await this.client.get(`/v1/stickers/${stickerId}/file`, {
      responseType: "blob",
    });
    return response.data;
  }
}

export const stickersAPI = new StickersAPI();
