import { attachmentAPI } from "../api/attachment";
import { extractVideoPoster } from "./videoPoster";
import { blobCacheGeneration, deleteCachedBlob, readCachedBlob, writeCachedBlob } from "../../../shared/lib/blobCache";

export const IMAGE_CACHE_LIMIT = 40;
export interface CachedImage {
  url: string;
  refs: number;
}
export const imageUrlCache = new Map<string, CachedImage>();

export const evictImageUrls = (protectedId?: string) => {
  if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) return;
  for (const [id, entry] of imageUrlCache) {
    if (imageUrlCache.size <= IMAGE_CACHE_LIMIT) break;
    if (entry.refs > 0 || id === protectedId) continue;
    imageUrlCache.delete(id);
    URL.revokeObjectURL(entry.url);
  }
};

export const cacheImageUrl = (id: string, url: string): string => {
  const entry = imageUrlCache.get(id);
  if (entry) {
    if (entry.url === url) return url;
    if (entry.refs > 0) {
      URL.revokeObjectURL(url);
      return entry.url;
    }
    imageUrlCache.delete(id);
    URL.revokeObjectURL(entry.url);
  }
  imageUrlCache.set(id, { url, refs: 0 });
  evictImageUrls(id);
  return url;
};

export const pendingImageDownloads = new Map<string, Promise<string>>();

let imageSession = 0;

export const clearImageUrlCache = (): void => {
  imageSession += 1;
  for (const entry of imageUrlCache.values()) URL.revokeObjectURL(entry.url);
  imageUrlCache.clear();
  pendingImageDownloads.clear();
};

const downloadCachedUrl = (key: string, load: () => Promise<Blob>): Promise<string> => {
  const existing = pendingImageDownloads.get(key);
  if (existing) return existing;
  const storeKey = `img:${key}`;
  const startedAt = blobCacheGeneration();
  const session = imageSession;
  const promise: Promise<string> = readCachedBlob(storeKey)
    .then(
      (cached) =>
        cached ??
        load().then((blob) => {
          void writeCachedBlob(storeKey, blob, startedAt);
          return blob;
        }),
    )
    .then((blob) => {
      if (session !== imageSession) throw new Error("image session ended");
      return cacheImageUrl(key, URL.createObjectURL(blob));
    })
    .finally(() => {
      if (pendingImageDownloads.get(key) === promise) pendingImageDownloads.delete(key);
    });
  pendingImageDownloads.set(key, promise);
  return promise;
};

export const downloadImageUrl = (id: string, sizeBytes = 0): Promise<string> =>
  downloadCachedUrl(id, () => attachmentAPI.download(id, sizeBytes));

export const thumbnailKey = (id: string) => `thumb:${id}`;

export const downloadThumbnailUrl = (id: string, sizeBytes: number, originalMaxBytes: number): Promise<string> =>
  downloadCachedUrl(thumbnailKey(id), () =>
    attachmentAPI.thumbnail(id, sizeBytes).catch((err) => {
      if (sizeBytes > originalMaxBytes) throw err;
      return attachmentAPI.download(id, sizeBytes);
    }),
  );

export const posterKey = (id: string) => `poster:${id}`;

export const downloadVideoPosterUrl = (id: string, sizeBytes: number): Promise<string> =>
  downloadCachedUrl(posterKey(id), () => attachmentAPI.download(id, sizeBytes).then(extractVideoPoster));

export const acquireImageUrl = (id: string): string | null => {
  const entry = imageUrlCache.get(id);
  if (!entry) return null;
  imageUrlCache.delete(id);
  imageUrlCache.set(id, entry);
  entry.refs += 1;
  return entry.url;
};

export const retainImageUrl = (id: string, url: string) => {
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url) entry.refs += 1;
};

export const releaseImageUrl = (id: string, url: string | null) => {
  if (!url) return;
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url && entry.refs > 0) {
    entry.refs -= 1;
    if (entry.refs === 0) evictImageUrls();
  }
};

export const discardBrokenImageUrl = (id: string, url: string) => {
  void deleteCachedBlob(`img:${id}`);
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url) {
    if (entry.refs > 0) entry.refs -= 1;
    imageUrlCache.delete(id);
  }
  URL.revokeObjectURL(url);
};
