import React from "react";

import { stickersAPI } from "../api/stickers";

const CACHE_LIMIT = 300;

const urls = new Map<string, string>();
const pending = new Map<string, Promise<string | null>>();
const listeners = new Map<string, Set<() => void>>();

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

export const loadSticker = (id: string): Promise<string | null> => {
  const cached = urls.get(id);
  if (cached) return Promise.resolve(cached);
  const inFlight = pending.get(id);
  if (inFlight) return inFlight;
  const promise = stickersAPI
    .download(id)
    .then((blob) => {
      const url = URL.createObjectURL(blob);
      urls.set(id, url);
      evict();
      notify(id);
      return url;
    })
    .catch(() => null)
    .finally(() => {
      pending.delete(id);
    });
  pending.set(id, promise);
  return promise;
};

export const useStickerUrl = (id: string | null | undefined): string | null => {
  const [url, setUrl] = React.useState<string | null>(() => (id ? urls.get(id) ?? null : null));
  React.useEffect(() => {
    if (!id) {
      setUrl(null);
      return;
    }
    const update = () => setUrl(urls.get(id) ?? null);
    let set = listeners.get(id);
    if (!set) {
      set = new Set();
      listeners.set(id, set);
    }
    set.add(update);
    update();
    void loadSticker(id);
    return () => {
      set?.delete(update);
    };
  }, [id]);
  return url;
};
