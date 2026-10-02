import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import StickerViewer from "../StickerViewer";
import { stickersAPI } from "../../api/stickers";
import { useStickerStore } from "../../store/stickerStore";

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
  fireEvent.click(screen.getByRole("button", { name: "addPack" }));
  await waitFor(() => expect(stickersAPI.install).toHaveBeenCalledWith("p1"));
  await waitFor(() => expect(screen.queryByRole("button", { name: "addPack" })).toBeNull());
  fireEvent.click(screen.getByRole("button", { name: "openPack" }));
  expect(onOpenPack).toHaveBeenCalledWith("p1");
});

it("says when the pack is gone", async () => {
  (stickersAPI.pack as jest.Mock).mockRejectedValueOnce(new Error("not found"));
  render(<StickerViewer sticker={sticker} onClose={() => undefined} onOpenPack={() => undefined} />);
  expect(await screen.findByText("packGone")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "addPack" })).toBeNull();
});
