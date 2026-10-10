const POSTER_MAX_EDGE = 640;

export class PosterExtractionError extends Error {}
const POSTER_TIMEOUT_MS = 20_000;

export const extractVideoPoster = (video: Blob): Promise<Blob> =>
  new Promise((resolve, reject) => {
    const source = URL.createObjectURL(video);
    const el = document.createElement("video");
    let done = false;
    const finish = (result: Blob | Error) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      el.removeAttribute("src");
      el.load();
      URL.revokeObjectURL(source);
      if (result instanceof Blob) resolve(result);
      else reject(new PosterExtractionError(result.message));
    };
    const timer = setTimeout(() => finish(new Error("video poster timed out")), POSTER_TIMEOUT_MS);
    const capture = () => {
      const width = el.videoWidth;
      const height = el.videoHeight;
      if (!width || !height) {
        finish(new Error("video has no frames"));
        return;
      }
      const scale = Math.min(1, POSTER_MAX_EDGE / Math.max(width, height));
      const canvas = document.createElement("canvas");
      canvas.width = Math.round(width * scale);
      canvas.height = Math.round(height * scale);
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        finish(new Error("canvas unavailable"));
        return;
      }
      ctx.drawImage(el, 0, 0, canvas.width, canvas.height);
      canvas.toBlob(
        (blob) => finish(blob ?? new Error("video poster encoding failed")),
        "image/jpeg",
        0.82,
      );
    };
    el.muted = true;
    el.playsInline = true;
    el.preload = "auto";
    el.onloadeddata = () => {
      const duration = el.duration;
      if (Number.isFinite(duration) && duration > 0) el.currentTime = Math.min(0.1, duration / 2);
      else capture();
    };
    el.onseeked = capture;
    el.onerror = () => finish(new Error("video could not be decoded"));
    el.src = source;
  });
