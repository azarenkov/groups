import { applyPalette, GIFEncoder, quantize } from "gifenc";

import type { CropRect } from "./imageCrop";
import { frameTimes, GIF_FRAME_MS, GIF_FRAME_STEPS, GIF_MAX_BYTES, GIF_SIZE } from "./videoGif";

export class VideoGifError extends Error {
  constructor(readonly code: "unreadable" | "tooLarge" | "aborted") {
    super(code);
  }
}

const abortError = () => new VideoGifError("aborted");

const waitFor = (video: HTMLVideoElement, event: string, signal?: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const cleanup = () => {
      video.removeEventListener(event, done);
      video.removeEventListener("error", fail);
      signal?.removeEventListener("abort", abort);
    };
    const done = () => {
      cleanup();
      resolve();
    };
    const fail = () => {
      cleanup();
      reject(new VideoGifError("unreadable"));
    };
    const abort = () => {
      cleanup();
      reject(abortError());
    };
    if (signal?.aborted) {
      abort();
      return;
    }
    video.addEventListener(event, done, { once: true });
    video.addEventListener("error", fail, { once: true });
    signal?.addEventListener("abort", abort, { once: true });
  });

export const loadVideo = async (url: string, signal?: AbortSignal): Promise<HTMLVideoElement> => {
  const video = document.createElement("video");
  video.muted = true;
  video.playsInline = true;
  video.preload = "auto";
  const loaded = waitFor(video, "loadeddata", signal);
  video.src = url;
  video.load();
  await loaded;
  if (!Number.isFinite(video.duration)) {
    const settled = waitFor(video, "seeked", signal);
    video.currentTime = Number.MAX_SAFE_INTEGER;
    await settled;
  }
  if (!video.videoWidth || !video.videoHeight || !Number.isFinite(video.duration)) {
    throw new VideoGifError("unreadable");
  }
  return video;
};

export const videoDurationMs = (video: HTMLVideoElement) => Math.floor(video.duration * 1000);

const seek = async (video: HTMLVideoElement, ms: number, signal?: AbortSignal) => {
  const target = Math.min(ms / 1000, Math.max(0, video.duration - 0.001));
  if (Math.abs(video.currentTime - target) < 0.0005) return;
  const settled = waitFor(video, "seeked", signal);
  video.currentTime = target;
  await settled;
};

const yieldToUi = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

export interface VideoGifOptions {
  crop: CropRect;
  startMs: number;
  lengthMs: number;
  signal?: AbortSignal;
  onProgress?: (fraction: number) => void;
}

const captureFrames = async (
  video: HTMLVideoElement,
  times: number[],
  crop: CropRect,
  signal: AbortSignal | undefined,
  onFrame: (index: number) => void,
) => {
  const canvas = document.createElement("canvas");
  canvas.width = GIF_SIZE;
  canvas.height = GIF_SIZE;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new VideoGifError("unreadable");
  ctx.imageSmoothingQuality = "high";
  const frames: Uint8ClampedArray[] = [];
  for (let i = 0; i < times.length; i += 1) {
    await seek(video, times[i], signal);
    ctx.drawImage(video, crop.x, crop.y, crop.width, crop.height, 0, 0, GIF_SIZE, GIF_SIZE);
    frames.push(ctx.getImageData(0, 0, GIF_SIZE, GIF_SIZE).data);
    onFrame(i);
  }
  return frames;
};

const encode = async (
  frames: Uint8ClampedArray[],
  step: number,
  delayMs: number,
  signal: AbortSignal | undefined,
  onFrame: (index: number) => void,
) => {
  const gif = GIFEncoder();
  for (let i = 0; i < frames.length; i += step) {
    if (signal?.aborted) throw abortError();
    const palette = quantize(frames[i], 256);
    gif.writeFrame(applyPalette(frames[i], palette), GIF_SIZE, GIF_SIZE, {
      palette,
      delay: delayMs,
    });
    onFrame(i);
    await yieldToUi();
  }
  gif.finish();
  return gif.bytes();
};

export const videoToGif = async (url: string, options: VideoGifOptions): Promise<File> => {
  const { crop, startMs, lengthMs, signal, onProgress } = options;
  const video = await loadVideo(url, signal);
  try {
    const times = frameTimes(startMs, lengthMs, GIF_FRAME_MS);
    const frames = await captureFrames(video, times, crop, signal, (i) =>
      onProgress?.(((i + 1) / times.length) * 0.5),
    );
    for (const step of GIF_FRAME_STEPS) {
      const bytes = await encode(frames, step, step * GIF_FRAME_MS, signal, (i) =>
        onProgress?.(0.5 + ((i + 1) / frames.length) * 0.5),
      );
      if (bytes.byteLength <= GIF_MAX_BYTES) {
        onProgress?.(1);
        return new File([bytes], "avatar.gif", { type: "image/gif" });
      }
    }
    throw new VideoGifError("tooLarge");
  } finally {
    video.removeAttribute("src");
    video.load();
  }
};
