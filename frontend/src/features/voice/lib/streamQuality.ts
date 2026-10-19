export interface StreamQuality {
  id: "720p60" | "1080p60" | "1440p60" | "2160p60";
  label: string;
  width: number;
  height: number;
  frameRate: number;
  maxBitrate: number;
}

export const STREAM_QUALITIES: StreamQuality[] = [
  { id: "720p60", label: "720p 60", width: 1280, height: 720, frameRate: 60, maxBitrate: 4_000_000 },
  { id: "1080p60", label: "1080p 60", width: 1920, height: 1080, frameRate: 60, maxBitrate: 8_000_000 },
  { id: "1440p60", label: "1440p 60", width: 2560, height: 1440, frameRate: 60, maxBitrate: 12_000_000 },
  { id: "2160p60", label: "4K 60", width: 3840, height: 2160, frameRate: 60, maxBitrate: 20_000_000 },
];

export type StreamQualityChoice = "auto" | StreamQuality["id"];

export const MAX_STREAM_QUALITY: StreamQuality = STREAM_QUALITIES[STREAM_QUALITIES.length - 1];

export const streamQualityById = (id: string | null | undefined): StreamQuality =>
  STREAM_QUALITIES.find((q) => q.id === id) ?? MAX_STREAM_QUALITY;

export const resolveStreamQuality = (wanted: StreamQualityChoice): StreamQuality =>
  wanted === "auto" ? MAX_STREAM_QUALITY : streamQualityById(wanted);
