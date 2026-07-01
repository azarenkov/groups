import {
  ConnectionQuality,
  ConnectionState,
  DisconnectReason,
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
  VideoTrack,
} from "livekit-client";

import type { DfnPipeline } from "./dfn";

export type NoiseSuppressionMode = "off" | "standard" | "high";

export { ConnectionQuality, DisconnectReason };

export interface ParticipantInfo {
  identity: string;
  name: string;
  isSpeaking: boolean;
  isMuted: boolean;
  isSharingScreen: boolean;
  audioTrack: AudioTrack | null;
  cameraTrack: VideoTrack | null;
  screenTrack: VideoTrack | null;
  quality: ConnectionQuality;
}

export interface VideoTile {
  id: string;
  identity: string;
  name: string;
  isLocal: boolean;
  source: "camera" | "screen";
  track: VideoTrack;
}

export class LiveKitService {
  private room: Room | null = null;
  private participants: Map<string, ParticipantInfo> = new Map();
  private onParticipantsChange?: (participants: ParticipantInfo[]) => void;
  private onConnectionStateChange?: (connected: boolean) => void;
  private onError?: (error: Error) => void;
  private onUnexpectedDisconnect?: (reason: DisconnectReason) => void;
  private isConnecting: boolean = false;
  private dfnPipeline: DfnPipeline | null = null;
  private dfnContext: AudioContext | null = null;
  private dfnRawTrack: MediaStreamTrack | null = null;
  private noiseMode: NoiseSuppressionMode = "standard";
  private desiredMicMuted: boolean = false;
  private remoteVolumes: Map<string, number> = new Map();
  private remoteAudioElements: Map<string, HTMLMediaElement[]> = new Map();
  private remoteAudioCtx: AudioContext | null = null;
  private remoteGainNodes: Map<string, GainNode[]> = new Map();
  private remoteSourceNodes: WeakMap<
    HTMLMediaElement,
    { source: MediaElementAudioSourceNode; gain: GainNode; identity: string }
  > = new WeakMap();
  private static readonly REMOTE_GAIN_MAX = 6.0;
  private noiseStrength: number = 1.0;
  private noiseModeChain: Promise<void> = Promise.resolve();
  private localQuality: ConnectionQuality = ConnectionQuality.Unknown;
  private localQualityListeners: Set<(quality: ConnectionQuality) => void> =
    new Set();
  private localTrackListeners: Set<(track: MediaStreamTrack | null) => void> =
    new Set();
  private localSpeaking: boolean = false;
  private localSpeakingListeners: Set<(speaking: boolean) => void> = new Set();
  private participantsListeners: Set<
    (participants: ParticipantInfo[]) => void
  > = new Set();
  private reconnectAbortTimer: ReturnType<typeof setTimeout> | null = null;
  private readonly RECONNECT_GIVE_UP_MS = 15000;

  constructor(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
    onUnexpectedDisconnect?: (reason: DisconnectReason) => void,
  ) {
    this.onParticipantsChange = onParticipantsChange;
    this.onConnectionStateChange = onConnectionStateChange;
    this.onError = onError;
    this.onUnexpectedDisconnect = onUnexpectedDisconnect;
  }

  updateCallbacks(
    onParticipantsChange?: (participants: ParticipantInfo[]) => void,
    onConnectionStateChange?: (connected: boolean) => void,
    onError?: (error: Error) => void,
    onUnexpectedDisconnect?: (reason: DisconnectReason) => void,
  ) {
    if (onParticipantsChange !== undefined)
      this.onParticipantsChange = onParticipantsChange;
    if (onConnectionStateChange !== undefined)
      this.onConnectionStateChange = onConnectionStateChange;
    if (onError !== undefined) this.onError = onError;
    if (onUnexpectedDisconnect !== undefined)
      this.onUnexpectedDisconnect = onUnexpectedDisconnect;
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

  getLocalSpeaking(): boolean {
    return this.localSpeaking;
  }

  subscribeLocalSpeaking(fn: (speaking: boolean) => void): () => void {
    this.localSpeakingListeners.add(fn);
    return () => {
      this.localSpeakingListeners.delete(fn);
    };
  }

  setLocalSpeaking(speaking: boolean): void {
    if (speaking === this.localSpeaking) return;
    this.localSpeaking = speaking;
    this.localSpeakingListeners.forEach((fn) => {
      try {
        fn(speaking);
      } catch (e) {
        console.warn("[LiveKitService] localSpeaking listener error:", e);
      }
    });
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

      this.room.remoteParticipants.forEach((participant) => {
        this.addParticipant(participant);
      });

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

  private async tearDownDfn(): Promise<void> {
    if (!this.dfnPipeline) return;
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

  async disconnect(): Promise<void> {
    this.clearReconnectAbortTimer();
    await this.tearDownDfn();
    if (this.room) {
      console.log("[LiveKitService] Disconnecting from room");
      await this.room.disconnect();
      this.room = null;
      this.participants.clear();
      this.isConnecting = false;
      this.desiredMicMuted = false;
      this.remoteVolumes.clear();
      this.remoteAudioElements.clear();
      this.remoteGainNodes.clear();
      this.remoteSourceNodes = new WeakMap();
      this.emitLocalQuality(ConnectionQuality.Unknown);
      this.emitLocalTrack();
      this.setLocalSpeaking(false);
      this.notifyParticipantsChange();
      this.onConnectionStateChange?.(false);
    }
  }

  private startReconnectAbortTimer(): void {
    if (this.reconnectAbortTimer) return;
    this.reconnectAbortTimer = setTimeout(() => {
      console.warn(
        `[LiveKitService] Reconnect did not recover within ${this.RECONNECT_GIVE_UP_MS}ms — aborting`,
      );
      this.reconnectAbortTimer = null;
      const cb = this.onUnexpectedDisconnect;
      void this.disconnect().finally(() => {
        cb?.(DisconnectReason.UNKNOWN_REASON);
      });
    }, this.RECONNECT_GIVE_UP_MS);
  }

  private clearReconnectAbortTimer(): void {
    if (this.reconnectAbortTimer) {
      clearTimeout(this.reconnectAbortTimer);
      this.reconnectAbortTimer = null;
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
    this.desiredMicMuted = muted;
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
    if (this.dfnPipeline) {
      await this.disableDfnPipeline();
      if (this.dfnPipeline) {
        console.warn(
          "[LiveKitService] enableDfnPipeline: existing pipeline could not be torn down, aborting",
        );
        return;
      }
    }
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

    let dfnModule: typeof import("./dfn");
    try {
      dfnModule = await import("./dfn");
    } catch (e) {
      console.error("[LiveKitService] failed to load DFN module:", e);
      this.noiseMode = "standard";
      this.onError?.(
        e instanceof Error ? e : new Error("Failed to load noise suppression"),
      );
      return;
    }
    const { buildDfnPipeline, DENOISER_SAMPLE_RATE } = dfnModule;

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
        if (!pipeline.hasDenoiser) {
          console.warn(
            "[LiveKitService] DFN models unavailable; falling back to standard",
          );
          try {
            await pipeline.dispose();
          } catch {}
          try {
            pipeline.outputTrack.stop();
          } catch {}
          try {
            await ctx.close();
          } catch {}
          this.noiseMode = "standard";
          try {
            await rawTrack.applyConstraints({ noiseSuppression: true });
          } catch {}
          this.onError?.(new Error("Noise suppression models unavailable"));
          return;
        }
        step = "replaceTrack";
        await localTrack.replaceTrack(pipeline.outputTrack);

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
          await localTrack.replaceTrack(rawTrack);
        } catch (e) {
          console.warn(
            "[LiveKitService] failed to restore raw mic track, keeping DFN pipeline alive:",
            e,
          );
          return;
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

  getVideoTiles(): VideoTile[] {
    const tiles: VideoTile[] = [];

    const local = this.room?.localParticipant;
    if (local) {
      const localName = local.name || local.identity;
      local.videoTrackPublications.forEach((pub) => {
        const track = pub.track as VideoTrack | undefined;
        if (!track || pub.isMuted) return;
        const source =
          track.source === Track.Source.ScreenShare ? "screen" : "camera";
        tiles.push({
          id: `${local.identity}:${source}`,
          identity: local.identity,
          name: localName,
          isLocal: true,
          source,
          track,
        });
      });
    }

    this.participants.forEach((info) => {
      if (info.cameraTrack) {
        tiles.push({
          id: `${info.identity}:camera`,
          identity: info.identity,
          name: info.name,
          isLocal: false,
          source: "camera",
          track: info.cameraTrack,
        });
      }
      if (info.screenTrack) {
        tiles.push({
          id: `${info.identity}:screen`,
          identity: info.identity,
          name: info.name,
          isLocal: false,
          source: "screen",
          track: info.screenTrack,
        });
      }
    });

    return tiles;
  }

  async setCameraEnabled(enabled: boolean): Promise<void> {
    if (!this.room) return;
    const local = this.room.localParticipant;
    if (enabled) {
      await local.setCameraEnabled(true);
    } else {
      const publication = local.getTrackPublication(Track.Source.Camera);
      if (publication?.track) {
        await local.unpublishTrack(publication.track, true);
      } else {
        await local.setCameraEnabled(false);
      }
    }
    this.notifyParticipantsChange();
  }

  async setScreenShareEnabled(enabled: boolean): Promise<void> {
    if (!this.room) return;
    if (enabled) {
      await this.room.localParticipant.setScreenShareEnabled(
        true,
        {
          resolution: { width: 3840, height: 2160, frameRate: 30 },
          contentHint: "detail",
          audio: true,
        },
        {
          simulcast: false,
          videoCodec: "vp9",
          screenShareEncoding: {
            maxBitrate: 12_000_000,
            maxFramerate: 30,
          },
        },
      );
    } else {
      await this.room.localParticipant.setScreenShareEnabled(false);
    }
    this.notifyParticipantsChange();
  }

  isCameraEnabled(): boolean {
    return this.room?.localParticipant.isCameraEnabled ?? false;
  }

  isScreenShareEnabled(): boolean {
    return this.room?.localParticipant.isScreenShareEnabled ?? false;
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

    this.room.on(RoomEvent.LocalTrackPublished, () => {
      this.notifyParticipantsChange();
    });

    this.room.on(RoomEvent.LocalTrackUnpublished, () => {
      this.notifyParticipantsChange();
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
        } else if (track.kind === Track.Kind.Video) {
          this.handleVideoTrack(track as VideoTrack, participant.identity);
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
        } else if (track.kind === Track.Kind.Video) {
          if ((track as VideoTrack).source === Track.Source.ScreenShare) {
            this.updateParticipant(participant.identity, { screenTrack: null });
          } else {
            this.updateParticipant(participant.identity, { cameraTrack: null });
          }
        }
      },
    );

    this.room.on(RoomEvent.Disconnected, (reason) => {
      console.log("Disconnected from room", { reason });
      this.clearReconnectAbortTimer();
      void this.tearDownDfn().then(() => this.emitLocalTrack());
      this.participants.clear();
      this.emitLocalQuality(ConnectionQuality.Unknown);
      this.setLocalSpeaking(false);
      this.notifyParticipantsChange();
      this.onConnectionStateChange?.(false);
      if (reason !== undefined && reason !== DisconnectReason.CLIENT_INITIATED) {
        this.onUnexpectedDisconnect?.(reason);
      }
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
      if (
        state === ConnectionState.Reconnecting ||
        state === ConnectionState.SignalReconnecting
      ) {
        this.startReconnectAbortTimer();
      } else {
        this.clearReconnectAbortTimer();
      }
    });

    this.room.on(RoomEvent.LocalTrackPublished, (publication) => {
      if (publication.kind !== Track.Kind.Audio) return;
      if (this.desiredMicMuted && publication.track) {
        try {
          publication.track.mute();
        } catch (e) {
          console.warn(
            "[LiveKitService] failed to apply pending mute to new track:",
            e,
          );
        }
      }
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

  }

  private addParticipant(participant: RemoteParticipant): void {
    const info: ParticipantInfo = {
      identity: participant.identity,
      name: participant.name || participant.identity,
      isSpeaking: false,
      isMuted: false,
      isSharingScreen: false,
      audioTrack: null,
      cameraTrack: null,
      screenTrack: null,
      quality: participant.connectionQuality ?? ConnectionQuality.Unknown,
    };

    this.participants.set(participant.identity, info);

    participant.audioTrackPublications.forEach((publication) => {
      if (publication.track) {
        this.handleAudioTrack(publication.track as AudioTrack, participant);
      }
    });

    participant.videoTrackPublications.forEach((publication) => {
      if (publication.track) {
        this.handleVideoTrack(
          publication.track as VideoTrack,
          participant.identity,
        );
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

  private ensureRemoteAudioCtx(): AudioContext | null {
    if (this.remoteAudioCtx) return this.remoteAudioCtx;
    try {
      const Ctor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) return null;
      this.remoteAudioCtx = new Ctor();
      return this.remoteAudioCtx;
    } catch (e) {
      console.warn("[LiveKitService] AudioContext init failed:", e);
      return null;
    }
  }

  private uiVolumeToGain(uiVolume: number): number {
    return Math.max(0, Math.min(LiveKitService.REMOTE_GAIN_MAX, uiVolume));
  }

  private ensureParticipant(identity: string, name?: string): void {
    if (this.participants.has(identity)) return;
    this.participants.set(identity, {
      identity,
      name: name || identity,
      isSpeaking: false,
      isMuted: false,
      isSharingScreen: false,
      audioTrack: null,
      cameraTrack: null,
      screenTrack: null,
      quality: ConnectionQuality.Unknown,
    });
  }

  private handleVideoTrack(track: VideoTrack, identity: string): void {
    this.ensureParticipant(
      identity,
      this.room?.remoteParticipants.get(identity)?.name,
    );
    const isScreen = track.source === Track.Source.ScreenShare;
    this.updateParticipant(
      identity,
      isScreen ? { screenTrack: track } : { cameraTrack: track },
    );
  }

  private handleAudioTrack(
    track: AudioTrack,
    participant: RemoteParticipant,
  ): void {
    this.ensureParticipant(participant.identity, participant.name);
    this.updateParticipant(participant.identity, { audioTrack: track });

    const audioElement = track.attach();
    audioElement.volume = 1.0;
    document.body.appendChild(audioElement);
    const list = this.remoteAudioElements.get(participant.identity) ?? [];
    list.push(audioElement);
    this.remoteAudioElements.set(participant.identity, list);

    const desiredUiVolume = this.remoteVolumes.get(participant.identity) ?? 1.0;
    const ctx = this.ensureRemoteAudioCtx();
    if (ctx) {
      if (ctx.state === "suspended") {
        ctx.resume().catch(() => {});
      }
      try {
        const source = ctx.createMediaElementSource(audioElement);
        const gain = ctx.createGain();
        gain.gain.value = this.uiVolumeToGain(desiredUiVolume);
        const compressor = ctx.createDynamicsCompressor();
        compressor.threshold.value = -18;
        compressor.knee.value = 24;
        compressor.ratio.value = 12;
        compressor.attack.value = 0.003;
        compressor.release.value = 0.25;
        source.connect(gain);
        gain.connect(compressor);
        compressor.connect(ctx.destination);
        this.remoteSourceNodes.set(audioElement, {
          source,
          gain,
          identity: participant.identity,
        });
        const gains =
          this.remoteGainNodes.get(participant.identity) ?? [];
        gains.push(gain);
        this.remoteGainNodes.set(participant.identity, gains);
      } catch (e) {
        console.warn(
          "[LiveKitService] failed to wire remote gain node, falling back to element volume:",
          e,
        );
        audioElement.volume = desiredUiVolume;
      }
    } else {
      audioElement.volume = desiredUiVolume;
    }

    track.on(TrackEvent.Ended, () => {
      const nodes = this.remoteSourceNodes.get(audioElement);
      if (nodes) {
        try {
          nodes.gain.disconnect();
          nodes.source.disconnect();
        } catch {}
        const gains =
          this.remoteGainNodes.get(participant.identity) ?? [];
        const remainingGains = gains.filter((g) => g !== nodes.gain);
        if (remainingGains.length === 0) {
          this.remoteGainNodes.delete(participant.identity);
        } else {
          this.remoteGainNodes.set(participant.identity, remainingGains);
        }
        this.remoteSourceNodes.delete(audioElement);
      }
      audioElement.remove();
      const remaining = (
        this.remoteAudioElements.get(participant.identity) ?? []
      ).filter((el) => el !== audioElement);
      if (remaining.length === 0) {
        this.remoteAudioElements.delete(participant.identity);
      } else {
        this.remoteAudioElements.set(participant.identity, remaining);
      }
      this.updateParticipant(participant.identity, { audioTrack: null });
    });
  }

  setRemoteVolume(identity: string, volume: number): void {
    const v = Math.max(0, Math.min(LiveKitService.REMOTE_GAIN_MAX, volume));
    this.remoteVolumes.set(identity, v);
    const gain = this.uiVolumeToGain(v);
    const gains = this.remoteGainNodes.get(identity);
    if (gains && gains.length > 0) {
      for (const g of gains) g.gain.value = gain;
    } else {
      const els = this.remoteAudioElements.get(identity);
      if (els) for (const el of els) el.volume = Math.min(1, v);
    }
  }

  getRemoteVolume(identity: string): number {
    return this.remoteVolumes.get(identity) ?? 1.0;
  }

  private notifyParticipantsChange(): void {
    const list = this.getParticipants();
    this.onParticipantsChange?.(list);
    this.participantsListeners.forEach((fn) => {
      try {
        fn(list);
      } catch (e) {
        console.warn("[LiveKitService] participants listener error:", e);
      }
    });
  }

  subscribeParticipants(
    fn: (participants: ParticipantInfo[]) => void,
  ): () => void {
    this.participantsListeners.add(fn);
    return () => {
      this.participantsListeners.delete(fn);
    };
  }
}

let livekitService: LiveKitService | null = null;

export function getLiveKitService(
  onParticipantsChange?: (participants: ParticipantInfo[]) => void,
  onConnectionStateChange?: (connected: boolean) => void,
  onError?: (error: Error) => void,
  onUnexpectedDisconnect?: (reason: DisconnectReason) => void,
): LiveKitService {
  if (!livekitService) {
    livekitService = new LiveKitService(
      onParticipantsChange,
      onConnectionStateChange,
      onError,
      onUnexpectedDisconnect,
    );
  } else {
    livekitService.updateCallbacks(
      onParticipantsChange,
      onConnectionStateChange,
      onError,
      onUnexpectedDisconnect,
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
