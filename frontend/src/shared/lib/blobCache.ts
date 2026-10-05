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
  writesSinceEvict = 0;
};

export const evictBlobCacheNow = async (): Promise<void> => {
  const store = current();
  if (store) await evict(store, "");
};

let generation = 0;
let writesSinceEvict = 0;
const EVICT_EVERY_WRITES = 20;

export async function readCachedBlob(key: string): Promise<Blob | null> {
  const store = current();
  if (!store) return null;
  try {
    const record = await store.get(key);
    if (!record) return null;
    void store.put({ ...record, usedAt: Date.now() }).catch(() => undefined);
    return record.blob;
  } catch {
    return null;
  }
}

async function evict(store: BlobBackend, keep: string): Promise<void> {
  const records = await store.all();
  let total = records.reduce((sum, r) => sum + r.size, 0);
  if (total <= BLOB_CACHE_MAX_BYTES) return;
  for (const record of records.sort((a, b) => a.usedAt - b.usedAt)) {
    if (total <= BLOB_CACHE_MAX_BYTES) break;
    if (record.key === keep) continue;
    await store.delete(record.key);
    total -= record.size;
  }
}

export async function writeCachedBlob(key: string, blob: Blob): Promise<void> {
  const store = current();
  if (!store || blob.size > BLOB_CACHE_MAX_ITEM_BYTES) return;
  const gen = generation;
  try {
    if (gen !== generation) return;
    await store.put({ key, blob, size: blob.size, usedAt: Date.now() });
    if (gen !== generation) {
      await store.delete(key);
      return;
    }
    writesSinceEvict += 1;
    if (writesSinceEvict < EVICT_EVERY_WRITES) return;
    writesSinceEvict = 0;
    await evict(store, key);
  } catch {
    return;
  }
}

export async function deleteCachedBlob(key: string): Promise<void> {
  const store = current();
  if (!store) return;
  try {
    await store.delete(key);
  } catch {
    return;
  }
}

export async function clearBlobCache(): Promise<void> {
  generation += 1;
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
