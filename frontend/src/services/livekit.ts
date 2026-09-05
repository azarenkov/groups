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
  private targetRoomName: string | null = null;
  private dfnPipeline: DfnPipeline | null = null;
  private dfnContext: AudioContext | null = null;
  private dfnRawTrack: MediaStreamTrack | null = null;
  private dfnPrevPublishedTrack: MediaStreamTrack | null = null;
  private dfnPrevChain: {
    gain: GainNode | null;
    gate: GainNode | null;
  } | null = null;
  private micGainAnalyser: AnalyserNode | null = null;
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
  private preferredMicId: string | null = null;
  private preferredSpeakerId: string | null = null;
  private masterVolume: number = 1.0;
  private micVolume: number = 1.0;
  private micGainCtx: AudioContext | null = null;
  private micGainNode: GainNode | null = null;
  private micSourceTrack: MediaStreamTrack | null = null;
  private micGateGain: GainNode | null = null;
  private micGateTimer: ReturnType<typeof setInterval> | null = null;
  private micGainRetryCleanup: (() => void) | null = null;
  private micBypassedTrack: MediaStreamTrack | null = null;
  private defaultNoiseMode: NoiseSuppressionMode = "standard";
  private pushToTalkEnforced: boolean = false;
  private muteBeforePushToTalk: boolean = false;
  private pttHolding: boolean = false;
  private muteRequestSeq: number = 0;
  private micSwitchChain: Promise<void> = Promise.resolve();
  private micSwitchSeq: number = 0;
  private micRebuildPending: boolean = false;
  private micEnableInFlight: boolean = false;
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

  async connect(
    token: string,
    url: string,
    roomName?: string,
    startMuted?: boolean,
  ): Promise<void> {
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
      this.targetRoomName = roomName ?? null;
      const muteSeqAtConnect = this.muteRequestSeq;
      console.log("Connecting to LiveKit...", { url });

      this.noiseMode = this.defaultNoiseMode;
      this.room = new Room({
        adaptiveStream: true,
        dynacast: true,
        audioCaptureDefaults: {
          deviceId: this.preferredMicId ?? undefined,
          echoCancellation: true,
          noiseSuppression: this.defaultNoiseMode !== "off",
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
      if (this.preferredSpeakerId) {
        void this.switchDevice("audiooutput", this.preferredSpeakerId);
      }

      this.room.remoteParticipants.forEach((participant) => {
        this.addParticipant(participant);
      });

      const muteChanged = this.muteRequestSeq !== muteSeqAtConnect;
      if (!startMuted && !muteChanged) {
        this.muteBeforePushToTalk = false;
        if (this.pushToTalkEnforced) {
          this.desiredMicMuted = true;
        }
        await this.enableMicrophone();
      } else if (startMuted && !muteChanged) {
        this.desiredMicMuted = true;
        this.muteBeforePushToTalk = true;
      } else if (!this.desiredMicMuted) {
        await this.enableMicrophone();
      }
      if (this.pushToTalkEnforced) {
        this.applyMicMuted(!(this.pttHolding && !this.muteBeforePushToTalk));
      }

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
      this.targetRoomName = null;
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
    this.dfnPrevPublishedTrack = null;
    this.dfnPrevChain = null;
  }

  async disconnect(): Promise<void> {
    this.clearReconnectAbortTimer();
    this.micSwitchSeq += 1;
    await this.tearDownDfn();
    this.teardownMicGain();
    if (this.room) {
      console.log("[LiveKitService] Disconnecting from room");
      await this.room.disconnect();
      this.room = null;
      this.targetRoomName = null;
      this.participants.clear();
      this.isConnecting = false;
      this.desiredMicMuted = false;
      this.micRebuildPending = false;
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
    const switchSeqAtStart = this.micSwitchSeq;
    this.micEnableInFlight = true;

    const publish = async (deviceId?: string) => {
      const startedRoom = this.room;
      const track = await createLocalAudioTrack({
        deviceId,
        echoCancellation: true,
        noiseSuppression: this.noiseMode !== "off",
        autoGainControl: true,
      });
      const room = this.room;
      if (
        !room ||
        room !== startedRoom ||
        this.micSwitchSeq !== switchSeqAtStart
      ) {
        try {
          track.stop();
        } catch {}
        return;
      }
      await track.mute().catch(() => {});
      try {
        await room.localParticipant.publishTrack(track);
      } catch (error) {
        try {
          track.stop();
        } catch {}
        throw error;
      }
      if (this.room === room && this.micSwitchSeq === switchSeqAtStart) {
        await this.wireMicGain(track);
        if (this.room !== room || this.micSwitchSeq !== switchSeqAtStart) {
          this.teardownMicGain();
        } else {
          this.applyMicMuted(this.desiredMicMuted);
          void this.applyNoiseSuppressionMode(this.noiseMode, true).catch((e) =>
            console.warn(
              "[LiveKitService] apply noise mode after publish failed:",
              e,
            ),
          );
        }
      }
      this.emitLocalTrack();
    };

    try {
      let deviceId = this.preferredMicId ?? undefined;
      if (deviceId) {
        try {
          const devices = await navigator.mediaDevices.enumerateDevices();
          if (
            !devices.some(
              (d) => d.kind === "audioinput" && d.deviceId === deviceId,
            )
          ) {
            deviceId = undefined;
          }
        } catch {}
      }
      try {
        await publish(deviceId);
      } catch (error) {
        if (deviceId === undefined) throw error;
        console.warn(
          "[LiveKitService] saved microphone unavailable, falling back to default:",
          error,
        );
        await publish(undefined);
      }
    } catch (error) {
      console.error("Failed to enable microphone:", error);
      this.onError?.(
        error instanceof Error
          ? error
          : new Error("Failed to enable microphone"),
      );
    } finally {
      this.micEnableInFlight = false;
    }
  }

  private async enableMicrophoneIfNeeded(): Promise<void> {
    if (!this.room) return;
    if (this.desiredMicMuted) return;
    if (this.micEnableInFlight) return;
    if (this.room.localParticipant.audioTrackPublications.size > 0) return;
    await this.enableMicrophone();
  }

  async disableMicrophone(): Promise<void> {
    if (!this.room) return;

    this.teardownMicGain();

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
    this.dfnPrevPublishedTrack = null;
      this.dfnPrevChain = null;
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
    this.muteRequestSeq += 1;
    if (this.pushToTalkEnforced) {
      this.muteBeforePushToTalk = muted;
      if (!muted && !this.pttHolding) return;
    }
    this.applyMicMuted(muted);
  }

  pttSetTransmitting(active: boolean): void {
    if (!this.pushToTalkEnforced) return;
    this.pttHolding = active;
    if (active && this.muteBeforePushToTalk) return;
    this.applyMicMuted(!active);
  }

  private applyMicMuted(muted: boolean): void {
    this.desiredMicMuted = muted;
    if (!this.room) return;

    const audioTracks = Array.from(
      this.room.localParticipant.audioTrackPublications.values(),
    );

    if (!muted && audioTracks.length === 0) {
      this.micSwitchChain = this.micSwitchChain
        .then(() => this.enableMicrophoneIfNeeded())
        .catch((err) =>
          console.warn("[livekit] failed to publish microphone on unmute:", err),
        );
      return;
    }

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
    if (this.micSourceTrack) {
      this.micSourceTrack.enabled = !muted;
    }
    if (this.micBypassedTrack) {
      this.micBypassedTrack.enabled = !muted && this.micVolume > 0;
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
    if (this.room) {
      const publications = Array.from(
        this.room.localParticipant.audioTrackPublications.values(),
      );
      for (const pub of publications) {
        const track = pub.track?.mediaStreamTrack;
        if (track) return track;
      }
    }
    if (this.dfnPipeline) return this.dfnPipeline.outputTrack;
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

      const track = this.micSourceTrack ?? this.getLocalAudioMediaStreamTrack();
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
    const roomAtStart = this.room;
    const seqAtStart = this.micSwitchSeq;
    const stale = () =>
      this.room !== roomAtStart || this.micSwitchSeq !== seqAtStart;
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
    if (stale()) return;
    if (!localTrack) {
      const err = new Error("Local audio track did not appear in time");
      console.warn("[LiveKitService] enableDfnPipeline:", err.message);
      this.noiseMode = "standard";
      this.onError?.(err);
      return;
    }
    const publishedTrack = localTrack.mediaStreamTrack;
    const rawTrack = this.micSourceTrack ?? publishedTrack;
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
    if (stale()) return;

    let lastErr: unknown;
    for (let attempt = 1; attempt <= 3; attempt++) {
      let step = "init";
      let ctx: AudioContext | null = null;
      let pipeline: DfnPipeline | null = null;
      try {
        step = "applyConstraints";
        try {
          await (this.micSourceTrack ?? rawTrack).applyConstraints({ noiseSuppression: false });
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
            await (this.micSourceTrack ?? rawTrack).applyConstraints({ noiseSuppression: true });
          } catch {}
          this.onError?.(new Error("Noise suppression models unavailable"));
          return;
        }
        step = "postChain";
        const postSource = ctx.createMediaStreamSource(
          new MediaStream([pipeline.outputTrack]),
        );
        const postGain = ctx.createGain();
        postGain.gain.value = this.micVolume;
        const postGate = ctx.createGain();
        postGate.gain.value = 1;
        const postAnalyser = ctx.createAnalyser();
        postAnalyser.fftSize = 256;
        const preSource = ctx.createMediaStreamSource(
          new MediaStream([rawTrack]),
        );
        preSource.connect(postAnalyser);
        const postDest = ctx.createMediaStreamDestination();
        postSource.connect(postGain);
        postGain.connect(postGate);
        postGate.connect(postDest);
        const postTrack = postDest.stream.getAudioTracks()[0];
        const builtPipeline = pipeline;
        const builtCtx = ctx;
        const discard = async () => {
          try {
            await builtPipeline.dispose();
          } catch {}
          try {
            builtPipeline.outputTrack.stop();
          } catch {}
          try {
            postTrack?.stop();
          } catch {}
          try {
            await builtCtx.close();
          } catch {}
        };
        if (stale()) {
          await discard();
          return;
        }
        step = "replaceTrack";
        await localTrack.replaceTrack(postTrack ?? pipeline.outputTrack);
        if (stale()) {
          await discard();
          return;
        }

        this.dfnPipeline = pipeline;
        this.dfnContext = ctx;
        this.dfnRawTrack = rawTrack;
        this.dfnPrevPublishedTrack = publishedTrack;
        if (postTrack) {
          this.dfnPrevChain = {
            gain: this.micGainNode,
            gate: this.micGateGain,
          };
          this.micGainNode = postGain;
          this.micGateGain = postGate;
          this.startMicGate(postAnalyser);
        }
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
      await (this.micSourceTrack ?? rawTrack).applyConstraints({ noiseSuppression: true });
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
          await localTrack.replaceTrack(this.dfnPrevPublishedTrack ?? rawTrack);
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
    this.dfnPrevPublishedTrack = null;
    if (this.dfnPrevChain) {
      this.micGainNode = this.dfnPrevChain.gain;
      this.micGateGain = this.dfnPrevChain.gate;
      this.dfnPrevChain = null;
      if (this.micGainNode) {
        this.micGainNode.gain.value = this.micVolume;
      }
      if (this.micGateGain && this.micGainAnalyser) {
        this.micGateGain.gain.value = 1;
        this.startMicGate(this.micGainAnalyser);
      } else {
        this.stopMicGate();
      }
    }
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

  getRoomName(): string | null {
    return this.room?.state === "connected" ? this.room.name : null;
  }

  getActiveRoomName(): string | null {
    if (!this.room) return null;
    return this.targetRoomName ?? this.room.name ?? null;
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
      this.micSwitchSeq += 1;
      this.teardownMicGain();
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
    });

    this.room.on(RoomEvent.MediaDevicesError, (error) => {
      const dismissed =
        error.name === "NotAllowedError" ||
        error.name === "AbortError" ||
        error.message.includes("Permission dismissed") ||
        error.message.includes("Could not start video source");
      if (dismissed) {
        console.warn("[LiveKitService] media device request dismissed:", error);
        return;
      }
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
      if (this.preferredSpeakerId) {
        this.applySinkToCtx(this.preferredSpeakerId);
      }
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

    if (track.attachedElements.length > 0) {
      return;
    }

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
        gain.gain.value = this.effectiveGain(desiredUiVolume);
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
        audioElement.volume = Math.min(1, desiredUiVolume * this.masterVolume);
      }
    } else {
      audioElement.volume = Math.min(1, desiredUiVolume * this.masterVolume);
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
    const gain = this.effectiveGain(v);
    const gains = this.remoteGainNodes.get(identity);
    if (gains && gains.length > 0) {
      for (const g of gains) g.gain.value = gain;
    } else {
      const els = this.remoteAudioElements.get(identity);
      if (els) for (const el of els) el.volume = Math.min(1, v * this.masterVolume);
    }
  }

  getRemoteVolume(identity: string): number {
    return this.remoteVolumes.get(identity) ?? 1.0;
  }

  setAudioPreferences(prefs: {
    micDeviceId?: string | null;
    speakerDeviceId?: string | null;
    outputVolume?: number;
    micVolume?: number;
    noiseMode?: NoiseSuppressionMode;
  }): void {
    if (prefs.noiseMode !== undefined) {
      const changed = prefs.noiseMode !== this.defaultNoiseMode;
      this.defaultNoiseMode = prefs.noiseMode;
      if (changed && this.room && prefs.noiseMode !== this.noiseMode) {
        void this.setNoiseSuppressionMode(prefs.noiseMode).catch((e) =>
          console.warn("[LiveKitService] live noise toggle failed:", e),
        );
      }
    }
    if (
      prefs.micDeviceId !== undefined &&
      prefs.micDeviceId !== this.preferredMicId
    ) {
      this.preferredMicId = prefs.micDeviceId;
      if (this.room) {
        void this.switchDevice("audioinput", this.preferredMicId);
      }
    }
    if (
      prefs.speakerDeviceId !== undefined &&
      prefs.speakerDeviceId !== this.preferredSpeakerId
    ) {
      this.preferredSpeakerId = prefs.speakerDeviceId;
      if (this.room) {
        void this.switchDevice("audiooutput", this.preferredSpeakerId);
      }
    }
    if (prefs.outputVolume !== undefined) {
      this.masterVolume = Math.max(0, Math.min(1, prefs.outputVolume));
      this.applyMasterVolume();
    }
    if (prefs.micVolume !== undefined) {
      this.micVolume = Math.max(0, Math.min(1, prefs.micVolume));
      if (this.micGainNode) {
        this.micGainNode.gain.value = this.micVolume;
      }
      if (this.micBypassedTrack) {
        this.micBypassedTrack.enabled =
          !this.desiredMicMuted && this.micVolume > 0;
      }
    }
  }

  private async wireMicGain(track: LocalAudioTrack): Promise<void> {
    this.teardownMicGain();
    const raw = track.mediaStreamTrack;
    if (!raw) return;
    let ctx: AudioContext | null = null;
    try {
      const Ctor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) {
        console.warn(
          "[LiveKitService] AudioContext unavailable; publishing unprocessed mic",
        );
        this.micBypassedTrack = raw;
        raw.enabled = !this.desiredMicMuted && this.micVolume > 0;
        return;
      }
      ctx = new Ctor();
      const audioCtx = ctx;
      if (audioCtx.state === "suspended") {
        await Promise.race([
          audioCtx.resume().catch(() => {}),
          new Promise((resolve) => setTimeout(resolve, 3000)),
        ]);
      }
      if (audioCtx.state !== "running") {
        console.warn(
          "[LiveKitService] mic gain AudioContext not running; publishing unprocessed mic without gate:",
          audioCtx.state,
        );
        await audioCtx.close().catch(() => {});
        this.micBypassedTrack = raw;
        raw.enabled = !this.desiredMicMuted && this.micVolume > 0;
        this.scheduleMicGainRetry(track);
        return;
      }
      const source = audioCtx.createMediaStreamSource(new MediaStream([raw]));
      const gain = audioCtx.createGain();
      gain.gain.value = this.micVolume;
      const gate = audioCtx.createGain();
      gate.gain.value = 1;
      const analyser = audioCtx.createAnalyser();
      analyser.fftSize = 256;
      const destination = audioCtx.createMediaStreamDestination();
      source.connect(gain);
      gain.connect(gate);
      gain.connect(analyser);
      gate.connect(destination);
      const processed = destination.stream.getAudioTracks()[0];
      if (!processed) {
        await audioCtx.close().catch(() => {});
        return;
      }
      await track.replaceTrack(processed);
      this.micBypassedTrack = null;
      raw.enabled = !this.desiredMicMuted;
      this.micGainCtx = ctx;
      this.micGainNode = gain;
      this.micGateGain = gate;
      this.micGainAnalyser = analyser;
      this.micSourceTrack = raw;
      this.startMicGate(analyser);
    } catch (e) {
      console.warn("[LiveKitService] mic gain wiring failed:", e);
      if (ctx && this.micGainCtx !== ctx) {
        void ctx.close().catch(() => {});
      }
      this.micBypassedTrack = raw;
      raw.enabled = !this.desiredMicMuted && this.micVolume > 0;
      this.scheduleMicGainRetry(track);
    }
  }

  private teardownMicGain(): void {
    this.clearMicGainRetry();
    this.stopMicGate();
    this.micBypassedTrack = null;
    this.micGateGain = null;
    if (this.micGainCtx) {
      void this.micGainCtx.close().catch(() => {});
    }
    if (this.micSourceTrack) {
      try {
        this.micSourceTrack.stop();
      } catch {}
    }
    this.micGainCtx = null;
    this.micGainNode = null;
    this.micGainAnalyser = null;
    this.micSourceTrack = null;
  }

  setPushToTalkEnforced(enabled: boolean): void {
    if (enabled === this.pushToTalkEnforced) return;
    this.pushToTalkEnforced = enabled;
    this.pttHolding = false;
    if (enabled) {
      this.muteBeforePushToTalk = this.desiredMicMuted;
      this.applyMicMuted(true);
    } else {
      this.applyMicMuted(this.muteBeforePushToTalk);
    }
  }

  isPushToTalkEnforced(): boolean {
    return this.pushToTalkEnforced;
  }

  private async switchDevice(
    kind: MediaDeviceKind,
    deviceId: string | null,
  ): Promise<void> {
    if (!this.room) return;
    try {
      if (kind === "audioinput") {
        const seq = ++this.micSwitchSeq;
        const originRoom = this.room;
        const run = async () => {
          if (
            seq !== this.micSwitchSeq ||
            !this.room ||
            this.room !== originRoom
          ) {
            return;
          }
          const hasCapture =
            this.micRebuildPending ||
            this.micEnableInFlight ||
            this.micSourceTrack !== null ||
            this.dfnPipeline !== null ||
            this.room.localParticipant.audioTrackPublications.size > 0;
          if (!hasCapture) return;
          const mode = this.noiseMode;
          await this.disableMicrophone();
          this.micRebuildPending = true;
          if (seq !== this.micSwitchSeq) return;
          await this.enableMicrophone();
          this.micRebuildPending =
            this.room !== null &&
            this.micSourceTrack === null &&
            this.dfnPipeline === null &&
            this.micBypassedTrack === null &&
            this.room.localParticipant.audioTrackPublications.size === 0;
          if (mode === "high") {
            this.noiseMode = "standard";
            await this.setNoiseSuppressionMode("high");
          }
          this.applyMicMuted(this.desiredMicMuted);
        };
        this.micSwitchChain = this.micSwitchChain.then(run, run);
        await this.micSwitchChain;
        return;
      }
      let target = deviceId;
      if (!target) {
        const devices = await navigator.mediaDevices.enumerateDevices();
        const match =
          devices.find((d) => d.kind === kind && d.deviceId === "default") ??
          devices.find((d) => d.kind === kind);
        target = match?.deviceId ?? null;
      }
      if (target) {
        await this.room.switchActiveDevice(kind, target);
        if (kind === "audiooutput") {
          this.applySinkToCtx(target);
        }
      }
    } catch (e) {
      console.warn(`[LiveKitService] ${kind} switch failed:`, e);
    }
  }

  private scheduleMicGainRetry(track: LocalAudioTrack): void {
    this.clearMicGainRetry();
    const retry = () => {
      this.clearMicGainRetry();
      const seqAtRetry = this.micSwitchSeq;
      const roomAtRetry = this.room;
      const run = async () => {
        if (
          this.micSwitchSeq !== seqAtRetry ||
          !this.room ||
          this.room !== roomAtRetry ||
          this.micSourceTrack
        ) {
          return;
        }
        const publication = Array.from(
          this.room.localParticipant.audioTrackPublications.values(),
        )[0];
        if (publication?.track !== track) return;
        await this.wireMicGain(track);
      };
      this.micSwitchChain = this.micSwitchChain.then(run, run);
    };
    window.addEventListener("pointerdown", retry, { once: true });
    window.addEventListener("keydown", retry, { once: true });
    const timer = window.setTimeout(retry, 2000);
    this.micGainRetryCleanup = () => {
      window.clearTimeout(timer);
      window.removeEventListener("pointerdown", retry);
      window.removeEventListener("keydown", retry);
    };
  }

  private clearMicGainRetry(): void {
    this.micGainRetryCleanup?.();
    this.micGainRetryCleanup = null;
  }

  private startMicGate(analyser: AnalyserNode): void {
    this.stopMicGate();
    const gateAtStart = this.micGateGain;
    if (!gateAtStart) return;
    const data = new Uint8Array(analyser.frequencyBinCount);
    const tdata = new Uint8Array(analyser.fftSize);
    let holdUntil = 0;
    let noiseFloor = 8;
    void import("../store/voiceSettingsStore").then(({ useVoiceSettingsStore }) => {
      if (this.micGateGain !== gateAtStart) return;
      this.micGateTimer = setInterval(() => {
        if (this.micGateGain !== gateAtStart) return;
        const gate = gateAtStart;
        const { autoSensitivity, sensitivity } =
          useVoiceSettingsStore.getState();
        if (this.pushToTalkEnforced && this.pttHolding) {
          gate.gain.value = 1;
          return;
        }
        analyser.getByteFrequencyData(data);
        let max = 0;
        for (let i = 0; i < data.length; i++) {
          if (data[i] > max) max = data[i];
        }
        const now = Date.now();
        let passed: boolean;
        if (autoSensitivity) {
          passed = max > Math.min(45, noiseFloor + 10);
          if (!passed && now >= holdUntil) {
            if (max < noiseFloor + 6) {
              noiseFloor = noiseFloor * 0.95 + max * 0.05;
            } else {
              noiseFloor = Math.min(noiseFloor + 0.2, 60);
            }
          }
        } else {
          analyser.getByteTimeDomainData(tdata);
          let sum = 0;
          for (let i = 0; i < tdata.length; i++) {
            const d = (tdata[i] - 128) / 128;
            sum += d * d;
          }
          const db =
            20 * Math.log10(Math.max(Math.sqrt(sum / tdata.length), 1e-6));
          passed = db > -72 + 0.72 * sensitivity;
        }
        if (passed) holdUntil = now + 250;
        gate.gain.value = now < holdUntil ? 1 : 0;
      }, 50);
    });
  }

  private stopMicGate(): void {
    if (this.micGateTimer) {
      clearInterval(this.micGateTimer);
      this.micGateTimer = null;
    }
  }

  private applySinkToCtx(sinkId: string): void {
    const ctx = this.remoteAudioCtx as
      | (AudioContext & { setSinkId?: (id: string) => Promise<void> })
      | null;
    ctx?.setSinkId?.(sinkId).catch((e) =>
      console.warn("[LiveKitService] audio context sink switch failed:", e),
    );
  }

  private effectiveGain(uiVolume: number): number {
    return this.uiVolumeToGain(uiVolume) * this.masterVolume;
  }

  private applyMasterVolume(): void {
    for (const [identity, gains] of Array.from(this.remoteGainNodes.entries())) {
      const ui = this.remoteVolumes.get(identity) ?? 1.0;
      for (const g of gains) g.gain.value = this.effectiveGain(ui);
    }
    for (const [identity, els] of Array.from(
      this.remoteAudioElements.entries(),
    )) {
      if (this.remoteGainNodes.has(identity)) continue;
      const ui = this.remoteVolumes.get(identity) ?? 1.0;
      for (const el of els) el.volume = Math.min(1, ui * this.masterVolume);
    }
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
