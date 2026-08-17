import React from "react";

import { userAPI } from "../api/user";
import { serverAPI } from "../api/server";
import { ApiError } from "./apiError";

export type AvatarKind = "user" | "server";

const CACHE_LIMIT = 256;

const cache = new Map<string, string | null>();
const pending = new Map<string, Promise<void>>();
const listeners = new Map<string, Set<() => void>>();
const generations = new Map<string, number>();

const keyFor = (kind: AvatarKind, id: string) => `${kind}:${id}`;

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
    if (url) URL.revokeObjectURL(url);
  }
};

const load = (kind: AvatarKind, id: string) => {
  const key = keyFor(kind, id);
  if (cache.has(key) || pending.has(key)) return;
  const gen = generations.get(key) ?? 0;
  const promise = (kind === "user"
    ? userAPI.downloadAvatar(id)
    : serverAPI.downloadLogo(id)
  )
    .then((blob) => {
      const url = URL.createObjectURL(blob);
      if ((generations.get(key) ?? 0) !== gen) {
        URL.revokeObjectURL(url);
        return;
      }
      cache.set(key, url);
      evictIfNeeded();
    })
    .catch((err) => {
      if ((generations.get(key) ?? 0) !== gen) return;
      if (err instanceof ApiError && err.status === 404) {
        cache.set(key, null);
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
  const url = cache.get(key);
  cache.delete(key);
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
    return () => {
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
