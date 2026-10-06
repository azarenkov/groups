import React from "react";

import { userAPI } from "../../features/social/api/user";
import { communityAPI } from "../../features/communities/api/community";
import { ApiError } from "./apiError";
import { blobCacheGeneration, deleteCachedBlob, readCachedBlob, writeCachedBlob } from "./blobCache";

export type AvatarKind = "user" | "community" | "banner";

const CACHE_LIMIT = 256;
const REVALIDATE_MS = 5 * 60_000;

const cache = new Map<string, string | null>();
const pending = new Map<string, Promise<void>>();
const listeners = new Map<string, Set<() => void>>();
const generations = new Map<string, number>();
const fetchedAt = new Map<string, number>();

const keyFor = (kind: AvatarKind, id: string) => `${kind}:${id}`;
const storeKeyFor = (key: string) => `avatar:${key}`;
const restoring = new Set<string>();

const restoreFromDisk = (key: string, gen: number) => {
  if (cache.has(key) || restoring.has(key)) return;
  restoring.add(key);
  void readCachedBlob(storeKeyFor(key))
    .then((blob) => {
      if (!blob || cache.has(key) || (generations.get(key) ?? 0) !== gen) return;
      cache.set(key, URL.createObjectURL(blob));
      evictIfNeeded();
      notify(key);
    })
    .finally(() => restoring.delete(key));
};

const notify = (key: string) => {
  const set = listeners.get(key);
  if (set) for (const fn of Array.from(set)) fn();
};

const evictIfNeeded = () => {
  if (cache.size <= CACHE_LIMIT) return;
  for (const [key, url] of cache) {
    if (cache.size <= CACHE_LIMIT) break;
    if (listeners.get(key)?.size) continue;
    cache.delete(key);
    fetchedAt.delete(key);
    if (url) URL.revokeObjectURL(url);
  }
};

const load = (kind: AvatarKind, id: string) => {
  const key = keyFor(kind, id);
  if (pending.has(key)) return;
  if (
    cache.has(key) &&
    Date.now() - (fetchedAt.get(key) ?? 0) < REVALIDATE_MS
  ) {
    return;
  }
  const gen = generations.get(key) ?? 0;
  const startedAt = blobCacheGeneration();
  restoreFromDisk(key, gen);
  const promise = (kind === "user"
    ? userAPI.downloadAvatar(id)
    : kind === "banner"
      ? userAPI.downloadBanner(id)
      : communityAPI.downloadLogo(id)
  )
    .then((blob) => {
      const url = URL.createObjectURL(blob);
      if ((generations.get(key) ?? 0) !== gen) {
        URL.revokeObjectURL(url);
        return;
      }
      void writeCachedBlob(storeKeyFor(key), blob, startedAt);
      const old = cache.get(key);
      if (old && old !== url) URL.revokeObjectURL(old);
      cache.set(key, url);
      fetchedAt.set(key, Date.now());
      evictIfNeeded();
    })
    .catch((err) => {
      if ((generations.get(key) ?? 0) !== gen) return;
      if (err instanceof ApiError && err.status === 404) {
        void deleteCachedBlob(storeKeyFor(key));
        const old = cache.get(key);
        if (old) URL.revokeObjectURL(old);
        cache.set(key, null);
        fetchedAt.set(key, Date.now());
      } else if (cache.has(key)) {
        fetchedAt.set(key, Date.now());
      } else if (err instanceof ApiError && err.status === 429) {
        const delay = 2000 + Math.random() * 3000;
        setTimeout(() => {
          if (listeners.get(key)?.size) load(kind, id);
        }, delay);
      }
    })
    .finally(() => {
      pending.delete(key);
      const stale = (generations.get(key) ?? 0) !== gen;
      notify(key);
      if (stale && !cache.has(key) && listeners.get(key)?.size) {
        load(kind, id);
      }
    });
  pending.set(key, promise);
};

export const bustAvatar = (kind: AvatarKind, id: string) => {
  const key = keyFor(kind, id);
  generations.set(key, (generations.get(key) ?? 0) + 1);
  void deleteCachedBlob(storeKeyFor(key));
  const url = cache.get(key);
  cache.delete(key);
  fetchedAt.delete(key);
  if (url) URL.revokeObjectURL(url);
  if (!pending.has(key) && listeners.get(key)?.size) {
    load(kind, id);
  }
  notify(key);
};

export const clearAvatarCache = () => {
  for (const key of pending.keys()) {
    generations.set(key, (generations.get(key) ?? 0) + 1);
  }
  for (const url of cache.values()) {
    if (url) URL.revokeObjectURL(url);
  }
  cache.clear();
  fetchedAt.clear();
};

export function useAvatarUrl(
  kind: AvatarKind,
  id: string | null | undefined,
): string | null {
  const key = id ? keyFor(kind, id) : null;
  const [, force] = React.useReducer((x: number) => x + 1, 0);
  React.useEffect(() => {
    if (!key || !id) return;
    let set = listeners.get(key);
    if (!set) {
      set = new Set();
      listeners.set(key, set);
    }
    set.add(force);
    load(kind, id);
    const timer = setInterval(() => load(kind, id), REVALIDATE_MS);
    return () => {
      clearInterval(timer);
      const cur = listeners.get(key);
      if (cur) {
        cur.delete(force);
        if (cur.size === 0) listeners.delete(key);
      }
    };
  }, [key, kind, id]);
  if (!key) return null;
  return cache.get(key) ?? null;
}
