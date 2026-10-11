export const BLOB_CACHE_MAX_BYTES = 200 * 1024 * 1024;
export const BLOB_CACHE_MAX_ITEM_BYTES = 25 * 1024 * 1024;

export interface BlobRecord {
  key: string;
  blob: Blob;
  size: number;
  usedAt: number;
}

export interface BlobBackend {
  get(key: string): Promise<BlobRecord | undefined>;
  put(record: BlobRecord): Promise<void>;
  delete(key: string): Promise<void>;
  all(): Promise<Omit<BlobRecord, "blob">[]>;
  clear(): Promise<void>;
}

const DB_NAME = "rift-media";
const STORE = "blobs";

const request = <T>(req: IDBRequest<T>): Promise<T> =>
  new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });

function indexedDbBackend(): BlobBackend | null {
  if (typeof indexedDB === "undefined") return null;
  let opening: Promise<IDBDatabase> | null = null;
  const db = () => {
    if (!opening) {
      opening = new Promise((resolve, reject) => {
        const req = indexedDB.open(DB_NAME, 1);
        req.onupgradeneeded = () => {
          if (!req.result.objectStoreNames.contains(STORE)) req.result.createObjectStore(STORE, { keyPath: "key" });
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => {
          opening = null;
          reject(req.error);
        };
      });
    }
    return opening;
  };
  const store = async (mode: IDBTransactionMode) => (await db()).transaction(STORE, mode).objectStore(STORE);
  return {
    get: async (key) => (await request((await store("readonly")).get(key))) as BlobRecord | undefined,
    put: async (record) => {
      await request((await store("readwrite")).put(record));
    },
    delete: async (key) => {
      await request((await store("readwrite")).delete(key));
    },
    all: async () => {
      const records = (await request((await store("readonly")).getAll())) as BlobRecord[];
      return records.map(({ key, size, usedAt }) => ({ key, size, usedAt }));
    },
    clear: async () => {
      await request((await store("readwrite")).clear());
    },
  };
}

let backend: BlobBackend | null | undefined;
const current = (): BlobBackend | null => {
  if (backend === undefined) backend = indexedDbBackend();
  return backend;
};

export const setBlobBackend = (next: BlobBackend | null): void => {
  backend = next;
  knownTotal = null;
  disabled = false;
  limitBytes = null;
  limitResolving = null;
};

export const setBlobCacheLimit = (bytes: number | null): void => {
  limitBytes = bytes;
  limitResolving = null;
};

export const evictBlobCacheNow = async (): Promise<void> => {
  const store = current();
  if (store) await sizeChanging(() => evict(store, ""));
};

let generation = 0;
let knownTotal: number | null = null;
let disabled = false;
let limitBytes: number | null = null;
let limitResolving: Promise<number> | null = null;

export const BLOB_CACHE_MIN_BYTES = 20 * 1024 * 1024;
export const BLOB_CACHE_QUOTA_SHARE = 0.1;

export function limitFromQuota(quota: number | undefined): number {
  if (!quota || !Number.isFinite(quota) || quota <= 0) return BLOB_CACHE_MAX_BYTES;
  return Math.max(BLOB_CACHE_MIN_BYTES, Math.min(BLOB_CACHE_MAX_BYTES, Math.floor(quota * BLOB_CACHE_QUOTA_SHARE)));
}

const resolveLimit = (): Promise<number> => {
  if (limitBytes !== null) return Promise.resolve(limitBytes);
  if (!limitResolving) {
    limitResolving = (async () => {
      try {
        const storage = typeof navigator !== "undefined" ? navigator.storage : undefined;
        if (storage && typeof window !== "undefined" && (window as { electron?: unknown }).electron && storage.persist) {
          await storage.persist().catch(() => false);
        }
        const estimate = storage?.estimate ? await storage.estimate() : undefined;
        limitBytes = limitFromQuota(estimate?.quota);
      } catch {
        limitBytes = BLOB_CACHE_MAX_BYTES;
      }
      return limitBytes;
    })();
  }
  return limitResolving;
};

const keyVersions = new Map<string, number>();
const keyChains = new Map<string, Promise<unknown>>();

const bumpKey = (key: string) => keyVersions.set(key, (keyVersions.get(key) ?? 0) + 1);

let sizeChain: Promise<unknown> = Promise.resolve();

function sizeChanging<T>(task: () => Promise<T>): Promise<T> {
  const run = sizeChain.then(task, task);
  sizeChain = run.catch(() => undefined);
  return run;
}

function forKey<T>(key: string, task: () => Promise<T>): Promise<T> {
  const previous = keyChains.get(key) ?? Promise.resolve();
  const run = previous.then(task, task);
  const tail = run.catch(() => undefined);
  keyChains.set(key, tail);
  void tail.then(() => {
    if (keyChains.get(key) === tail) keyChains.delete(key);
  });
  return run;
}

function touch(store: BlobBackend, record: BlobRecord, gen: number, version: number | undefined): Promise<void> {
  return forKey(record.key, async () => {
    if (gen !== generation || keyVersions.get(record.key) !== version) return;
    try {
      await store.put({ ...record, usedAt: Date.now() });
      if (gen !== generation) await store.delete(record.key);
    } catch {
      return;
    }
  });
}

const readBytes = (blob: Blob): Promise<ArrayBuffer> =>
  typeof blob.arrayBuffer === "function"
    ? blob.arrayBuffer()
    : new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as ArrayBuffer);
        reader.onerror = () => reject(reader.error);
        reader.readAsArrayBuffer(blob);
      });

export async function readCachedBlob(key: string): Promise<Blob | null> {
  const store = current();
  if (!store) return null;
  const gen = generation;
  const version = keyVersions.get(key);
  let record: BlobRecord | undefined;
  try {
    record = await store.get(key);
  } catch {
    return null;
  }
  if (!record || gen !== generation) return null;
  let bytes: ArrayBuffer;
  try {
    bytes = await readBytes(record.blob);
  } catch {
    bytes = new ArrayBuffer(0);
  }
  if (gen !== generation) return null;
  if (bytes.byteLength === 0 || bytes.byteLength !== record.blob.size) {
    void deleteCachedBlob(key);
    return null;
  }
  void touch(store, record, gen, version);
  return new Blob([bytes], { type: record.blob.type });
}

async function evict(store: BlobBackend, keep: string, target?: number): Promise<void> {
  const limit = target ?? (await resolveLimit());
  const records = await store.all();
  let total = records.reduce((sum, r) => sum + r.size, 0);
  if (total <= limit) {
    knownTotal = total;
    return;
  }
  for (const record of records.sort((a, b) => a.usedAt - b.usedAt)) {
    if (total <= limit) break;
    if (record.key === keep) continue;
    bumpKey(record.key);
    await store.delete(record.key);
    total -= record.size;
  }
  knownTotal = total;
}

async function totalSize(store: BlobBackend): Promise<number> {
  if (knownTotal === null) {
    const records = await store.all();
    knownTotal = records.reduce((sum, r) => sum + r.size, 0);
  }
  return knownTotal;
}

export const blobCacheGeneration = (): number => generation;

export function writeCachedBlob(key: string, blob: Blob, startedAt: number = generation): Promise<void> {
  const store = current();
  if (!store || disabled || blob.size > BLOB_CACHE_MAX_ITEM_BYTES || startedAt !== generation) return Promise.resolve();
  bumpKey(key);
  const gen = generation;
  return forKey(key, () => sizeChanging(() => storeBlob(store, key, blob, gen)));
}

async function storeBlob(store: BlobBackend, key: string, blob: Blob, gen: number): Promise<void> {
  try {
    const limit = await resolveLimit();
    if (gen !== generation || blob.size > limit) return;
    const record = { key, blob, size: blob.size, usedAt: Date.now() };
    const previous = await store.get(key);
    const before = await totalSize(store);
    try {
      await store.put(record);
    } catch {
      await evict(store, key, Math.floor(before / 2));
      try {
        await store.put(record);
      } catch {
        disabled = true;
        knownTotal = null;
        return;
      }
    }
    if (gen !== generation) {
      await store.delete(key);
      return;
    }
    knownTotal = (knownTotal ?? before) + blob.size - (previous?.size ?? 0);
    if (knownTotal > limit) await evict(store, key);
  } catch {
    knownTotal = null;
    return;
  }
}

export function deleteCachedBlob(key: string): Promise<void> {
  const store = current();
  if (!store) return Promise.resolve();
  bumpKey(key);
  return forKey(key, () =>
    sizeChanging(async () => {
      try {
        const previous = await store.get(key);
        await store.delete(key);
        if (previous && knownTotal !== null) knownTotal = Math.max(0, knownTotal - previous.size);
      } catch {
        return;
      }
    }),
  );
}

export async function clearBlobCache(): Promise<void> {
  generation += 1;
  knownTotal = 0;
  keyVersions.clear();
  const store = current();
  if (!store) return;
  try {
    await store.clear();
  } catch {
    return;
  }
}

export function memoryBlobBackend(): BlobBackend {
  const records = new Map<string, BlobRecord>();
  return {
    get: async (key) => records.get(key),
    put: async (record) => {
      records.set(record.key, record);
    },
    delete: async (key) => {
      records.delete(key);
    },
    all: async () => Array.from(records.values()).map(({ key, size, usedAt }) => ({ key, size, usedAt })),
    clear: async () => {
      records.clear();
    },
  };
}
