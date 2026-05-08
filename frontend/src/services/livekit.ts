import {
  Room,
  RoomEvent,
  Track,
  RemoteParticipant,
  RemoteTrackPublication,
  RemoteTrack,
  AudioTrack,
  createLocalAudioTrack,
  RoomConnectOptions,
  TrackEvent,
} from "livekit-client";

export interface ParticipantInfo {
  identity: string;
  name: string;
  isSpeaking: boolean;
  audioTrack: AudioTrack | null;
}

export class LiveKitService {
  private room: Room | null = null;
  private participants: Map<string, ParticipantInfo> = new Map();
  private onParticipantsChange?: (participants: ParticipantInfo[]) => void;
  private onConnectionStateChange?: (connected: boolean) => void;
  private onError?: (error: Error) => void;
  private isConnecting: boolean = false;

  constructor(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
  ) {
    this.onParticipantsChange = onParticipantsChange;
    this.onConnectionStateChange = onConnectionStateChange;
    this.onError = onError;
  }

  // Update callbacks dynamically (for singleton pattern)
  updateCallbacks(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
  ) {
    this.onParticipantsChange = onParticipantsChange;
    this.onConnectionStateChange = onConnectionStateChange;
    this.onError = onError;
  }

  async connect(token: string, url: string): Promise<void> {
    // Prevent concurrent connection attempts
    if (this.isConnecting) {
      console.log("[LiveKitService] Connection already in progress, skipping");
      return;
    }

    // Check if already connected to this room
    if (this.room?.state === "connected") {
      console.log("[LiveKitService] Already connected to a room, skipping");
      return;
    }

    try {
      this.isConnecting = true;
      console.log("Connecting to LiveKit...", { url });

      // Create new room instance
      this.room = new Room({
        adaptiveStream: true,
        dynacast: true,
        audioCaptureDefaults: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      // Set up event listeners
      this.setupEventListeners();

      // Connect to room
      const connectOptions: RoomConnectOptions = {
        autoSubscribe: true,
      };

      console.log("Attempting room connection...");
      await this.room.connect(url, token, connectOptions);
      console.log("Successfully connected to room");

      // Enable local audio
      await this.enableMicrophone();

      this.isConnecting = false;
      this.onConnectionStateChange?.(true);
    } catch (error) {
      console.error("Failed to connect to LiveKit room:", error);
      console.error("LiveKit URL was:", url);
      console.error("Error details:", {
        name: error instanceof Error ? error.name : "Unknown",
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });

      this.isConnecting = false;
      this.onError?.(
        error instanceof Error ? error : new Error("Failed to connect"),
      );
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    if (this.room) {
      console.log("[LiveKitService] Disconnecting from room");
      await this.room.disconnect();
      this.room = null;
      this.participants.clear();
      this.isConnecting = false;
      this.notifyParticipantsChange();
      this.onConnectionStateChange?.(false);
    }
  }

  async enableMicrophone(): Promise<void> {
    if (!this.room) return;

    try {
      const track = await createLocalAudioTrack({
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
      });
      await this.room.localParticipant.publishTrack(track);
    } catch (error) {
      console.error("Failed to enable microphone:", error);
      this.onError?.(
        error instanceof Error
          ? error
          : new Error("Failed to enable microphone"),
      );
    }
  }

  async disableMicrophone(): Promise<void> {
    if (!this.room) return;

    const audioTracks = Array.from(
      this.room.localParticipant.audioTrackPublications.values(),
    );

    for (const publication of audioTracks) {
      if (publication.track) {
        await this.room.localParticipant.unpublishTrack(publication.track);
        publication.track.stop();
      }
    }
  }

  setMicrophoneMuted(muted: boolean): void {
    if (!this.room) return;

    const audioTracks = Array.from(
      this.room.localParticipant.audioTrackPublications.values(),
    );

    for (const publication of audioTracks) {
      if (publication.track) {
        if (muted) {
          publication.track.mute();
        } else {
          publication.track.unmute();
        }
      }
    }
  }

  isMicrophoneMuted(): boolean {
    if (!this.room) return true;

    const audioTracks = Array.from(
      this.room.localParticipant.audioTrackPublications.values(),
    );

    if (audioTracks.length === 0) return true;

    return audioTracks.some((pub) => pub.track?.isMuted);
  }

  getParticipants(): ParticipantInfo[] {
    return Array.from(this.participants.values());
  }

  isConnected(): boolean {
    return this.room?.state === "connected";
  }

  isConnectingNow(): boolean {
    return this.isConnecting;
  }

  private setupEventListeners(): void {
    if (!this.room) return;

    // Connection state events
    this.room.on(RoomEvent.Connected, () => {
      console.log("Room connected successfully");
      this.onConnectionStateChange?.(true);
    });

    this.room.on(RoomEvent.Reconnecting, () => {
      console.log("Room reconnecting...");
    });

    this.room.on(RoomEvent.Reconnected, () => {
      console.log("Room reconnected");
    });

    // Participant connected
    this.room.on(
      RoomEvent.ParticipantConnected,
      (participant: RemoteParticipant) => {
        console.log("Participant connected:", participant.identity);
        this.addParticipant(participant);
      },
    );

    // Participant disconnected
    this.room.on(
      RoomEvent.ParticipantDisconnected,
      (participant: RemoteParticipant) => {
        console.log("Participant disconnected:", participant.identity);
        this.removeParticipant(participant.identity);
      },
    );

    // Track subscribed
    this.room.on(
      RoomEvent.TrackSubscribed,
      (
        track: RemoteTrack,
        _publication: RemoteTrackPublication,
        participant: RemoteParticipant,
      ) => {
        console.log("Track subscribed:", track.kind, participant.identity);
        if (track.kind === Track.Kind.Audio) {
          this.handleAudioTrack(track as AudioTrack, participant);
        }
      },
    );

    // Track unsubscribed
    this.room.on(
      RoomEvent.TrackUnsubscribed,
      (
        track: RemoteTrack,
        _publication: RemoteTrackPublication,
        participant: RemoteParticipant,
      ) => {
        console.log("Track unsubscribed:", track.kind, participant.identity);
        if (track.kind === Track.Kind.Audio) {
          this.updateParticipant(participant.identity, { audioTrack: null });
        }
      },
    );

    // Connection state changed
    this.room.on(RoomEvent.Disconnected, (reason) => {
      console.log("Disconnected from room", { reason });
      this.participants.clear();
      this.notifyParticipantsChange();
      this.onConnectionStateChange?.(false);
    });

    // Connection quality
    this.room.on(RoomEvent.ConnectionQualityChanged, (quality, participant) => {
      console.log("Connection quality changed:", {
        quality,
        participant: participant?.identity || "local",
      });
    });

    // Connection state
    this.room.on(RoomEvent.ConnectionStateChanged, (state) => {
      console.log("Connection state changed:", state);
    });

    // Media devices error
    this.room.on(RoomEvent.MediaDevicesError, (error) => {
      console.error("Media devices error:", error);
      this.onError?.(error);
    });

    // Participant permission changed
    this.room.on(
      RoomEvent.ParticipantPermissionsChanged,
      (prevPermissions, participant) => {
        console.log("Participant permissions changed:", {
          participant: participant.identity,
          prevPermissions,
        });
      },
    );

    // Add existing participants
    this.room.remoteParticipants.forEach((participant) => {
      this.addParticipant(participant);
    });
  }

  private addParticipant(participant: RemoteParticipant): void {
    const info: ParticipantInfo = {
      identity: participant.identity,
      name: participant.name || participant.identity,
      isSpeaking: false,
      audioTrack: null,
    };

    this.participants.set(participant.identity, info);

    // Check for existing audio tracks
    participant.audioTrackPublications.forEach((publication) => {
      if (publication.track) {
        this.handleAudioTrack(publication.track as AudioTrack, participant);
      }
    });

    this.notifyParticipantsChange();
  }

  private removeParticipant(identity: string): void {
    this.participants.delete(identity);
    this.notifyParticipantsChange();
  }

  private updateParticipant(
    identity: string,
    updates: Partial<ParticipantInfo>,
  ): void {
    const participant = this.participants.get(identity);
    if (participant) {
      Object.assign(participant, updates);
      this.notifyParticipantsChange();
    }
  }

  private handleAudioTrack(
    track: AudioTrack,
    participant: RemoteParticipant,
  ): void {
    this.updateParticipant(participant.identity, { audioTrack: track });

    // Attach audio element for playback
    const audioElement = track.attach();
    audioElement.volume = 1.0;
    document.body.appendChild(audioElement);

    // Clean up when track ends
    track.on(TrackEvent.Ended, () => {
      audioElement.remove();
      this.updateParticipant(participant.identity, { audioTrack: null });
    });
  }

  private notifyParticipantsChange(): void {
    this.onParticipantsChange?.(this.getParticipants());
  }
}

// Singleton instance
let livekitService: LiveKitService | null = null;

export function getLiveKitService(
  onParticipantsChange?: (participants: ParticipantInfo[]) => void,
  onConnectionStateChange?: (connected: boolean) => void,
  onError?: (error: Error) => void,
): LiveKitService {
  if (!livekitService) {
    livekitService = new LiveKitService(
      onParticipantsChange,
      onConnectionStateChange,
      onError,
    );
  } else {
    // Update callbacks for existing singleton
    livekitService.updateCallbacks(
      onParticipantsChange,
      onConnectionStateChange,
      onError,
    );
  }
  return livekitService;
}

export function resetLiveKitService(): void {
  if (livekitService) {
    livekitService.disconnect();
    livekitService = null;
  }
}
