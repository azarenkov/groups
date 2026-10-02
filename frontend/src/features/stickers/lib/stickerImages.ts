import React from "react";

import { stickersAPI } from "../api/stickers";

const CACHE_LIMIT = 300;

const RETRY_DELAYS_MS = [2000, 5000, 15000, 30000, 60000];

const MAX_CONCURRENT_DOWNLOADS = 6;

const urls = new Map<string, string>();
const failures = new Map<string, number>();
const retryTimers = new Map<string, ReturnType<typeof setTimeout>>();
const pending = new Map<string, Promise<string | null>>();
const listeners = new Map<string, Set<() => void>>();

let cacheGeneration = 0;
let activeDownloads = 0;
const waitingDownloads: Array<() => void> = [];

const acquireSlot = (): Promise<void> => {
  if (activeDownloads < MAX_CONCURRENT_DOWNLOADS) {
    activeDownloads += 1;
    return Promise.resolve();
  }
  return new Promise((resolve) => waitingDownloads.push(resolve));
};

const releaseSlot = () => {
  const next = waitingDownloads.shift();
  if (next) next();
  else activeDownloads -= 1;
};

const wanted = (id: string) => Boolean(listeners.get(id)?.size);

const notify = (id: string) => {
  listeners.get(id)?.forEach((fn) => fn());
};

const evict = () => {
  if (urls.size <= CACHE_LIMIT) return;
  for (const [id, url] of urls) {
    if (urls.size <= CACHE_LIMIT) break;
    if (listeners.get(id)?.size) continue;
    urls.delete(id);
    URL.revokeObjectURL(url);
  }
};

const scheduleRetry = (id: string) => {
  const attempt = failures.get(id) ?? 0;
  failures.set(id, attempt + 1);
  const delay = RETRY_DELAYS_MS[Math.min(attempt, RETRY_DELAYS_MS.length - 1)];
  if (retryTimers.has(id)) return;
  retryTimers.set(
    id,
    setTimeout(() => {
      retryTimers.delete(id);
      if (wanted(id)) void loadSticker(id);
    }, delay),
  );
};

export const retryFailedStickers = () => {
  for (const id of Array.from(failures.keys())) {
    if (!wanted(id)) continue;
    const timer = retryTimers.get(id);
    if (timer) clearTimeout(timer);
    retryTimers.delete(id);
    void loadSticker(id);
  }
};

if (typeof window !== "undefined") {
  window.addEventListener("online", retryFailedStickers);
}

export const loadSticker = (id: string): Promise<string | null> => {
  const cached = urls.get(id);
  if (cached) return Promise.resolve(cached);
  const inFlight = pending.get(id);
  if (inFlight) return inFlight;
  const promise = acquireSlot()
    .then(async () => {
      const generation = cacheGeneration;
      try {
        if (!wanted(id)) return null;
        const blob = await stickersAPI.download(id);
        if (generation !== cacheGeneration) return null;
        const url = URL.createObjectURL(blob);
        failures.delete(id);
        urls.set(id, url);
        evict();
        notify(id);
        return url;
      } catch {
        if (generation !== cacheGeneration || !wanted(id)) return null;
        scheduleRetry(id);
        return null;
      } finally {
        releaseSlot();
      }
    })
    .finally(() => {
      if (pending.get(id) === promise) pending.delete(id);
    });
  pending.set(id, promise);
  return promise;
};

export const useStickerUrl = (id: string | null | undefined, enabled = true): string | null => {
  const [url, setUrl] = React.useState<string | null>(() => (id ? urls.get(id) ?? null : null));
  React.useEffect(() => {
    if (!id) {
      setUrl(null);
      return;
    }
    const update = () => setUrl(urls.get(id) ?? null);
    update();
    if (!enabled) return;
    let set = listeners.get(id);
    if (!set) {
      set = new Set();
      listeners.set(id, set);
    }
    set.add(update);
    void loadSticker(id);
    return () => {
      set?.delete(update);
      if (set?.size || listeners.get(id) !== set) return;
      listeners.delete(id);
      const timer = retryTimers.get(id);
      if (timer) clearTimeout(timer);
      retryTimers.delete(id);
      failures.delete(id);
    };
  }, [id, enabled]);
  return url;
};

export const clearStickerImages = () => {
  cacheGeneration += 1;
  retryTimers.forEach((timer) => clearTimeout(timer));
  retryTimers.clear();
  failures.clear();
  pending.clear();
  urls.forEach((url) => URL.revokeObjectURL(url));
  urls.clear();
  listeners.forEach((_, id) => notify(id));
};
