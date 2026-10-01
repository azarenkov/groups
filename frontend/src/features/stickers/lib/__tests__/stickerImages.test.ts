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
});
