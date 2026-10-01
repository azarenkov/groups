import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import StickerPacksModal from "../StickerPacksModal";
import { stickersAPI } from "../../api/stickers";

jest.mock("../../api/stickers", () => ({
  stickersAPI: {
    mine: jest.fn(),
    search: jest.fn(),
    create: jest.fn(),
    pack: jest.fn(),
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
