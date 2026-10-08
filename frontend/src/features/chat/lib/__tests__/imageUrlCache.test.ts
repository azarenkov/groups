import { memoryBlobBackend, readCachedBlob, setBlobBackend, writeCachedBlob } from "../../../../shared/lib/blobCache";
import { acquireImageUrl, clearImageUrlCache, discardBrokenImageUrl, downloadImageUrl, downloadThumbnailUrl } from "../imageUrlCache";
import { attachmentAPI } from "../../api/attachment";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { download: jest.fn(), thumbnail: jest.fn() },
}));

const thumbnail = attachmentAPI.thumbnail as jest.Mock;
const download = attachmentAPI.download as jest.Mock;

beforeAll(() => {
  Object.assign(URL, {
    createObjectURL: jest.fn(() => `blob:${Math.random()}`),
    revokeObjectURL: jest.fn(),
  });
});

beforeEach(() => {
  setBlobBackend(memoryBlobBackend());
  thumbnail.mockReset();
});

afterAll(() => setBlobBackend(null));

it("serves a stored thumbnail without downloading it again", async () => {
  await writeCachedBlob("img:thumb:a1", { size: 3 } as Blob);
  await downloadThumbnailUrl("a1", 3, 10);
  expect(thumbnail).not.toHaveBeenCalled();
});

it("forgets a stored image that turned out to be broken", async () => {
  await writeCachedBlob("img:thumb:a2", { size: 3 } as Blob);
  const url = await downloadThumbnailUrl("a2", 3, 10);
  discardBrokenImageUrl("thumb:a2", url);
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(await readCachedBlob("img:thumb:a2")).toBeNull();
  thumbnail.mockResolvedValueOnce({ size: 4 } as Blob);
  await downloadThumbnailUrl("a2", 3, 10);
  expect(thumbnail).toHaveBeenCalledTimes(1);
});

it("forgets attachment URLs from the previous account on sign-out", async () => {
  (URL.createObjectURL as jest.Mock).mockImplementation(() => `blob:${Math.random()}`);
  download.mockResolvedValueOnce({ size: 3 } as Blob);
  const seen = await downloadImageUrl("a3", 3);
  let finish: (blob: Blob) => void = () => undefined;
  download.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const late = downloadImageUrl("a4", 3);
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearImageUrlCache();
  finish({ size: 3 } as Blob);
  await expect(late).rejects.toThrow();
  expect(URL.revokeObjectURL).toHaveBeenCalledWith(seen);
  expect(acquireImageUrl("a3")).toBeNull();
  expect(acquireImageUrl("a4")).toBeNull();
});
