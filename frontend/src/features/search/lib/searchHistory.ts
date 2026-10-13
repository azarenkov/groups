import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { historyTargetId, searchHistoryAPI, type HistoryItem, type HistoryTarget } from "../api/searchHistory";

export const SEARCH_HISTORY_LIMIT = 20;
const STORAGE_KEY = "rift.searchHistory.v1";
const TEMP_PREFIX = "pending:";

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

async function apply(
  userId: string,
  change: (items: HistoryItem[]) => HistoryItem[],
  request: () => Promise<void>,
): Promise<void> {
  void queryClient.cancelQueries({ queryKey: queryKeys.searchHistory(userId) }, { revert: false });
  const before = read(userId) ?? [];
  const optimistic = write(userId, change(before));
  try {
    await request();
  } catch (err) {
    if (read(userId) === optimistic) write(userId, before);
    else void queryClient.invalidateQueries({ queryKey: queryKeys.searchHistory(userId) });
    throw err;
  }
}

let tempSeq = 0;
const settledIds = new Map<string, Promise<string | null>>();
const pendingWrites = new Map<string, Set<Promise<unknown>>>();

const track = (userId: string, write: Promise<unknown>) => {
  const writes = pendingWrites.get(userId) ?? new Set<Promise<unknown>>();
  pendingWrites.set(userId, writes);
  writes.add(write);
  void write.catch(() => {}).finally(() => writes.delete(write));
};

export function recordSearch(userId: string, target: HistoryTarget): Promise<void> {
  const tempId = `${TEMP_PREFIX}${++tempSeq}`;
  const key = historyKey(target);
  let resolveId: (id: string | null) => void = () => {};
  settledIds.set(tempId, new Promise((resolve) => (resolveId = resolve)));
  const saving = apply(
    userId,
    (items) =>
      [
        { ...target, id: tempId, usedAt: new Date().toISOString() } as HistoryItem,
        ...items.filter((item) => historyKey(item) !== key),
      ].slice(0, SEARCH_HISTORY_LIMIT),
    async () => {
      try {
        const saved = await searchHistoryAPI.record(target);
        resolveId(saved.id);
        const current = read(userId);
        if (current?.some((item) => item.id === tempId)) {
          write(
            userId,
            current.map((item) => (item.id === tempId ? { ...item, id: saved.id, usedAt: saved.usedAt } : item)),
          );
        }
      } catch (err) {
        resolveId(null);
        throw err;
      }
    },
  ).finally(() => settledIds.delete(tempId));
  track(userId, saving);
  return saving;
}

export function removeSearch(userId: string, id: string): Promise<void> {
  const pending = settledIds.get(id);
  return apply(
    userId,
    (items) => items.filter((item) => item.id !== id),
    async () => {
      const serverId = pending ? await pending : id;
      if (serverId) await searchHistoryAPI.remove(serverId);
    },
  );
}

export function clearSearches(userId: string): Promise<void> {
  const earlier = [...(pendingWrites.get(userId) ?? [])];
  return apply(
    userId,
    () => [],
    async () => {
      await Promise.allSettled(earlier);
      await searchHistoryAPI.clear();
    },
  );
}

export function refreshSearchHistory(): void {
  void queryClient.invalidateQueries({ queryKey: ["searchHistory"] });
}
