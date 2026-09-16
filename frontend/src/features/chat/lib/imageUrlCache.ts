import { attachmentAPI } from "../api/attachment";

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

export const downloadImageUrl = (id: string): Promise<string> => {
  const existing = pendingImageDownloads.get(id);
  if (existing) return existing;
  const promise = attachmentAPI
    .download(id)
    .then((blob) => cacheImageUrl(id, URL.createObjectURL(blob)))
    .finally(() => {
      pendingImageDownloads.delete(id);
    });
  pendingImageDownloads.set(id, promise);
  return promise;
};

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
