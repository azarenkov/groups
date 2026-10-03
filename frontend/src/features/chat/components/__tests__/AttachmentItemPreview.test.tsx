import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import AttachmentItem from "../AttachmentItem";
import { attachmentAPI } from "../../api/attachment";
import { imageUrlCache } from "../../lib/imageUrlCache";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { thumbnail: jest.fn(), download: jest.fn() },
}));

const thumbnail = attachmentAPI.thumbnail as jest.Mock;
const download = attachmentAPI.download as jest.Mock;

const attachment = (id: string, contentType: string, sizeBytes = 1000) => ({
  id,
  fileName: `${id}.bin`,
  contentType,
  sizeBytes,
  createdAt: "2026-10-01T00:00:00Z",
});

let created = 0;

beforeEach(() => {
  thumbnail.mockReset();
  download.mockReset();
  imageUrlCache.clear();
  created = 0;
  URL.createObjectURL = () => `blob:${++created}`;
  URL.revokeObjectURL = () => undefined;
});

it("shows the server thumbnail in the grid and the original in the viewer", async () => {
  thumbnail.mockResolvedValue(new Blob(["t"]));
  download.mockResolvedValue(new Blob(["o"]));
  render(<AttachmentItem attachment={attachment("img1", "image/png", 20 * 1024 * 1024)} thumbnail />);
  const img = await screen.findByRole("img");
  expect(thumbnail).toHaveBeenCalledWith("img1", 20 * 1024 * 1024);
  expect(download).not.toHaveBeenCalled();
  fireEvent.click(img.closest("button")!);
  await waitFor(() => expect(download).toHaveBeenCalledWith("img1", 20 * 1024 * 1024));
});

it("falls back to the original image when there is no thumbnail", async () => {
  thumbnail.mockRejectedValue(new Error("404"));
  download.mockResolvedValue(new Blob(["o"]));
  render(<AttachmentItem attachment={attachment("img2", "image/png")} thumbnail />);
  expect(await screen.findByRole("img")).toBeInTheDocument();
  expect(download).toHaveBeenCalledWith("img2", 1000);
});

it("plays videos in a viewer instead of only downloading them", async () => {
  download.mockResolvedValue(new Blob(["v"]));
  render(<AttachmentItem attachment={attachment("vid1", "video/mp4")} />);
  fireEvent.click(screen.getByRole("button", { name: /vid1\.bin/ }));
  await waitFor(() => expect(document.querySelector("video")).not.toBeNull());
  expect(download).toHaveBeenCalledWith("vid1", 1000, expect.any(AbortSignal));
});
