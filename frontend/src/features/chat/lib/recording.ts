import type { VoiceRecording } from "./voiceRecorder";
import type { VideoNoteRecording } from "./videoNoteRecorder";

export type MediaRecording = VoiceRecording | VideoNoteRecording;

export type RecordKind = MediaRecording["kind"];
