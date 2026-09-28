import { downsampleWaveform, pickVoiceMimeType } from "./voice";

export interface VoiceRecording {
  kind: "voice";
  blob: Blob;
  mimeType: string;
  durationMs: number;
  waveform: number[];
}

const SAMPLE_INTERVAL_MS = 50;
const CHUNK_INTERVAL_MS = 250;
const VOICE_BITRATE = 32_000;

export const voiceRecordingSupported = (): boolean =>
  typeof window !== "undefined" &&
  typeof window.MediaRecorder !== "undefined" &&
  !!navigator.mediaDevices?.getUserMedia &&
  pickVoiceMimeType((t) => window.MediaRecorder.isTypeSupported(t)) !== null;

export class VoiceRecorder {
  private stream: MediaStream | null = null;
  private recorder: MediaRecorder | null = null;
  private audioContext: AudioContext | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array | null = null;
  private chunks: Blob[] = [];
  private levels: number[] = [];
  private sampleTimer: ReturnType<typeof setInterval> | null = null;
  private startedAt = 0;
  private elapsedBefore = 0;
  private paused = false;
  mimeType = "";
  onLevel?: (level: number) => void;

  async start(deviceId?: string): Promise<void> {
    const audio: MediaTrackConstraints = {
      echoCancellation: true,
      autoGainControl: true,
      noiseSuppression: false,
    };
    if (deviceId) audio.deviceId = { ideal: deviceId };
    const stream = await navigator.mediaDevices.getUserMedia({ audio });
    this.stream = stream;
    try {
      this.setUp(stream);
    } catch (err) {
      this.release();
      throw err;
    }
  }

  private setUp(stream: MediaStream): void {
    const mime = pickVoiceMimeType((t) => MediaRecorder.isTypeSupported(t));
    if (!mime) throw new Error("unsupported");
    const recorder = new MediaRecorder(stream, { mimeType: mime, audioBitsPerSecond: VOICE_BITRATE });
    this.recorder = recorder;
    this.mimeType = recorder.mimeType || mime;
    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) this.chunks.push(e.data);
    };
    try {
      const context = new AudioContext();
      const source = context.createMediaStreamSource(stream);
      const analyser = context.createAnalyser();
      analyser.fftSize = 1024;
      source.connect(analyser);
      this.audioContext = context;
      this.analyser = analyser;
      this.samples = new Float32Array(analyser.fftSize);
    } catch {
      this.audioContext = null;
      this.analyser = null;
    }
    recorder.start(CHUNK_INTERVAL_MS);
    this.startedAt = performance.now();
    this.sampleTimer = setInterval(() => this.sample(), SAMPLE_INTERVAL_MS);
  }

  private sample(): void {
    if (this.paused) return;
    let level = 0;
    if (this.analyser && this.samples) {
      this.analyser.getFloatTimeDomainData(this.samples);
      let sum = 0;
      for (let i = 0; i < this.samples.length; i++) sum += this.samples[i] * this.samples[i];
      level = Math.min(1, Math.sqrt(sum / this.samples.length) * 4);
    }
    this.levels.push(level);
    this.onLevel?.(level);
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

  async stop(): Promise<VoiceRecording> {
    const durationMs = Math.round(this.elapsedMs());
    const recorder = this.recorder;
    if (recorder && recorder.state !== "inactive") {
      await new Promise<void>((resolve) => {
        recorder.addEventListener("stop", () => resolve(), { once: true });
        recorder.stop();
      });
    }
    const recording: VoiceRecording = {
      kind: "voice",
      blob: new Blob(this.chunks, { type: this.mimeType }),
      mimeType: this.mimeType,
      durationMs,
      waveform: downsampleWaveform(this.levels),
    };
    this.release();
    return recording;
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
    if (this.sampleTimer) clearInterval(this.sampleTimer);
    this.sampleTimer = null;
    this.stream?.getTracks().forEach((t) => t.stop());
    this.stream = null;
    void this.audioContext?.close().catch(() => {});
    this.audioContext = null;
    this.analyser = null;
    this.recorder = null;
    this.onLevel = undefined;
  }
}
