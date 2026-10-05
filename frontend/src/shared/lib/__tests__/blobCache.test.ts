import {
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

const blobOf = (size: number) => ({ size } as Blob);

beforeEach(() => {
  setBlobBackend(memoryBlobBackend());
});

afterAll(() => {
  setBlobBackend(null);
});

it("stores and returns blobs", async () => {
  const blob = blobOf(10);
  await writeCachedBlob("a", blob);
  expect(await readCachedBlob("a")).toBe(blob);
  await deleteCachedBlob("a");
  expect(await readCachedBlob("a")).toBeNull();
});

it("skips files that are too large to keep", async () => {
  await writeCachedBlob("huge", blobOf(BLOB_CACHE_MAX_ITEM_BYTES + 1));
  expect(await readCachedBlob("huge")).toBeNull();
});

it("evicts the least recently used blobs above the size limit", async () => {
  const size = 20 * 1024 * 1024;
  const count = Math.floor(BLOB_CACHE_MAX_BYTES / size) + 2;
  const now = jest.spyOn(Date, "now");
  for (let i = 0; i < count; i += 1) {
    now.mockReturnValue(i + 1);
    await writeCachedBlob(`b${i}`, blobOf(size));
  }
  now.mockReturnValue(count + 1);
  await readCachedBlob("b0");
  await new Promise((resolve) => setTimeout(resolve, 0));
  await evictBlobCacheNow();
  now.mockRestore();
  expect(await readCachedBlob("b0")).not.toBeNull();
  expect(await readCachedBlob("b1")).toBeNull();
  expect(await readCachedBlob("b2")).toBeNull();
  expect(await readCachedBlob("b3")).not.toBeNull();
  expect(await readCachedBlob(`b${count - 1}`)).not.toBeNull();
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
