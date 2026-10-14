import { create } from "zustand";

export const STREAM_CEILING_ATTRIBUTE = "rift.streamCeiling";

export interface StreamCeiling {
  width: number;
  height: number;
  frameRate: number;
  maxBitrate: number;
}

const positive = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;

export const parseStreamCeiling = (value: unknown): StreamCeiling | null => {
  let raw: unknown = value;
  if (typeof value === "string") {
    try {
      raw = JSON.parse(value);
    } catch {
      return null;
    }
  }
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  const width = positive(r.width);
  const height = positive(r.height);
  const frameRate = positive(r.frameRate);
  const maxBitrate = positive(r.maxBitrate);
  if (width === null || height === null || frameRate === null || maxBitrate === null) return null;
  return { width, height, frameRate, maxBitrate };
};

interface StreamCeilingState {
  ceiling: StreamCeiling | null;
  setCeiling: (ceiling: StreamCeiling | null) => void;
}

export const useStreamCeilingStore = create<StreamCeilingState>((set) => ({
  ceiling: null,
  setCeiling: (ceiling) => set({ ceiling }),
}));
