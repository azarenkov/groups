import type { AudioTrack, VideoTrack } from "livekit-client";
import { ConnectionQuality } from "livekit-client";

export type NoiseSuppressionMode = "off" | "standard" | "high";

export interface ParticipantInfo {
  identity: string;
  name: string;
  isSpeaking: boolean;
  isMuted: boolean;
  isDeafened: boolean;
  isSharingScreen: boolean;
  audioTrack: AudioTrack | null;
  cameraTrack: VideoTrack | null;
  screenTrack: VideoTrack | null;
  quality: ConnectionQuality;
}

export interface ScreenShareInfo {
  identity: string;
  name: string;
  isLocal: boolean;
}

export interface ScreenShareState {
  shares: ScreenShareInfo[];
  watching: string | null;
  watched: string[];
}

export interface VideoTile {
  id: string;
  identity: string;
  name: string;
  isLocal: boolean;
  source: "camera" | "screen";
  track: VideoTrack;
}
