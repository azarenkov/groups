import {
  ConnectionQuality,
  ConnectionState,
  DisconnectReason,
  Room,
  RoomEvent,
  Track,
} from "livekit-client";
import type {
  AudioTrack,
  LocalTrackPublication,
  RemoteParticipant,
  RemoteTrack,
  RemoteTrackPublication,
  VideoTrack,
} from "livekit-client";
import type { ParticipantInfo } from "./types";

export interface RoomEventHandlers {
  onConnected: () => void;
  onLocalTracksChanged: () => void;
  onParticipantConnected: (participant: RemoteParticipant) => void;
  onParticipantDisconnected: (identity: string) => void;
  onAudioTrack: (track: AudioTrack, participant: RemoteParticipant) => void;
  onVideoTrack: (track: VideoTrack, identity: string) => void;
  onParticipantPatch: (
    identity: string,
    patch: Partial<ParticipantInfo>,
  ) => void;
  onScreenSharesChanged: () => void;
  onDisconnected: (reason?: DisconnectReason) => void;
  onLocalQuality: (quality: ConnectionQuality) => void;
  onReconnectingStarted: () => void;
  onReconnectingSettled: () => void;
  onLocalAudioPublished: (publication: LocalTrackPublication) => void;
  onMediaDevicesError: (error: Error) => void;
  onLocalAttributes?: (attributes: Readonly<Record<string, string>>) => void;
  onLocalScreenMuted?: (trackSid: string) => void;
}

export function isScreenSource(source: Track.Source): boolean {
  return (
    source === Track.Source.ScreenShare ||
    source === Track.Source.ScreenShareAudio
  );
}

export function attachRoomListeners(
  room: Room,
  handlers: RoomEventHandlers,
): void {
  room.on(RoomEvent.Connected, () => {
    console.log("Channel connected successfully");
    handlers.onConnected();
  });

  room.on(RoomEvent.Reconnecting, () => {
    console.log("Channel reconnecting...");
  });

  room.on(RoomEvent.Reconnected, () => {
    console.log("Channel reconnected");
  });

  room.on(RoomEvent.LocalTrackPublished, () => {
    handlers.onLocalTracksChanged();
  });

  room.on(RoomEvent.LocalTrackUnpublished, () => {
    handlers.onLocalTracksChanged();
  });

  room.on(
    RoomEvent.ParticipantConnected,
    (participant: RemoteParticipant) => {
      console.log("Participant connected:", participant.identity);
      handlers.onParticipantConnected(participant);
      handlers.onScreenSharesChanged();
    },
  );

  room.on(
    RoomEvent.ParticipantDisconnected,
    (participant: RemoteParticipant) => {
      console.log("Participant disconnected:", participant.identity);
      handlers.onParticipantDisconnected(participant.identity);
      handlers.onScreenSharesChanged();
    },
  );

  room.on(RoomEvent.TrackPublished, (publication, participant) => {
    if (isScreenSource(publication.source)) handlers.onScreenSharesChanged();
    if (publication.source === Track.Source.Microphone) {
      handlers.onParticipantPatch(participant.identity, { isMuted: publication.isMuted });
    }
  });

  room.on(RoomEvent.TrackUnpublished, (publication, participant) => {
    if (isScreenSource(publication.source)) handlers.onScreenSharesChanged();
    if (publication.source === Track.Source.Microphone) {
      handlers.onParticipantPatch(participant.identity, { isMuted: true });
    }
  });

  room.on(RoomEvent.TrackMuted, (publication, participant) => {
    if (participant.isLocal && publication.source === Track.Source.ScreenShare) {
      handlers.onLocalScreenMuted?.(publication.trackSid);
      return;
    }
    if (participant.isLocal || publication.source !== Track.Source.Microphone) return;
    handlers.onParticipantPatch(participant.identity, { isMuted: true });
  });

  room.on(RoomEvent.TrackUnmuted, (publication, participant) => {
    if (participant.isLocal || publication.source !== Track.Source.Microphone) return;
    handlers.onParticipantPatch(participant.identity, { isMuted: false });
  });

  room.on(
    RoomEvent.TrackSubscribed,
    (
      track: RemoteTrack,
      _publication: RemoteTrackPublication,
      participant: RemoteParticipant,
    ) => {
      console.log("Track subscribed:", track.kind, participant.identity);
      if (isScreenSource(track.source)) handlers.onScreenSharesChanged();
      if (track.kind === Track.Kind.Audio) {
        handlers.onAudioTrack(track as AudioTrack, participant);
      } else if (track.kind === Track.Kind.Video) {
        handlers.onVideoTrack(track as VideoTrack, participant.identity);
      }
    },
  );

  room.on(
    RoomEvent.TrackUnsubscribed,
    (
      track: RemoteTrack,
      _publication: RemoteTrackPublication,
      participant: RemoteParticipant,
    ) => {
      console.log("Track unsubscribed:", track.kind, participant.identity);
      if (track.kind === Track.Kind.Audio) {
        if (track.source !== Track.Source.ScreenShareAudio) {
          handlers.onParticipantPatch(participant.identity, {
            audioTrack: null,
          });
        }
      } else if (track.kind === Track.Kind.Video) {
        if ((track as VideoTrack).source === Track.Source.ScreenShare) {
          handlers.onParticipantPatch(participant.identity, {
            screenTrack: null,
          });
        } else {
          handlers.onParticipantPatch(participant.identity, {
            cameraTrack: null,
          });
        }
      }
    },
  );

  room.on(RoomEvent.ActiveSpeakersChanged, (speakers) => {
    const speaking = new Set(speakers.map((p) => p.identity));
    for (const participant of room.remoteParticipants.values()) {
      handlers.onParticipantPatch(participant.identity, {
        isSpeaking: speaking.has(participant.identity),
      });
    }
  });

  room.on(RoomEvent.Disconnected, (reason) => {
    console.log("Disconnected from room", { reason });
    handlers.onDisconnected(reason);
  });

  room.on(RoomEvent.ConnectionQualityChanged, (quality, participant) => {
    const isLocal =
      !participant || participant.identity === room.localParticipant.identity;
    if (isLocal) {
      handlers.onLocalQuality(quality);
      return;
    }
    handlers.onParticipantPatch(participant.identity, { quality });
  });

  room.on(RoomEvent.ConnectionStateChanged, (state) => {
    console.log("Connection state changed:", state);
    if (
      state === ConnectionState.Reconnecting ||
      state === ConnectionState.SignalReconnecting
    ) {
      handlers.onReconnectingStarted();
    } else {
      handlers.onReconnectingSettled();
    }
  });

  room.on(RoomEvent.LocalTrackPublished, (publication) => {
    if (publication.kind !== Track.Kind.Audio) return;
    handlers.onLocalAudioPublished(publication);
  });

  room.on(RoomEvent.MediaDevicesError, (error, kind) => {
    const fromScreenShare = kind === undefined;
    const dismissed =
      fromScreenShare &&
      (error.name === "NotAllowedError" ||
        error.name === "AbortError" ||
        error.message.includes("Permission dismissed") ||
        error.message.includes("Could not start video source"));
    if (dismissed) {
      console.warn("[LiveKitService] media device request dismissed:", error);
      return;
    }
    console.error("Media devices error:", error);
    handlers.onMediaDevicesError(error);
  });

  room.on(
    RoomEvent.ParticipantPermissionsChanged,
    (prevPermissions, participant) => {
      console.log("Participant permissions changed:", {
        participant: participant.identity,
        prevPermissions,
      });
    },
  );

  room.on(RoomEvent.ParticipantAttributesChanged, (_changed, participant) => {
    if (participant.isLocal) {
      handlers.onLocalAttributes?.(participant.attributes ?? {});
      return;
    }
    handlers.onParticipantPatch(participant.identity, {
      isDeafened: participant.attributes?.deafened === "1",
    });
  });
}
