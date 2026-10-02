import { act, renderHook, waitFor } from "@testing-library/react";

import { stickersAPI } from "../../api/stickers";
import { useStickerUrl } from "../stickerImages";

jest.mock("../../api/stickers", () => ({
  stickersAPI: { download: jest.fn() },
}));

describe("useStickerUrl", () => {
  beforeAll(() => {
    URL.createObjectURL = () => "blob:sticker";
    URL.revokeObjectURL = () => undefined;
  });

  it("tries again after a failed download while the sticker is shown", async () => {
    jest.useFakeTimers();
    try {
      const download = stickersAPI.download as jest.Mock;
      download.mockRejectedValueOnce(new Error("offline")).mockResolvedValueOnce(new Blob(["x"]));
      const { result } = renderHook(() => useStickerUrl("sticker-1"));
      await act(async () => {
        for (let i = 0; i < 5; i += 1) await Promise.resolve();
      });
      expect(result.current).toBeNull();
      await act(async () => {
        jest.advanceTimersByTime(2000);
        for (let i = 0; i < 5; i += 1) await Promise.resolve();
      });
      await waitFor(() => expect(result.current).toBe("blob:sticker"));
      expect(download).toHaveBeenCalledTimes(2);
    } finally {
      jest.useRealTimers();
    }
  });

  it("waits until the sticker is visible before downloading it", async () => {
    const download = stickersAPI.download as jest.Mock;
    download.mockResolvedValue(new Blob(["x"]));
    const { result, rerender } = renderHook(({ enabled }) => useStickerUrl("sticker-lazy", enabled), {
      initialProps: { enabled: false },
    });
    await act(async () => {
      for (let i = 0; i < 5; i += 1) await Promise.resolve();
    });
    expect(download).not.toHaveBeenCalled();
    rerender({ enabled: true });
    await waitFor(() => expect(result.current).toBe("blob:sticker"));
    expect(download).toHaveBeenCalledTimes(1);
  });

  it("downloads at most six stickers at once", async () => {
    const download = stickersAPI.download as jest.Mock;
    const finish: Array<() => void> = [];
    download.mockImplementation(
      () => new Promise<Blob>((resolve) => finish.push(() => resolve(new Blob(["x"])))),
    );
    const hooks = Array.from({ length: 10 }, (_, i) => renderHook(() => useStickerUrl(`sticker-many-${i}`)));
    await act(async () => {
      for (let i = 0; i < 5; i += 1) await Promise.resolve();
    });
    expect(download).toHaveBeenCalledTimes(6);
    await act(async () => {
      finish.splice(0).forEach((done) => done());
      for (let i = 0; i < 10; i += 1) await Promise.resolve();
    });
    expect(download).toHaveBeenCalledTimes(10);
    await act(async () => {
      finish.splice(0).forEach((done) => done());
      for (let i = 0; i < 10; i += 1) await Promise.resolve();
    });
    await waitFor(() => expect(hooks.every((hook) => hook.result.current === "blob:sticker")).toBe(true));
  });
});
