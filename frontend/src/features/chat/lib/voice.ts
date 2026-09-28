import type { Attachment, ChannelMessage } from "../../../shared/types/channel";

export const VOICE_WAVEFORM_BARS = 64;

const VOICE_MIME_CANDIDATES = [
  "audio/webm;codecs=opus",
  "audio/ogg;codecs=opus",
  "audio/mp4;codecs=mp4a.40.2",
  "audio/mp4",
  "audio/webm",
];

export function pickVoiceMimeType(isTypeSupported: (type: string) => boolean): string | null {
  return VOICE_MIME_CANDIDATES.find((type) => isTypeSupported(type)) ?? null;
}

export function voiceBaseType(mime: string): string {
  return mime.split(";")[0].trim().toLowerCase();
}

export function voiceFileName(mime: string): string {
  const base = voiceBaseType(mime);
  const ext = base === "audio/mp4" ? "m4a" : base === "audio/ogg" ? "ogg" : "webm";
  return `voice.${ext}`;
}

export function formatVoiceDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const ss = String(seconds).padStart(2, "0");
  if (hours > 0) return `${hours}:${String(minutes).padStart(2, "0")}:${ss}`;
  return `${minutes}:${ss}`;
}

export function downsampleWaveform(levels: number[], bars: number = VOICE_WAVEFORM_BARS): number[] {
  if (levels.length === 0) return new Array(bars).fill(0);
  const peaks: number[] = [];
  for (let i = 0; i < bars; i++) {
    const start = Math.floor((i * levels.length) / bars);
    const end = Math.max(start + 1, Math.floor(((i + 1) * levels.length) / bars));
    let peak = 0;
    for (let j = start; j < end && j < levels.length; j++) peak = Math.max(peak, levels[j]);
    peaks.push(peak);
  }
  const max = Math.max(...peaks);
  if (max <= 0) return peaks.map(() => 0);
  return peaks.map((p) => Math.round(Math.min(1, p / max) * 255));
}

export function voiceAttachmentOf(msg: Pick<ChannelMessage, "attachments">): Attachment | null {
  return msg.attachments?.find((a) => !!a.voice) ?? null;
}

export function videoNoteAttachmentOf(msg: Pick<ChannelMessage, "attachments">): Attachment | null {
  return msg.attachments?.find((a) => !!a.videoNote) ?? null;
}

export function isVoiceMessage(msg: Pick<ChannelMessage, "attachments">): boolean {
  return voiceAttachmentOf(msg) !== null || videoNoteAttachmentOf(msg) !== null;
}

const VIDEO_NOTE_MIME_CANDIDATES = [
  "video/webm;codecs=vp9,opus",
  "video/webm;codecs=vp8,opus",
  "video/mp4;codecs=avc1.42E01E,mp4a.40.2",
  "video/mp4",
];

export function pickVideoNoteMimeType(isTypeSupported: (type: string) => boolean): string | null {
  return VIDEO_NOTE_MIME_CANDIDATES.find((type) => isTypeSupported(type)) ?? null;
}

export function videoNoteDurationFromPayload(attachments: unknown): number | null {
  if (!Array.isArray(attachments)) return null;
  for (const a of attachments) {
    const ms = (a as { videoNote?: { durationMs?: unknown } } | null)?.videoNote?.durationMs;
    if (typeof ms === "number") return ms;
  }
  return null;
}

export function voiceDurationFromPayload(attachments: unknown): number | null {
  if (!Array.isArray(attachments)) return null;
  for (const a of attachments) {
    const ms = (a as { voice?: { durationMs?: unknown } } | null)?.voice?.durationMs;
    if (typeof ms === "number") return ms;
  }
  return null;
}
