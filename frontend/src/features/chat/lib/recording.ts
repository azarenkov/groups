import type { VoiceRecording } from "./voiceRecorder";
import type { VideoNoteRecording } from "./videoNoteRecorder";

export type MediaRecording = VoiceRecording | VideoNoteRecording;

export type RecordKind = MediaRecording["kind"];

const KIND_KEY = "rift.recordKind";

export const readRecordKind = (): RecordKind => {
  try {
    return localStorage.getItem(KIND_KEY) === "video" ? "video" : "voice";
  } catch {
    return "voice";
  }
};

export const saveRecordKind = (kind: RecordKind): void => {
  try {
    localStorage.setItem(KIND_KEY, kind);
  } catch {}
};
