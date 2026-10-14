import {
  blobCacheGeneration,
  BLOB_CACHE_MIN_BYTES,
  limitFromQuota,
  setBlobCacheLimit,
  type BlobBackend,
  BLOB_CACHE_MAX_BYTES,
  BLOB_CACHE_MAX_ITEM_BYTES,
  clearBlobCache,
  deleteCachedBlob,
  evictBlobCacheNow,
  memoryBlobBackend,
  readCachedBlob,
  setBlobBackend,
  writeCachedBlob,
} from "../blobCache";

const blobOf = (size: number) =>
  ({ size, type: "image/webp", arrayBuffer: () => Promise.resolve(new ArrayBuffer(size)) }) as unknown as Blob;

beforeEach(() => {
  setBlobBackend(memoryBlobBackend());
});

afterAll(() => {
  setBlobBackend(null);
});

it("stores and returns blobs", async () => {
  const blob = blobOf(10);
  await writeCachedBlob("a", blob);
  const read = await readCachedBlob("a");
  expect(read?.size).toBe(10);
  expect(read?.type).toBe("image/webp");
  await deleteCachedBlob("a");
  expect(await readCachedBlob("a")).toBeNull();
});

it("skips files that are too large to keep", async () => {
  await writeCachedBlob("huge", blobOf(BLOB_CACHE_MAX_ITEM_BYTES + 1));
  expect(await readCachedBlob("huge")).toBeNull();
});

it("evicts the least recently used blobs above the size limit", async () => {
  const size = 20 * 1024 * 1024;
  const fits = Math.floor(BLOB_CACHE_MAX_BYTES / size);
  const now = jest.spyOn(Date, "now");
  for (let i = 0; i < fits; i += 1) {
    now.mockReturnValue(i + 1);
    await writeCachedBlob(`b${i}`, blobOf(size));
  }
  now.mockReturnValue(fits + 1);
  await readCachedBlob("b0");
  await new Promise((resolve) => setTimeout(resolve, 0));
  for (let i = fits; i < fits + 2; i += 1) {
    now.mockReturnValue(i + 2);
    await writeCachedBlob(`b${i}`, blobOf(size));
  }
  await evictBlobCacheNow();
  now.mockRestore();
  expect(await readCachedBlob("b0")).not.toBeNull();
  expect(await readCachedBlob("b1")).toBeNull();
  expect(await readCachedBlob("b2")).toBeNull();
  expect(await readCachedBlob("b3")).not.toBeNull();
  expect(await readCachedBlob(`b${fits + 1}`)).not.toBeNull();
});

it("forgets everything on sign out, including writes still in flight", async () => {
  await writeCachedBlob("a", blobOf(1));
  const late = writeCachedBlob("b", blobOf(1));
  await clearBlobCache();
  await late;
  expect(await readCachedBlob("a")).toBeNull();
  expect(await readCachedBlob("b")).toBeNull();
});

it("works without a storage backend", async () => {
  setBlobBackend(null);
  await writeCachedBlob("a", blobOf(1));
  expect(await readCachedBlob("a")).toBeNull();
});

it("sizes the cache from the storage quota", () => {
  expect(limitFromQuota(undefined)).toBe(BLOB_CACHE_MAX_BYTES);
  expect(limitFromQuota(10 * 1024 * 1024 * 1024)).toBe(BLOB_CACHE_MAX_BYTES);
  expect(limitFromQuota(500 * 1024 * 1024)).toBe(50 * 1024 * 1024);
  expect(limitFromQuota(50 * 1024 * 1024)).toBe(BLOB_CACHE_MIN_BYTES);
});

it("keeps the cache under the quota-based limit", async () => {
  setBlobCacheLimit(30 * 1024 * 1024);
  const now = jest.spyOn(Date, "now");
  for (let i = 0; i < 3; i += 1) {
    now.mockReturnValue(i + 1);
    await writeCachedBlob(`q${i}`, blobOf(12 * 1024 * 1024));
  }
  await evictBlobCacheNow();
  now.mockRestore();
  expect(await readCachedBlob("q0")).toBeNull();
  expect(await readCachedBlob("q2")).not.toBeNull();
});

it("frees space and retries when the browser runs out of quota", async () => {
  const memory = memoryBlobBackend();
  let failures = 1;
  const flaky: BlobBackend = {
    ...memory,
    put: async (record) => {
      if (record.key === "big" && failures > 0) {
        failures -= 1;
        throw new DOMException("full", "QuotaExceededError");
      }
      await memory.put(record);
    },
  };
  setBlobBackend(flaky);
  await writeCachedBlob("old", blobOf(10));
  await writeCachedBlob("big", blobOf(10));
  expect(await readCachedBlob("big")).not.toBeNull();
  expect(await readCachedBlob("old")).toBeNull();
});

it("stops writing for the session when storage keeps failing", async () => {
  const memory = memoryBlobBackend();
  const putSpy = jest.fn(async () => {
    throw new DOMException("full", "QuotaExceededError");
  });
  setBlobBackend({ ...memory, put: putSpy });
  await writeCachedBlob("a", blobOf(10));
  const calls = putSpy.mock.calls.length;
  await writeCachedBlob("b", blobOf(10));
  expect(putSpy.mock.calls.length).toBe(calls);
});

it("keeps the size under the limit after every single write", async () => {
  setBlobCacheLimit(25 * 1024 * 1024);
  const now = jest.spyOn(Date, "now");
  for (let i = 0; i < 4; i += 1) {
    now.mockReturnValue(i + 1);
    await writeCachedBlob(`w${i}`, blobOf(10 * 1024 * 1024));
  }
  now.mockRestore();
  const kept = [];
  for (let i = 0; i < 4; i += 1) if (await readCachedBlob(`w${i}`)) kept.push(i);
  expect(kept).toEqual([2, 3]);
});

it("does not bring a blob back when sign out happens during a read", async () => {
  const memory = memoryBlobBackend();
  let releaseGet: () => void = () => undefined;
  const slow: BlobBackend = {
    ...memory,
    get: async (key) => {
      const record = await memory.get(key);
      await new Promise<void>((resolve) => (releaseGet = resolve));
      return record;
    },
  };
  setBlobBackend(slow);
  await memory.put({ key: "secret", blob: blobOf(1), size: 1, usedAt: 1 });
  const reading = readCachedBlob("secret");
  await Promise.resolve();
  await clearBlobCache();
  releaseGet();
  expect(await reading).toBeNull();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(await memory.get("secret")).toBeUndefined();
});

it("does not store a download that started before sign out", async () => {
  const startedAt = blobCacheGeneration();
  await clearBlobCache();
  await writeCachedBlob("private", blobOf(1), startedAt);
  expect(await readCachedBlob("private")).toBeNull();
  await writeCachedBlob("fresh", blobOf(1), blobCacheGeneration());
  expect(await readCachedBlob("fresh")).not.toBeNull();
});

it("does not let a read's usage update bring back a blob deleted meanwhile", async () => {
  const memory = memoryBlobBackend();
  let releaseGet: () => void = () => undefined;
  let slow = true;
  setBlobBackend({
    ...memory,
    get: async (key) => {
      const record = await memory.get(key);
      if (slow) {
        slow = false;
        await new Promise<void>((resolve) => (releaseGet = resolve));
      }
      return record;
    },
  });
  await memory.put({ key: "avatar", blob: blobOf(1), size: 1, usedAt: 1 });
  const reading = readCachedBlob("avatar");
  await Promise.resolve();
  await deleteCachedBlob("avatar");
  releaseGet();
  await reading;
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(await memory.get("avatar")).toBeUndefined();
});

it("does not let a read's usage update overwrite a newer blob", async () => {
  const memory = memoryBlobBackend();
  let releaseGet: () => void = () => undefined;
  let slow = true;
  setBlobBackend({
    ...memory,
    get: async (key) => {
      const record = await memory.get(key);
      if (slow) {
        slow = false;
        await new Promise<void>((resolve) => (releaseGet = resolve));
      }
      return record;
    },
  });
  const oldBlob = blobOf(1);
  const newBlob = blobOf(2);
  await memory.put({ key: "avatar", blob: oldBlob, size: 1, usedAt: 1 });
  const reading = readCachedBlob("avatar");
  await Promise.resolve();
  await writeCachedBlob("avatar", newBlob);
  releaseGet();
  await reading;
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect((await memory.get("avatar"))?.blob).toBe(newBlob);
});

it("drops a cached blob whose data can no longer be read", async () => {
  const broken = { size: 10, type: "image/webp", arrayBuffer: () => Promise.reject(new Error("WebKitBlobResource")) } as unknown as Blob;
  await writeCachedBlob("lost", broken);
  expect(await readCachedBlob("lost")).toBeNull();
  await writeCachedBlob("lost", blobOf(10));
  expect((await readCachedBlob("lost"))?.size).toBe(10);
});
