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
    rename: jest.fn(),
    reorder: jest.fn(),
    setCover: jest.fn(),
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

it("keeps selecting and removing a sticker as separate buttons", async () => {
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
  render(<StickerPacksModal onClose={() => undefined} packId="p1" />);
  const remove = await screen.findByRole("button", { name: "Remove sticker" });
  const select = screen.getByRole("button", { name: "😺" });
  expect(select.contains(remove)).toBe(false);
  expect(remove.closest("[role=button]")).toBeNull();
  fireEvent.click(select);
  expect(select).toHaveAttribute("aria-pressed", "true");
  expect(document.querySelector(".sticker-pack-cell.selected")).not.toBeNull();
});

it("lets the owner rename the pack, move a sticker and pick the cover", async () => {
  useAuthStore.setState({ user: { id: "u1" } } as never);
  const stickers = ["s1", "s2", "s3"].map((id) => ({
    id,
    packId: "p1",
    emoji: "😺",
    animated: false,
    width: 512,
    height: 512,
  }));
  (stickersAPI.pack as jest.Mock).mockResolvedValueOnce({
    id: "p1",
    ownerId: "u1",
    name: "Cats",
    slug: "cats",
    installCount: 1,
    stickerCount: 3,
    stickers,
  });
  (stickersAPI.rename as jest.Mock).mockResolvedValue(undefined);
  (stickersAPI.reorder as jest.Mock).mockResolvedValue(undefined);
  (stickersAPI.setCover as jest.Mock).mockResolvedValue(undefined);
  render(<StickerPacksModal onClose={() => undefined} packId="cats" />);

  fireEvent.click(await screen.findByRole("button", { name: "Rename" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Kittens" } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  expect(await screen.findByText("Kittens")).toBeInTheDocument();
  expect(stickersAPI.rename).toHaveBeenCalledWith("p1", "Kittens");

  fireEvent.click(screen.getAllByRole("button", { name: "😺" })[2]);
  fireEvent.click(screen.getByRole("button", { name: "Move left" }));
  await waitFor(() => expect(stickersAPI.reorder).toHaveBeenCalledWith("p1", ["s1", "s3", "s2"]));

  await waitFor(() => expect(screen.getByRole("button", { name: "Make cover" })).not.toBeDisabled());
  fireEvent.click(screen.getByRole("button", { name: "Make cover" }));
  await waitFor(() => expect(stickersAPI.setCover).toHaveBeenCalledWith("p1", "s3"));
  await waitFor(() =>
    expect(document.querySelectorAll(".sticker-pack-cell")[1].querySelector(".sticker-pack-cover-badge")).not.toBeNull(),
  );
  expect(document.querySelectorAll(".sticker-pack-cover-badge")).toHaveLength(1);
});

