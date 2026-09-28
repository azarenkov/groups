import { pickVideoNoteMimeType } from "./voice";

export interface VideoNoteRecording {
  kind: "video";
  blob: Blob;
  mimeType: string;
  durationMs: number;
  size: number;
  poster: string;
}

const FRAME_MS = 1000 / 30;
const CHUNK_INTERVAL_MS = 250;
const POSTER_SIZE = 160;
const POSTER_MAX_BYTES = 16 * 1024;
const AUDIO_BITRATE = 48_000;

export const videoNoteRecordingSupported = (): boolean =>
  typeof window !== "undefined" &&
  typeof window.MediaRecorder !== "undefined" &&
  !!navigator.mediaDevices?.getUserMedia &&
  typeof HTMLCanvasElement !== "undefined" &&
  typeof HTMLCanvasElement.prototype.captureStream === "function" &&
  pickVideoNoteMimeType((t) => window.MediaRecorder.isTypeSupported(t)) !== null;

const blobToBase64 = (blob: Blob): Promise<string> =>
  new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const url = String(reader.result ?? "");
      resolve(url.slice(url.indexOf(",") + 1));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });

const canvasJpeg = (canvas: HTMLCanvasElement, quality: number): Promise<Blob | null> =>
  new Promise((resolve) => canvas.toBlob((b) => resolve(b), "image/jpeg", quality));

export class VideoNoteRecorder {
  readonly canvas: HTMLCanvasElement;
  readonly size: number;
  facingUser = true;
  private video: HTMLVideoElement;
  private camera: MediaStream | null = null;
  private mic: MediaStream | null = null;
  private recorder: MediaRecorder | null = null;
  private chunks: Blob[] = [];
  private frame = 0;
  private lastDraw = 0;
  private startedAt = 0;
  private elapsedBefore = 0;
  private paused = false;
  private posterPromise: Promise<string> | null = null;
  private cameraIds: string[] = [];
  private cameraIndex = 0;
  private released = false;
  mimeType = "";

  constructor(size: number) {
    this.size = size;
    this.canvas = document.createElement("canvas");
    this.canvas.width = size;
    this.canvas.height = size;
    this.video = document.createElement("video");
    this.video.muted = true;
    this.video.playsInline = true;
  }

  async start(micId?: string): Promise<void> {
    const audio: MediaTrackConstraints = {
      echoCancellation: true,
      autoGainControl: true,
      noiseSuppression: false,
    };
    if (micId) audio.deviceId = { ideal: micId };
    const stream = await navigator.mediaDevices.getUserMedia({
      audio,
      video: { facingMode: "user", width: { ideal: 720 }, height: { ideal: 720 }, frameRate: { ideal: 30 } },
    });
    this.camera = new MediaStream(stream.getVideoTracks());
    this.mic = new MediaStream(stream.getAudioTracks());
    try {
      await this.attachCamera(this.camera);
      const devices = await navigator.mediaDevices.enumerateDevices().catch(() => []);
      this.cameraIds = devices.filter((d) => d.kind === "videoinput" && d.deviceId).map((d) => d.deviceId);
      const current = this.camera.getVideoTracks()[0]?.getSettings().deviceId;
      this.cameraIndex = Math.max(0, current ? this.cameraIds.indexOf(current) : 0);
      this.setUp();
    } catch (err) {
      this.release();
      throw err;
    }
  }

  private async attachCamera(stream: MediaStream): Promise<void> {
    this.video.srcObject = stream;
    await this.video.play();
    const facing = stream.getVideoTracks()[0]?.getSettings().facingMode;
    this.facingUser = facing ? facing === "user" : this.cameraIndex === 0;
  }

  private setUp(): void {
    const mime = pickVideoNoteMimeType((t) => MediaRecorder.isTypeSupported(t));
    if (!mime) throw new Error("unsupported");
    this.drawLoop();
    const out = this.canvas.captureStream(30);
    this.mic?.getAudioTracks().forEach((t) => out.addTrack(t));
    const recorder = new MediaRecorder(out, {
      mimeType: mime,
      videoBitsPerSecond: this.size >= 640 ? 1_200_000 : 700_000,
      audioBitsPerSecond: AUDIO_BITRATE,
    });
    this.recorder = recorder;
    this.mimeType = recorder.mimeType || mime;
    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) this.chunks.push(e.data);
    };
    recorder.start(CHUNK_INTERVAL_MS);
    this.startedAt = performance.now();
    this.posterPromise = new Promise<void>((r) => setTimeout(r, 300)).then(() => this.makePoster());
  }

  private drawLoop = (): void => {
    const now = performance.now();
    if (now - this.lastDraw >= FRAME_MS - 2) {
      this.lastDraw = now;
      const ctx = this.canvas.getContext("2d");
      const w = this.video.videoWidth;
      const h = this.video.videoHeight;
      if (ctx && w > 0 && h > 0) {
        const side = Math.min(w, h);
        ctx.drawImage(this.video, (w - side) / 2, (h - side) / 2, side, side, 0, 0, this.size, this.size);
      }
    }
    this.frame = requestAnimationFrame(this.drawLoop);
  };

  private async makePoster(): Promise<string> {
    const small = document.createElement("canvas");
    small.width = POSTER_SIZE;
    small.height = POSTER_SIZE;
    small.getContext("2d")?.drawImage(this.canvas, 0, 0, POSTER_SIZE, POSTER_SIZE);
    for (const quality of [0.7, 0.5, 0.35, 0.2]) {
      const blob = await canvasJpeg(small, quality);
      if (blob && blob.size <= POSTER_MAX_BYTES) return blobToBase64(blob);
    }
    throw new Error("poster");
  }

  canSwitchCamera(): boolean {
    return this.cameraIds.length > 1;
  }

  private openCamera(index: number): Promise<MediaStream> {
    return navigator.mediaDevices.getUserMedia({
      video: { deviceId: { exact: this.cameraIds[index] }, width: { ideal: 720 }, height: { ideal: 720 } },
    });
  }

  async switchCamera(): Promise<void> {
    if (this.cameraIds.length < 2) return;
    const previous = this.cameraIndex;
    const next = (previous + 1) % this.cameraIds.length;
    this.camera?.getTracks().forEach((t) => t.stop());
    this.camera = null;
    const adopt = async (stream: MediaStream, index: number): Promise<boolean> => {
      if (this.released) {
        stream.getTracks().forEach((t) => t.stop());
        return false;
      }
      this.cameraIndex = index;
      this.camera = stream;
      await this.attachCamera(stream);
      return true;
    };
    try {
      await adopt(await this.openCamera(next), next);
    } catch (err) {
      if (this.released) return;
      const restored = await this.openCamera(previous).catch(() => null);
      if (restored) await adopt(restored, previous).catch(() => false);
      throw err;
    }
  }

  elapsedMs(): number {
    return this.elapsedBefore + (this.paused || !this.recorder ? 0 : performance.now() - this.startedAt);
  }

  isPaused(): boolean {
    return this.paused;
  }

  pause(): void {
    if (!this.recorder || this.paused || this.recorder.state !== "recording") return;
    this.recorder.pause();
    this.elapsedBefore += performance.now() - this.startedAt;
    this.paused = true;
  }

  resume(): void {
    if (!this.recorder || !this.paused || this.recorder.state !== "paused") return;
    this.recorder.resume();
    this.startedAt = performance.now();
    this.paused = false;
  }

  async snapshot(): Promise<Blob> {
    const recorder = this.recorder;
    if (recorder && recorder.state !== "inactive") {
      await new Promise<void>((resolve) => {
        const done = () => {
          recorder.removeEventListener("dataavailable", done);
          resolve();
        };
        recorder.addEventListener("dataavailable", done);
        recorder.requestData();
      });
    }
    return new Blob(this.chunks, { type: this.mimeType });
  }

  async stop(): Promise<VideoNoteRecording> {
    const durationMs = Math.round(this.elapsedMs());
    const recorder = this.recorder;
    try {
      if (recorder && recorder.state !== "inactive") {
        await new Promise<void>((resolve) => {
          recorder.addEventListener("stop", () => resolve(), { once: true });
          recorder.stop();
        });
      }
      const poster = await (this.posterPromise ?? this.makePoster()).catch(() => this.makePoster());
      return {
        kind: "video",
        blob: new Blob(this.chunks, { type: this.mimeType }),
        mimeType: this.mimeType,
        durationMs,
        size: this.size,
        poster,
      };
    } finally {
      this.release();
    }
  }

  cancel(): void {
    const recorder = this.recorder;
    if (recorder && recorder.state !== "inactive") {
      try {
        recorder.stop();
      } catch {}
    }
    this.chunks = [];
    this.release();
  }

  private release(): void {
    this.released = true;
    cancelAnimationFrame(this.frame);
    this.camera?.getTracks().forEach((t) => t.stop());
    this.mic?.getTracks().forEach((t) => t.stop());
    this.camera = null;
    this.mic = null;
    this.video.srcObject = null;
    this.recorder = null;
  }
}
