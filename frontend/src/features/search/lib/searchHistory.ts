import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { useAuthStore } from "../../auth/store/authStore";
import { historyTargetId, searchHistoryAPI, type HistoryItem, type HistoryTarget } from "../api/searchHistory";

export const SEARCH_HISTORY_LIMIT = 20;
const STORAGE_KEY = "rift.searchHistory.v1";
const TEMP_PREFIX = "pending:";

export const isPendingHistoryId = (id: string): boolean => id.startsWith(TEMP_PREFIX);

export const historyKey = (target: HistoryTarget): string =>
  target.type === "query" ? `query:${target.text.trim().toLowerCase()}` : `${target.type}:${historyTargetId(target)}`;

export function readStoredHistory(userId: string): HistoryItem[] | undefined {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return undefined;
    const parsed = JSON.parse(raw) as { userId?: unknown; items?: unknown };
    if (parsed.userId !== userId || !Array.isArray(parsed.items)) return undefined;
    return parsed.items as HistoryItem[];
  } catch {
    return undefined;
  }
}

export function storeHistory(userId: string, items: HistoryItem[]): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ userId, items: items.filter((item) => !item.id.startsWith(TEMP_PREFIX)) }),
    );
  } catch {
    return;
  }
}

export function clearStoredHistory(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    return;
  }
}

let persisting = false;

export function installHistoryPersistence(): void {
  if (persisting) return;
  persisting = true;
  queryClient.getQueryCache().subscribe((event) => {
    const [scope, userId] = event.query.queryKey as [unknown, unknown];
    if (scope !== "searchHistory" || typeof userId !== "string" || !userId) return;
    if (event.type !== "updated" && event.type !== "added") return;
    const items = event.query.state.data as HistoryItem[] | undefined;
    if (items) storeHistory(userId, items);
  });
}

const read = (userId: string) => queryClient.getQueryData<HistoryItem[]>(queryKeys.searchHistory(userId));
const write = (userId: string, items: HistoryItem[]) =>
  queryClient.setQueryData<HistoryItem[]>(queryKeys.searchHistory(userId), items);

const queues = new Map<string, Promise<unknown>>();
const savedTemps = new Map<string, string | null>();
const pendingChanges = new Map<string, Set<(items: HistoryItem[]) => HistoryItem[]>>();

const verifiedUsers = new Set<string>();
const verificationListeners = new Set<() => void>();

const setVerified = (userId: string | null) => {
  if (userId === null) verifiedUsers.clear();
  else if (verifiedUsers.has(userId)) return;
  else verifiedUsers.add(userId);
  verificationListeners.forEach((notify) => notify());
};

export const historyVerified = (userId: string): boolean => verifiedUsers.has(userId);

export const subscribeHistoryVerification = (notify: () => void): (() => void) => {
  verificationListeners.add(notify);
  return () => verificationListeners.delete(notify);
};

export async function fetchSearchHistory(userId: string): Promise<HistoryItem[]> {
  const items = await searchHistoryAPI.list();
  setVerified(userId);
  return withPendingChanges(userId, items);
}

export const withPendingChanges = (userId: string, items: HistoryItem[]): HistoryItem[] =>
  [...(pendingChanges.get(userId) ?? [])].reduce((acc, change) => change(acc), items);

export class HistorySessionChanged extends Error {
  constructor() {
    super("The account changed before this search history change was sent");
  }
}

let sessionGen = 0;

const enqueue = (userId: string, request: () => Promise<void>): Promise<void> => {
  const gen = sessionGen;
  const run = (queues.get(userId) ?? Promise.resolve())
    .catch(() => {})
    .then(() => {
      if (gen !== sessionGen || useAuthStore.getState().user?.id !== userId) throw new HistorySessionChanged();
      return request();
    });
  queues.set(userId, run);
  return run;
};

export function resetSearchHistorySession(): void {
  sessionGen += 1;
  queues.clear();
  savedTemps.clear();
  pendingChanges.clear();
  setVerified(null);
}

const settled = (items: HistoryItem[]): HistoryItem[] =>
  items.flatMap((item) => {
    if (!savedTemps.has(item.id)) return [item];
    const serverId = savedTemps.get(item.id);
    return serverId ? [{ ...item, id: serverId }] : [];
  });

async function apply(
  userId: string,
  change: (items: HistoryItem[]) => HistoryItem[],
  request: () => Promise<void>,
): Promise<void> {
  void queryClient.cancelQueries({ queryKey: queryKeys.searchHistory(userId) }, { revert: false });
  const changes = pendingChanges.get(userId) ?? new Set();
  pendingChanges.set(userId, changes);
  changes.add(change);
  const before = read(userId) ?? [];
  const optimistic = write(userId, change(before));
  try {
    await enqueue(userId, request);
  } catch (err) {
    if (read(userId) === optimistic) write(userId, settled(before));
    void queryClient.invalidateQueries({ queryKey: queryKeys.searchHistory(userId) });
    throw err;
  } finally {
    changes.delete(change);
  }
}

let tempSeq = 0;

export function recordSearch(userId: string, target: HistoryTarget): Promise<void> {
  const tempId = `${TEMP_PREFIX}${++tempSeq}`;
  const key = historyKey(target);
  return apply(
    userId,
    (items) =>
      [
        { ...target, id: tempId, usedAt: new Date().toISOString() } as HistoryItem,
        ...items.filter((item) => historyKey(item) !== key),
      ].slice(0, SEARCH_HISTORY_LIMIT),
    async () => {
      try {
        const saved = await searchHistoryAPI.record(target);
        savedTemps.set(tempId, saved.id);
        const current = read(userId);
        if (current?.some((item) => item.id === tempId)) {
          write(
            userId,
            current.map((item) => (item.id === tempId ? { ...item, id: saved.id, usedAt: saved.usedAt } : item)),
          );
        }
      } catch (err) {
        savedTemps.set(tempId, null);
        throw err;
      }
    },
  );
}

export function removeSearch(userId: string, id: string): Promise<void> {
  return apply(
    userId,
    (items) => items.filter((item) => item.id !== id && item.id !== savedTemps.get(id)),
    async () => {
      const serverId = id.startsWith(TEMP_PREFIX) ? savedTemps.get(id) : id;
      if (serverId) await searchHistoryAPI.remove(serverId);
    },
  );
}

export function clearSearches(userId: string): Promise<void> {
  return apply(userId, () => [], () => searchHistoryAPI.clear());
}

export function refreshSearchHistory(): void {
  void queryClient.invalidateQueries({ queryKey: ["searchHistory"] });
}
