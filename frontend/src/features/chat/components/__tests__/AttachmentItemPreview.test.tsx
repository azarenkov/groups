import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import AttachmentItem from "../AttachmentItem";
import { attachmentAPI } from "../../api/attachment";
import { imageUrlCache } from "../../lib/imageUrlCache";
import { extractVideoPoster, PosterExtractionError } from "../../lib/videoPoster";
import { ApiError } from "../../../../shared/lib/apiError";

jest.mock("../../api/attachment", () => ({
  attachmentAPI: { thumbnail: jest.fn(), download: jest.fn() },
}));

jest.mock("../../lib/videoPoster", () => {
  class PosterExtractionError extends Error {}
  return { extractVideoPoster: jest.fn(), PosterExtractionError };
});

const thumbnail = attachmentAPI.thumbnail as jest.Mock;
const download = attachmentAPI.download as jest.Mock;
const extractPoster = extractVideoPoster as jest.Mock;

const attachment = (id: string, contentType: string, sizeBytes = 1000) => ({
  id,
  fileName: `${id}.bin`,
  contentType,
  sizeBytes,
  createdAt: "2026-10-01T00:00:00Z",
});

let created = 0;

beforeEach(() => {
  HTMLMediaElement.prototype.canPlayType = (type: string) => (type === "video/mp4" ? "probably" : "");
  thumbnail.mockReset();
  download.mockReset();
  extractPoster.mockReset();
  extractPoster.mockRejectedValue(new PosterExtractionError("no frames"));
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
  let revoked = "";
  URL.revokeObjectURL = (url: string) => {
    revoked = url;
  };
  fireEvent.click(img.closest("button")!);
  await waitFor(() => expect(download).toHaveBeenCalledWith("img1", 20 * 1024 * 1024, expect.any(AbortSignal)));
  await waitFor(() => expect(document.querySelector(".image-viewer-overlay img")?.getAttribute("src")).toBe("blob:2"));
  fireEvent.click(document.querySelector(".image-viewer-overlay")!);
  await waitFor(() => expect(revoked).toBe("blob:2"));
  expect(imageUrlCache.has("img1")).toBe(false);
});

it("falls back to the original image when there is no thumbnail", async () => {
  thumbnail.mockRejectedValue(new Error("404"));
  download.mockResolvedValue(new Blob(["o"]));
  render(<AttachmentItem attachment={attachment("img2", "image/png")} thumbnail />);
  expect(await screen.findByRole("img")).toBeInTheDocument();
  expect(download).toHaveBeenCalledWith("img2", 1000);
});

it("keeps formats the browser cannot play as a download", () => {
  render(<AttachmentItem attachment={attachment("avi1", "video/x-msvideo")} />);
  expect(document.querySelector(".msg-attach-video")).toBeNull();
  expect(screen.getByRole("button", { name: /avi1\.bin/ })).toBeInTheDocument();
});

it("plays videos in a viewer instead of only downloading them", async () => {
  download.mockResolvedValue(new Blob(["v"]));
  render(<AttachmentItem attachment={attachment("vid1", "video/mp4")} />);
  fireEvent.click(screen.getByRole("button", { name: /vid1\.bin/ }));
  await waitFor(() => expect(document.querySelector("video")).not.toBeNull());
  expect(download).toHaveBeenCalledWith("vid1", 1000, expect.any(AbortSignal));
});

it("shows a file row at once when a large image has no thumbnail", async () => {
  thumbnail.mockRejectedValue(new ApiError("no thumbnail", 404, "ATTACHMENT_THUMBNAIL_UNAVAILABLE"));
  render(<AttachmentItem attachment={attachment("big1", "image/tiff", 20 * 1024 * 1024)} thumbnail />);
  expect(await screen.findByText("big1.bin")).toBeInTheDocument();
  expect(thumbnail).toHaveBeenCalledTimes(1);
  expect(download).not.toHaveBeenCalled();
});


it("shows the first frame of a video as its preview", async () => {
  download.mockResolvedValue(new Blob(["v"]));
  extractPoster.mockResolvedValue(new Blob(["p"], { type: "image/jpeg" }));
  render(<AttachmentItem attachment={attachment("vid2", "video/mp4")} thumbnail />);
  await waitFor(() => expect(document.querySelector(".msg-attach-video-poster img")).not.toBeNull());
  expect(extractPoster).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "vid2.bin" })).toBeInTheDocument();
});

it("falls back to the video row when no frame can be extracted", async () => {
  download.mockResolvedValue(new Blob(["v"]));
  render(<AttachmentItem attachment={attachment("vid3", "video/mp4")} />);
  expect(await screen.findByText("vid3.bin")).toBeInTheDocument();
  expect(document.querySelector(".msg-attach-video-poster")).toBeNull();
});

it("keeps the video preview and retries when the download fails for a moment", async () => {
  jest.useFakeTimers();
  download.mockRejectedValueOnce(new ApiError("server error", 503, "UNAVAILABLE")).mockResolvedValue(new Blob(["v"]));
  extractPoster.mockResolvedValue(new Blob(["p"], { type: "image/jpeg" }));
  render(<AttachmentItem attachment={attachment("vid4", "video/mp4")} />);
  await act(async () => {
    await Promise.resolve();
  });
  expect(document.querySelector(".msg-attach-video-poster")).not.toBeNull();
  await act(async () => {
    jest.advanceTimersByTime(1600);
  });
  jest.useRealTimers();
  await waitFor(() => expect(document.querySelector(".msg-attach-video-poster img")).not.toBeNull());
  expect(download).toHaveBeenCalledTimes(2);
});
