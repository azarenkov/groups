import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import StickerViewer from "../StickerViewer";
import { stickersAPI } from "../../api/stickers";
import { useStickerStore } from "../../store/stickerStore";
import { ApiError } from "../../../../shared/lib/apiError";
import { useReportModalStore } from "../../../moderation/store/reportModalStore";

jest.mock("../../api/stickers", () => ({
  stickersAPI: {
    mine: jest.fn(),
    pack: jest.fn(),
    install: jest.fn(),
    download: jest.fn(),
  },
}));

const sticker = { id: "s1", packId: "p1", emoji: "😺", width: 512, height: 512 };
const pack = {
  id: "p1",
  ownerId: "someone",
  name: "Cats",
  slug: "cats",
  installCount: 3,
  stickerCount: 1,
  stickers: [{ ...sticker, animated: false }],
};

beforeEach(() => {
  useStickerStore.getState().clear();
  (stickersAPI.mine as jest.Mock).mockResolvedValue([]);
  (stickersAPI.pack as jest.Mock).mockResolvedValue(pack);
  (stickersAPI.install as jest.Mock).mockResolvedValue(undefined);
  (stickersAPI.download as jest.Mock).mockReturnValue(new Promise(() => undefined));
});

it("shows the pack of the sticker and adds it", async () => {
  const onOpenPack = jest.fn();
  render(<StickerViewer sticker={sticker} onClose={() => undefined} onOpenPack={onOpenPack} />);
  expect(await screen.findByText("Cats")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Add pack" }));
  await waitFor(() => expect(stickersAPI.install).toHaveBeenCalledWith("p1"));
  await waitFor(() => expect(screen.queryByRole("button", { name: "Add pack" })).toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "Open pack" }));
  expect(onOpenPack).toHaveBeenCalledWith("p1");
});

it("says when the pack is gone", async () => {
  (stickersAPI.pack as jest.Mock).mockRejectedValueOnce(new ApiError("gone", 404, "STICKER_PACK_NOT_FOUND"));
  render(<StickerViewer sticker={sticker} onClose={() => undefined} onOpenPack={() => undefined} />);
  expect(await screen.findByText("Pack deleted")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Add pack" })).toBeNull();
});

it("offers a retry when the pack fails to load", async () => {
  (stickersAPI.pack as jest.Mock).mockRejectedValueOnce(new ApiError("down", 503));
  render(<StickerViewer sticker={sticker} onClose={() => undefined} onOpenPack={() => undefined} />);
  fireEvent.click(await screen.findByRole("button", { name: "Try again" }));
  expect(await screen.findByText("Cats")).toBeInTheDocument();
  expect(stickersAPI.pack).toHaveBeenCalledTimes(2);
});

it("reports the pack with the sticker message as evidence", async () => {
  const onClose = jest.fn();
  render(<StickerViewer sticker={sticker} messageId="m1" onClose={onClose} onOpenPack={() => undefined} />);
  fireEvent.click(await screen.findByRole("button", { name: "Report" }));
  expect(onClose).toHaveBeenCalled();
  expect(useReportModalStore.getState().target).toEqual({ type: "stickerPack", id: "p1", name: "Cats", messageId: "m1" });
});

it("shows that a cached pack was deleted when a refresh says so", async () => {
  const { queryClient, queryKeys } = jest.requireActual("../../../../shared/query/queryClient");
  render(<StickerViewer sticker={sticker} onClose={jest.fn()} onOpenPack={jest.fn()} />);
  await screen.findByRole("button", { name: "Add pack" });
  (stickersAPI.pack as jest.Mock).mockRejectedValueOnce(new ApiError("gone", 404, "STICKER_PACK_NOT_FOUND"));
  await act(async () => {
    await queryClient.refetchQueries({ queryKey: queryKeys.stickerPack("p1") });
  });
  expect(await screen.findByText("Pack deleted")).toBeTruthy();
});
