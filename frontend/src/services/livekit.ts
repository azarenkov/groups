import {
  ConnectionQuality,
  Room,
  RoomEvent,
  Track,
  RemoteParticipant,
  RemoteTrackPublication,
  RemoteTrack,
  AudioTrack,
  LocalAudioTrack,
  createLocalAudioTrack,
  RoomConnectOptions,
  TrackEvent,
} from "livekit-client";

import { buildDfnPipeline, DENOISER_SAMPLE_RATE, DfnPipeline } from "./dfn";

export type NoiseSuppressionMode = "off" | "standard" | "high";

export { ConnectionQuality };

export interface ParticipantInfo {
  identity: string;
  name: string;
  isSpeaking: boolean;
  audioTrack: AudioTrack | null;
  quality: ConnectionQuality;
}

export class LiveKitService {
  private room: Room | null = null;
  private participants: Map<string, ParticipantInfo> = new Map();
  private onParticipantsChange?: (participants: ParticipantInfo[]) => void;
  private onConnectionStateChange?: (connected: boolean) => void;
  private onError?: (error: Error) => void;
  private isConnecting: boolean = false;
  private dfnPipeline: DfnPipeline | null = null;
  private dfnContext: AudioContext | null = null;
  private dfnRawTrack: MediaStreamTrack | null = null;
  private noiseMode: NoiseSuppressionMode = "standard";
  private noiseStrength: number = 1.0;
  private noiseModeChain: Promise<void> = Promise.resolve();
  private localQuality: ConnectionQuality = ConnectionQuality.Unknown;
  private localQualityListeners: Set<(quality: ConnectionQuality) => void> =
    new Set();
  private localTrackListeners: Set<(track: MediaStreamTrack | null) => void> =
    new Set();

  constructor(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
  ) {
    this.onParticipantsChange = onParticipantsChange;
    this.onConnectionStateChange = onConnectionStateChange;
    this.onError = onError;
  }

  updateCallbacks(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
  ) {
    if (onParticipantsChange !== undefined)
      this.onParticipantsChange = onParticipantsChange;
    if (onConnectionStateChange !== undefined)
      this.onConnectionStateChange = onConnectionStateChange;
    if (onError !== undefined) this.onError = onError;
  }

  getLocalQuality(): ConnectionQuality {
    return this.localQuality;
  }

  subscribeLocalQuality(
    fn: (quality: ConnectionQuality) => void,
  ): () => void {
    this.localQualityListeners.add(fn);
    return () => {
      this.localQualityListeners.delete(fn);
    };
  }

  subscribeLocalTrack(
    fn: (track: MediaStreamTrack | null) => void,
  ): () => void {
    this.localTrackListeners.add(fn);
    return () => {
      this.localTrackListeners.delete(fn);
    };
  }

  private emitLocalTrack(): void {
    const track = this.getLocalAudioMediaStreamTrack();
    this.localTrackListeners.forEach((fn) => {
      try {
        fn(track);
      } catch (e) {
        console.warn("[LiveKitService] localTrack listener error:", e);
      }
    });
  }

  private emitLocalQuality(quality: ConnectionQuality): void {
    this.localQuality = quality;
    this.localQualityListeners.forEach((fn) => {
      try {
        fn(quality);
      } catch (e) {
        console.warn("[LiveKitService] localQuality listener error:", e);
      }
    });
  }

  async connect(token: string, url: string): Promise<void> {
    if (this.isConnecting) {
      console.log("[LiveKitService] Connection already in progress, skipping");
      return;
    }

    if (this.room?.state === "connected") {
      console.log("[LiveKitService] Already connected to a room, skipping");
      return;
    }

    try {
      this.isConnecting = true;
      console.log("Connecting to LiveKit...", { url });

      this.room = new Room({
        adaptiveStream: true,
        dynacast: true,
        audioCaptureDefaults: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      this.setupEventListeners();

      const connectOptions: RoomConnectOptions = {
        autoSubscribe: true,
      };

      console.log("Attempting room connection...");
      await this.room.connect(url, token, connectOptions);
      console.log("Successfully connected to room");

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
    if (this.dfnPipeline) {
      try {
        await this.dfnPipeline.dispose();
      } catch {}
      try {
        this.dfnPipeline.outputTrack.stop();
      } catch {}
      try {
        await this.dfnContext?.close();
      } catch {}
      try {
        this.dfnRawTrack?.stop();
      } catch {}
      this.dfnPipeline = null;
      this.dfnContext = null;
      this.dfnRawTrack = null;
    }
    if (this.room) {
      console.log("[LiveKitService] Disconnecting from room");
      await this.room.disconnect();
      this.room = null;
      this.participants.clear();
      this.isConnecting = false;
      this.emitLocalQuality(ConnectionQuality.Unknown);
      this.emitLocalTrack();
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
      this.emitLocalTrack();
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
    this.emitLocalTrack();
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

    if (this.dfnRawTrack) {
      this.dfnRawTrack.enabled = !muted;
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

  getLocalAudioMediaStreamTrack(): MediaStreamTrack | null {
    if (this.dfnPipeline) return this.dfnPipeline.outputTrack;
    if (!this.room) return null;
    const publications = Array.from(
      this.room.localParticipant.audioTrackPublications.values(),
    );
    for (const pub of publications) {
      const track = pub.track?.mediaStreamTrack;
      if (track) return track;
    }
    return null;
  }

  getLocalSessionId(): string | null {
    return this.room?.localParticipant.sid ?? null;
  }

  async setNoiseSuppression(enabled: boolean): Promise<void> {
    await this.setNoiseSuppressionMode(enabled ? "standard" : "off");
  }

  getNoiseSuppressionMode(): NoiseSuppressionMode {
    return this.noiseMode;
  }

  getNoiseStrength(): number {
    return this.noiseStrength;
  }

  setNoiseStrength(value: number): void {
    const clamped = Math.min(1, Math.max(0, value));
    this.noiseStrength = clamped;
    this.dfnPipeline?.setStrength(clamped);
  }

  async setNoiseSuppressionMode(mode: NoiseSuppressionMode): Promise<void> {
    return this.applyNoiseSuppressionMode(mode, false);
  }

  private async applyNoiseSuppressionMode(
    mode: NoiseSuppressionMode,
    force: boolean,
  ): Promise<void> {
    const run = async () => {
      if (
        !force &&
        mode === "high" &&
        this.noiseMode === "high" &&
        this.dfnPipeline
      ) {
        return;
      }
      if (force && mode === "high" && this.dfnPipeline) {
        await this.disableDfnPipeline();
      }
      this.noiseMode = mode;

      if (mode === "high") {
        await this.enableDfnPipeline();
        return;
      }

      await this.disableDfnPipeline();

      const track = this.getLocalAudioMediaStreamTrack();
      if (!track) return;
      try {
        await track.applyConstraints({ noiseSuppression: mode === "standard" });
      } catch (e) {
        console.warn("[LiveKitService] applyConstraints failed:", e);
      }
    };
    this.noiseModeChain = this.noiseModeChain.then(run, run);
    return this.noiseModeChain;
  }

  private async waitForLocalAudioTrack(
    timeoutMs: number,
  ): Promise<LocalAudioTrack | null> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      if (!this.room) return null;
      const publication = Array.from(
        this.room.localParticipant.audioTrackPublications.values(),
      )[0];
      const track = publication?.track;
      if (track instanceof LocalAudioTrack && track.mediaStreamTrack) {
        return track;
      }
      await new Promise((r) => setTimeout(r, 100));
    }
    return null;
  }

  private async enableDfnPipeline(): Promise<void> {
    if (!this.room) return;
    const localTrack = await this.waitForLocalAudioTrack(5000);
    if (!localTrack) {
      const err = new Error("Local audio track did not appear in time");
      console.warn("[LiveKitService] enableDfnPipeline:", err.message);
      this.noiseMode = "standard";
      this.onError?.(err);
      return;
    }
    const rawTrack = localTrack.mediaStreamTrack;
    if (!rawTrack) {
      const err = new Error("Raw mic track missing");
      console.warn("[LiveKitService] enableDfnPipeline:", err.message);
      this.noiseMode = "standard";
      this.onError?.(err);
      return;
    }

    let lastErr: unknown;
    for (let attempt = 1; attempt <= 3; attempt++) {
      let step = "init";
      let ctx: AudioContext | null = null;
      let pipeline: DfnPipeline | null = null;
      try {
        step = "applyConstraints";
        try {
          await rawTrack.applyConstraints({ noiseSuppression: false });
        } catch {}

        step = "AudioContext";
        const AudioCtx =
          window.AudioContext ||
          (window as unknown as { webkitAudioContext: typeof AudioContext })
            .webkitAudioContext;
        ctx = new AudioCtx({ sampleRate: DENOISER_SAMPLE_RATE });
        if (ctx.state === "suspended") {
          step = "ctx.resume";
          await ctx.resume();
        }
        step = "createMediaStreamSource";
        const source = ctx.createMediaStreamSource(new MediaStream([rawTrack]));
        step = "buildDfnPipeline";
        pipeline = await buildDfnPipeline({ source, context: ctx });
        step = "replaceTrack";
        await localTrack.replaceTrack(pipeline.outputTrack, {
          userProvidedTrack: false,
        });

        this.dfnPipeline = pipeline;
        this.dfnContext = ctx;
        this.dfnRawTrack = rawTrack;
        rawTrack.enabled = !this.isMicrophoneMuted();
        pipeline.setStrength(this.noiseStrength);
        this.emitLocalTrack();
        console.log("[LiveKitService] DFN pipeline enabled on attempt", attempt);
        return;
      } catch (e) {
        lastErr = e;
        console.warn(
          `[LiveKitService] enableDfnPipeline attempt ${attempt} failed at step "${step}":`,
          e,
        );
        if (pipeline) {
          try {
            await pipeline.dispose();
          } catch {}
          try {
            pipeline.outputTrack.stop();
          } catch {}
        }
        if (ctx) {
          try {
            await ctx.close();
          } catch {}
        }
        await new Promise((r) => setTimeout(r, 200));
      }
    }

    console.error("[LiveKitService] failed to enable DFN pipeline:", lastErr);
    this.noiseMode = "standard";
    try {
      await rawTrack.applyConstraints({ noiseSuppression: true });
    } catch {}
    this.onError?.(
      lastErr instanceof Error
        ? lastErr
        : new Error("Failed to enable noise suppression"),
    );
  }

  private async disableDfnPipeline(): Promise<void> {
    if (!this.dfnPipeline) return;
    const room = this.room;
    const rawTrack = this.dfnRawTrack;
    const pipeline = this.dfnPipeline;
    const ctx = this.dfnContext;

    if (room && rawTrack) {
      const publication = Array.from(
        room.localParticipant.audioTrackPublications.values(),
      )[0];
      const localTrack = publication?.track;
      if (localTrack instanceof LocalAudioTrack) {
        try {
          await localTrack.replaceTrack(rawTrack, {
            userProvidedTrack: false,
          });
        } catch (e) {
          console.warn("[LiveKitService] failed to restore raw mic track:", e);
        }
      }
    }

    try {
      await pipeline.dispose();
    } catch {}
    try {
      pipeline.outputTrack.stop();
    } catch {}
    try {
      await ctx?.close();
    } catch {}

    this.dfnPipeline = null;
    this.dfnContext = null;
    this.dfnRawTrack = null;
    this.emitLocalTrack();
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

    this.room.on(RoomEvent.Connected, () => {
      console.log("Channel connected successfully");
      this.onConnectionStateChange?.(true);
    });

    this.room.on(RoomEvent.Reconnecting, () => {
      console.log("Channel reconnecting...");
    });

    this.room.on(RoomEvent.Reconnected, () => {
      console.log("Channel reconnected");
    });

    this.room.on(
      RoomEvent.ParticipantConnected,
      (participant: RemoteParticipant) => {
        console.log("Participant connected:", participant.identity);
        this.addParticipant(participant);
      },
    );

    this.room.on(
      RoomEvent.ParticipantDisconnected,
      (participant: RemoteParticipant) => {
        console.log("Participant disconnected:", participant.identity);
        this.removeParticipant(participant.identity);
      },
    );

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

    this.room.on(RoomEvent.Disconnected, (reason) => {
      console.log("Disconnected from room", { reason });
      this.participants.clear();
      this.emitLocalQuality(ConnectionQuality.Unknown);
      this.notifyParticipantsChange();
      this.onConnectionStateChange?.(false);
    });

    this.room.on(RoomEvent.ConnectionQualityChanged, (quality, participant) => {
      const isLocal =
        !participant ||
        participant.identity === this.room?.localParticipant.identity;
      if (isLocal) {
        this.emitLocalQuality(quality);
        return;
      }
      this.updateParticipant(participant.identity, { quality });
    });

    this.room.on(RoomEvent.ConnectionStateChanged, (state) => {
      console.log("Connection state changed:", state);
    });

    this.room.on(RoomEvent.LocalTrackPublished, (publication) => {
      if (publication.kind !== Track.Kind.Audio) return;
      this.applyNoiseSuppressionMode(this.noiseMode, true).catch((e) => {
        console.warn(
          "[LiveKitService] reapply noise mode after publish failed:",
          e,
        );
      });
    });

    this.room.on(RoomEvent.MediaDevicesError, (error) => {
      console.error("Media devices error:", error);
      this.onError?.(error);
    });

    this.room.on(
      RoomEvent.ParticipantPermissionsChanged,
      (prevPermissions, participant) => {
        console.log("Participant permissions changed:", {
          participant: participant.identity,
          prevPermissions,
        });
      },
    );

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
      quality: participant.connectionQuality ?? ConnectionQuality.Unknown,
    };

    this.participants.set(participant.identity, info);

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

    const audioElement = track.attach();
    audioElement.volume = 1.0;
    document.body.appendChild(audioElement);

    track.on(TrackEvent.Ended, () => {
      audioElement.remove();
      this.updateParticipant(participant.identity, { audioTrack: null });
    });
  }

  private notifyParticipantsChange(): void {
    this.onParticipantsChange?.(this.getParticipants());
  }
}

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
