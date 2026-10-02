import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import StickerPacksModal from "../StickerPacksModal";
import { stickersAPI } from "../../api/stickers";
import { useStickerStore } from "../../store/stickerStore";
import { useAuthStore } from "../../../auth/store/authStore";

jest.mock("../../api/stickers", () => ({
  stickersAPI: {
    mine: jest.fn(),
    search: jest.fn(),
    create: jest.fn(),
    pack: jest.fn(),
    removeSticker: jest.fn(),
  },
}));

beforeEach(() => {
  (stickersAPI.mine as jest.Mock).mockResolvedValue([]);
  (stickersAPI.search as jest.Mock).mockResolvedValue([]);
  (stickersAPI.create as jest.Mock).mockResolvedValue({
    id: "p1",
    ownerId: "u1",
    name: "Cats",
    slug: "cats",
    installCount: 1,
    stickerCount: 0,
    stickers: [],
  });
  (stickersAPI.pack as jest.Mock).mockReturnValue(new Promise(() => undefined));
});

it("creates a pack without submitting the surrounding composer form", async () => {
  const outer = jest.fn((e: React.FormEvent) => e.preventDefault());
  render(
    <form onSubmit={outer}>
      <StickerPacksModal onClose={() => undefined} />
    </form>,
  );
  fireEvent.click(screen.getAllByRole("tab")[2]);
  const nameInput = document.querySelector<HTMLInputElement>(".sticker-create input")!;
  fireEvent.change(nameInput, { target: { value: "Cats" } });
  fireEvent.submit(document.querySelector(".sticker-create")!);
  await waitFor(() => expect(stickersAPI.create).toHaveBeenCalled());
  expect(outer).not.toHaveBeenCalled();
});

it("keeps a pack created before an account switch out of the next account's stickers", async () => {
  let finishCreate: (pack: unknown) => void = () => undefined;
  (stickersAPI.create as jest.Mock).mockReturnValueOnce(new Promise((resolve) => (finishCreate = resolve)));
  await useStickerStore.getState().load("u1");
  render(<StickerPacksModal onClose={() => undefined} />);
  fireEvent.click(screen.getAllByRole("tab")[2]);
  fireEvent.change(document.querySelector<HTMLInputElement>(".sticker-create input")!, { target: { value: "Cats" } });
  fireEvent.submit(document.querySelector(".sticker-create")!);
  await waitFor(() => expect(stickersAPI.create).toHaveBeenCalled());
  await useStickerStore.getState().load("u2");
  finishCreate({ id: "p1", ownerId: "u1", name: "Cats", slug: "cats", installCount: 1, stickerCount: 0, stickers: [] });
  await waitFor(() => expect(document.querySelector(".sticker-create button[type=submit]:disabled")).toBeNull());
  expect(useStickerStore.getState().packs).toEqual([]);
});

it("removes a sticker from the keyboard without toggling the selection", async () => {
  useAuthStore.setState({ user: { id: "u1" } } as never);
  (stickersAPI.pack as jest.Mock).mockResolvedValueOnce({
    id: "p1",
    ownerId: "u1",
    name: "Cats",
    slug: "cats",
    installCount: 1,
    stickerCount: 1,
    stickers: [{ id: "s1", packId: "p1", emoji: "😺", animated: false, width: 512, height: 512 }],
  });
  (stickersAPI.removeSticker as jest.Mock).mockResolvedValueOnce(undefined);
  render(<StickerPacksModal onClose={() => undefined} packId="p1" />);
  const remove = await screen.findByRole("button", { name: "Remove sticker" });
  const keydown = fireEvent.keyDown(remove, { key: "Enter" });
  expect(keydown).toBe(true);
  expect(document.querySelector(".sticker-pack-cell.selected")).toBeNull();
});
