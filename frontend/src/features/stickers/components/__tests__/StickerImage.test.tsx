import React from "react";
import { act, render, waitFor } from "@testing-library/react";
import StickerImage from "../StickerImage";
import { stickersAPI } from "../../api/stickers";

jest.mock("../../api/stickers", () => ({
  stickersAPI: { download: jest.fn() },
}));

type Callback = (entries: { isIntersecting: boolean }[]) => void;

let observers: Callback[] = [];

beforeEach(() => {
  observers = [];
  URL.createObjectURL = () => "blob:sticker";
  URL.revokeObjectURL = () => undefined;
  (stickersAPI.download as jest.Mock).mockResolvedValue(new Blob(["x"]));
  (window as unknown as { IntersectionObserver: unknown }).IntersectionObserver = class {
    constructor(callback: Callback) {
      observers.push(callback);
    }
    observe() {}
    disconnect() {}
  };
});

afterEach(() => {
  delete (window as unknown as { IntersectionObserver?: unknown }).IntersectionObserver;
});

const scroll = (isIntersecting: boolean) =>
  act(() => {
    observers.forEach((callback) => callback([{ isIntersecting }]));
  });

it("stops an animated sticker while it is off screen", async () => {
  const { container } = render(<StickerImage id="gif-1" emoji="😺" size={160} animated />);
  expect(container.querySelector("img")).toBeNull();
  scroll(true);
  await waitFor(() => expect(container.querySelector("img")).not.toBeNull());
  scroll(false);
  expect(container.querySelector("img")).toBeNull();
  expect(container.querySelector(".sticker-img-placeholder")).toBeNull();
  scroll(true);
  expect(container.querySelector("img")).not.toBeNull();
});

it("keeps a static sticker once it has been shown", async () => {
  const { container } = render(<StickerImage id="png-1" emoji="😺" size={160} />);
  scroll(true);
  await waitFor(() => expect(container.querySelector("img")).not.toBeNull());
  scroll(false);
  expect(container.querySelector("img")).not.toBeNull();
});
