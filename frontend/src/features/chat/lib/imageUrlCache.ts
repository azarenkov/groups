import { attachmentAPI } from "../api/attachment";
import { readCachedBlob, writeCachedBlob } from "../../../shared/lib/blobCache";

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

const downloadCachedUrl = (key: string, load: () => Promise<Blob>): Promise<string> => {
  const existing = pendingImageDownloads.get(key);
  if (existing) return existing;
  const storeKey = `img:${key}`;
  const promise = readCachedBlob(storeKey)
    .then(
      (cached) =>
        cached ??
        load().then((blob) => {
          void writeCachedBlob(storeKey, blob);
          return blob;
        }),
    )
    .then((blob) => cacheImageUrl(key, URL.createObjectURL(blob)))
    .finally(() => {
      pendingImageDownloads.delete(key);
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
  const entry = imageUrlCache.get(id);
  if (entry && entry.url === url) {
    if (entry.refs > 0) entry.refs -= 1;
    imageUrlCache.delete(id);
  }
  URL.revokeObjectURL(url);
};
