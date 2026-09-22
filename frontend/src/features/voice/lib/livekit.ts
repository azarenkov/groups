import {
  ConnectionQuality,
  DisconnectReason,
  Room,
  Track,
  RemoteParticipant,
  AudioTrack,
  LocalAudioTrack,
  createLocalAudioTrack,
  RoomConnectOptions,
  VideoTrack,
} from "livekit-client";

import type { DfnPipeline } from "../../../shared/lib/dfn";
import {
  MAKEUP_BASE,
  MAKEUP_INTERVAL_MS,
  isSpeech,
  makeupGainFor,
  speechBandRatio,
} from "./makeupGain";
import { RemoteAudioGraph } from "./livekit/remoteAudio";
import { attachRoomListeners } from "./livekit/roomEvents";
import type {
  NoiseSuppressionMode,
  ParticipantInfo,
  VideoTile,
} from "./livekit/types";

export type { NoiseSuppressionMode, ParticipantInfo, VideoTile };

export interface ScreenShareQuality {
  width: number;
  height: number;
  frameRate: number;
  maxBitrate: number;
}

export { ConnectionQuality, DisconnectReason };

export const REMOTE_VOLUME_MAX = 200;
export const REMOTE_VOLUME_PLAIN_MAX = 100;

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
  private dfnStopMakeup: (() => void) | null = null;
  private dfnPrevChain: {
    gain: GainNode | null;
    gate: GainNode | null;
  } | null = null;
  private micGainAnalyser: AnalyserNode | null = null;
  private noiseMode: NoiseSuppressionMode = "standard";
  private screenShareQuality: ScreenShareQuality = {
    width: 1280,
    height: 720,
    frameRate: 60,
    maxBitrate: 4_000_000,
  };
  private desiredMicMuted: boolean = false;
  private remoteAudio = new RemoteAudioGraph(() => this.room);
  private noiseStrength: number = 1.0;
  private preferredMicId: string | null = null;
  private preferredSpeakerId: string | null = null;
  private deafened: boolean = false;
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
  private speakerSwitchSeq: number = 0;
  private speakerSwitchChain: Promise<void> = Promise.resolve();
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

  private localDeafenListeners: Set<(deafened: boolean) => void> = new Set();

  subscribeLocalDeafened(fn: (deafened: boolean) => void): () => void {
    this.localDeafenListeners.add(fn);
    return () => {
      this.localDeafenListeners.delete(fn);
    };
  }

  private emitLocalDeafened(): void {
    this.localDeafenListeners.forEach((fn) => {
      try {
        fn(this.deafened);
      } catch (e) {
        console.warn("[LiveKitService] localDeafened listener error:", e);
      }
    });
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
      if (this.deafened) this.publishDeafenAttribute();

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
    this.dfnStopMakeup?.();
    this.dfnStopMakeup = null;
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
      this.remoteAudio.clear();
      if (this.deafened) {
        this.deafened = false;
        this.remoteAudio.setDeafened(false);
        this.emitLocalDeafened();
      }
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
          const requestedMode = this.noiseMode;
          void this.applyNoiseSuppressionMode(requestedMode, true)
            .catch((e) =>
              console.warn(
                "[LiveKitService] apply noise mode after publish failed:",
                e,
              ),
            )
            .then(() => this.reconcileNoiseModeWithStore(requestedMode));
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
      this.dfnStopMakeup?.();
    this.dfnStopMakeup = null;
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

    let dfnModule: typeof import("../../../shared/lib/dfn");
    try {
      dfnModule = await import("../../../shared/lib/dfn");
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
      let stopMakeup: (() => void) | null = null;
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
        const makeup = ctx.createGain();
        makeup.gain.value = MAKEUP_BASE;
        const limiter = ctx.createDynamicsCompressor();
        limiter.threshold.value = -6;
        limiter.knee.value = 6;
        limiter.ratio.value = 12;
        limiter.attack.value = 0.003;
        limiter.release.value = 0.2;
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
        const denoisedAnalyser = ctx.createAnalyser();
        denoisedAnalyser.fftSize = 256;
        postSource.connect(denoisedAnalyser);
        const postDest = ctx.createMediaStreamDestination();
        postSource.connect(makeup);
        makeup.connect(limiter);
        limiter.connect(postGain);
        postGain.connect(postGate);
        postGate.connect(postDest);
        stopMakeup = trackDenoiserMakeup(
          ctx,
          postAnalyser,
          denoisedAnalyser,
          makeup,
          postGate,
        );
        const postTrack = postDest.stream.getAudioTracks()[0];
        const builtPipeline = pipeline;
        const builtCtx = ctx;
        const discard = async () => {
          stopMakeup?.();
          stopMakeup = null;
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

        this.dfnStopMakeup?.();
        this.dfnStopMakeup = stopMakeup;
        stopMakeup = null;
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
        stopMakeup?.();
        stopMakeup = null;
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

    this.dfnStopMakeup?.();
    this.dfnStopMakeup = null;
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
      const quality = this.screenShareQuality;
      await this.room.localParticipant.setScreenShareEnabled(
        true,
        {
          resolution: {
            width: quality.width,
            height: quality.height,
            frameRate: quality.frameRate,
          },
          contentHint: quality.frameRate >= 60 ? "motion" : "detail",
          audio: true,
        },
        {
          simulcast: false,
          videoCodec: "vp9",
          screenShareEncoding: {
            maxBitrate: quality.maxBitrate,
            maxFramerate: quality.frameRate,
          },
        },
      );
    } else {
      await this.room.localParticipant.setScreenShareEnabled(false);
    }
    this.notifyParticipantsChange();
  }

  getScreenShareQuality(): ScreenShareQuality {
    return this.screenShareQuality;
  }

  async setScreenShareQuality(quality: ScreenShareQuality): Promise<void> {
    this.screenShareQuality = quality;
    const publication = this.room?.localParticipant.getTrackPublication(
      Track.Source.ScreenShare,
    );
    const track = publication?.track;
    if (!track) return;
    try {
      await track.mediaStreamTrack.applyConstraints({
        width: { ideal: quality.width },
        height: { ideal: quality.height },
        frameRate: { ideal: quality.frameRate, max: quality.frameRate },
      });
    } catch (e) {
      console.warn("[LiveKitService] screen share constraints rejected:", e);
    }
    const sender = (track as { sender?: RTCRtpSender }).sender;
    if (!sender) return;
    try {
      const parameters = sender.getParameters();
      for (const encoding of parameters.encodings ?? []) {
        encoding.maxBitrate = quality.maxBitrate;
        encoding.maxFramerate = quality.frameRate;
      }
      await sender.setParameters(parameters);
    } catch (e) {
      console.warn("[LiveKitService] screen share encoding update failed:", e);
    }
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
    attachRoomListeners(this.room, {
      onConnected: () => this.onConnectionStateChange?.(true),
      onLocalTracksChanged: () => this.notifyParticipantsChange(),
      onParticipantConnected: (participant) => this.addParticipant(participant),
      onParticipantDisconnected: (identity) => this.removeParticipant(identity),
      onAudioTrack: (track, participant) =>
        this.handleAudioTrack(track, participant),
      onVideoTrack: (track, identity) => this.handleVideoTrack(track, identity),
      onParticipantPatch: (identity, patch) =>
        this.updateParticipant(identity, patch),
      onDisconnected: (reason) => this.handleRoomDisconnected(reason),
      onLocalQuality: (quality) => this.emitLocalQuality(quality),
      onReconnectingStarted: () => this.startReconnectAbortTimer(),
      onReconnectingSettled: () => this.clearReconnectAbortTimer(),
      onLocalAudioPublished: (publication) => {
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
      },
      onMediaDevicesError: (error) => this.onError?.(error),
    });
  }

  private handleRoomDisconnected(reason?: DisconnectReason): void {
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
  }

  private addParticipant(participant: RemoteParticipant): void {
    const info: ParticipantInfo = {
      identity: participant.identity,
      name: participant.name || participant.identity,
      isSpeaking: participant.isSpeaking,
      isMuted: false,
      isDeafened: participant.attributes?.deafened === "1",
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

  private ensureParticipant(identity: string, name?: string): void {
    if (this.participants.has(identity)) return;
    const remote = this.room?.remoteParticipants.get(identity);
    this.participants.set(identity, {
      identity,
      name: name || remote?.name || identity,
      isSpeaking: remote?.isSpeaking ?? false,
      isMuted: false,
      isDeafened: remote?.attributes?.deafened === "1",
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

    this.remoteAudio.attach(track, participant.identity, () =>
      this.updateParticipant(participant.identity, { audioTrack: null }),
    );
  }

  subscribeRemoteSpeaking(fn: (ids: Set<string>) => void): () => void {
    return this.remoteAudio.subscribeSpeaking(fn);
  }

  remoteSpeakingDetected(): boolean {
    return this.remoteAudio.tracksSpeaking();
  }

  micGateActive(): boolean {
    return this.micGateTimer !== null;
  }

  setRemoteVolume(identity: string, volume: number): void {
    this.remoteAudio.setVolume(identity, volume);
  }

  getRemoteVolume(identity: string): number {
    return this.remoteAudio.getVolume(identity);
  }

  supportsRemoteBoost(identity: string): boolean {
    return this.remoteAudio.supportsBoost(identity);
  }

  private reconcileNoiseModeWithStore(requested: NoiseSuppressionMode): void {
    if (!this.room) return;
    const actual = this.noiseMode;
    if (actual === requested) return;
    void import("../store/voiceSettingsStore").then(
      ({ useVoiceSettingsStore }) => {
        const store = useVoiceSettingsStore.getState();
        if (store.noiseMode === requested) store.update({ noiseMode: actual });
      },
    );
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
        const requestedMode = prefs.noiseMode;
        void this.setNoiseSuppressionMode(requestedMode)
          .catch((e) =>
            console.warn("[LiveKitService] live noise toggle failed:", e),
          )
          .then(() => this.reconcileNoiseModeWithStore(requestedMode));
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
      this.remoteAudio.setSpeakerId(prefs.speakerDeviceId);
      if (this.room) {
        void this.switchDevice("audiooutput", this.preferredSpeakerId);
      }
    }
    if (prefs.outputVolume !== undefined) {
      this.remoteAudio.setMasterVolume(prefs.outputVolume);
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
      if (kind === "audiooutput") {
        const seq = ++this.speakerSwitchSeq;
        const run = async () => {
          if (seq !== this.speakerSwitchSeq || !this.room) return;
          const target = await this.resolveDeviceTarget(kind, deviceId);
          if (!target || seq !== this.speakerSwitchSeq || !this.room) return;
          await this.room.switchActiveDevice(kind, target);
          if (seq === this.speakerSwitchSeq) await this.remoteAudio.applySink(target);
        };
        this.speakerSwitchChain = this.speakerSwitchChain.then(run, run);
        await this.speakerSwitchChain;
        return;
      }
      const target = await this.resolveDeviceTarget(kind, deviceId);
      if (target && this.room) {
        await this.room.switchActiveDevice(kind, target);
      }
    } catch (e) {
      console.warn(`[LiveKitService] ${kind} switch failed:`, e);
    }
  }

  private async resolveDeviceTarget(
    kind: MediaDeviceKind,
    deviceId: string | null,
  ): Promise<string | null> {
    if (deviceId) return deviceId;
    const devices = await navigator.mediaDevices.enumerateDevices();
    const match =
      devices.find((d) => d.kind === kind && d.deviceId === "default") ??
      devices.find((d) => d.kind === kind);
    return match?.deviceId ?? null;
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
        const active = now < holdUntil;
        this.setLocalSpeaking(active && !this.desiredMicMuted);
        if (this.pushToTalkEnforced && this.pttHolding) {
          gate.gain.value = 1;
          return;
        }
        gate.gain.value = active ? 1 : 0;
      }, 50);
    });
  }

  private stopMicGate(): void {
    if (this.micGateTimer) {
      clearInterval(this.micGateTimer);
      this.micGateTimer = null;
    }
  }

  setDeafened(deafened: boolean): void {
    if (this.deafened === deafened) return;
    this.deafened = deafened;
    this.remoteAudio.setDeafened(deafened);
    this.publishDeafenAttribute();
    this.emitLocalDeafened();
  }

  isDeafened(): boolean {
    return this.deafened;
  }

  private publishDeafenAttribute(): void {
    const room = this.room;
    if (!room || room.state !== "connected") return;
    void room.localParticipant
      .setAttributes({ deafened: this.deafened ? "1" : "" })
      .catch((e) =>
        console.warn("[LiveKitService] failed to publish deafen attribute:", e),
      );
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

function analyserRms(analyser: AnalyserNode, buffer: Float32Array): number {
  analyser.getFloatTimeDomainData(buffer);
  let sum = 0;
  for (let i = 0; i < buffer.length; i++) sum += buffer[i] * buffer[i];
  return Math.sqrt(sum / buffer.length);
}

function trackDenoiserMakeup(
  ctx: AudioContext,
  input: AnalyserNode,
  denoised: AnalyserNode,
  makeup: GainNode,
  gate: GainNode,
): () => void {
  const inputBuffer = new Float32Array(input.fftSize);
  const denoisedBuffer = new Float32Array(denoised.fftSize);
  const spectrum = new Uint8Array(input.frequencyBinCount);
  let gain = makeup.gain.value;

  const timer = setInterval(() => {
    const inputRms = analyserRms(input, inputBuffer);
    const denoisedRms = analyserRms(denoised, denoisedBuffer);
    input.getByteFrequencyData(spectrum);
    const speech = isSpeech(
      inputRms,
      speechBandRatio(spectrum, ctx.sampleRate),
      gate.gain.value > 0,
    );
    const next = makeupGainFor(inputRms, denoisedRms, gain, speech);
    if (next === gain) return;
    gain = next;
    makeup.gain.setTargetAtTime(gain, ctx.currentTime, 0.1);
  }, MAKEUP_INTERVAL_MS);

  return () => clearInterval(timer);
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
