import { TrackEvent } from "livekit-client";
import type { AudioTrack, Room } from "livekit-client";

export const SPEAKING_RMS = 0.012;
export const SPEAKING_HOLD_MS = 250;
export const SPEAKING_INTERVAL_MS = 50;

export function speakingFor(
  rms: number,
  now: number,
  holdUntil: number,
): { speaking: boolean; holdUntil: number } {
  const next = rms >= SPEAKING_RMS ? now + SPEAKING_HOLD_MS : holdUntil;
  return { speaking: now < next, holdUntil: next };
}

export interface SpeakingState {
  speaking: Set<string>;
  tracked: Set<string>;
}

interface SpeechAnalyser {
  node: AnalyserNode;
  buffer: Float32Array;
}

export class RemoteAudioGraph {
  static readonly GAIN_MAX = 6.0;

  private volumes: Map<string, number> = new Map();
  private elements: Map<string, HTMLMediaElement[]> = new Map();
  private gainNodes: Map<string, GainNode[]> = new Map();
  private sourceNodes: WeakMap<
    HTMLMediaElement,
    {
      source: MediaElementAudioSourceNode;
      gain: GainNode;
      identity: string;
      analyser?: SpeechAnalyser;
    }
  > = new WeakMap();
  private analysers: Map<string, SpeechAnalyser[]> = new Map();
  private holdUntil: Map<string, number> = new Map();
  private speaking: Set<string> = new Set();
  private speakingListeners: Set<(state: SpeakingState) => void> = new Set();
  private speakingTimer: ReturnType<typeof setInterval> | null = null;
  private ctx: AudioContext | null = null;
  private disabled: boolean = false;
  private masterVolume: number = 1.0;
  private deafened: boolean = false;
  private speakerId: string | null = null;

  constructor(private readonly roomProvider: () => Room | null) {}

  setSpeakerId(speakerId: string | null): void {
    this.speakerId = speakerId;
  }

  setMasterVolume(volume: number): void {
    this.masterVolume = Math.max(0, Math.min(1, volume));
    this.applyMasterVolume();
  }

  setDeafened(deafened: boolean): void {
    this.deafened = deafened;
    this.applyMasterVolume();
  }

  clear(): void {
    this.volumes.clear();
    this.elements.clear();
    this.gainNodes.clear();
    this.sourceNodes = new WeakMap();
    for (const identity of Array.from(this.analysers.keys())) {
      this.dropAnalysers(identity);
    }
    this.holdUntil.clear();
    this.stopSpeakingLoop();
    this.publishSpeaking(new Set());
  }

  subscribeSpeaking(fn: (state: SpeakingState) => void): () => void {
    this.speakingListeners.add(fn);
    fn(this.speakingState());
    return () => {
      this.speakingListeners.delete(fn);
    };
  }

  tracksSpeaking(): boolean {
    return this.analysers.size > 0;
  }

  private speakingState(): SpeakingState {
    return {
      speaking: new Set(this.speaking),
      tracked: new Set(this.analysers.keys()),
    };
  }

  private publishSpeaking(ids: Set<string>): void {
    this.speaking = ids;
    const state = this.speakingState();
    this.speakingListeners.forEach((fn) => {
      try {
        fn({
          speaking: new Set(state.speaking),
          tracked: new Set(state.tracked),
        });
      } catch (e) {
        console.warn("[LiveKitService] speaking listener failed:", e);
      }
    });
  }

  private sameSpeaking(ids: Set<string>): boolean {
    if (ids.size !== this.speaking.size) return false;
    for (const id of ids) if (!this.speaking.has(id)) return false;
    return true;
  }

  private addAnalyser(identity: string, analyser: SpeechAnalyser): void {
    const list = this.analysers.get(identity) ?? [];
    list.push(analyser);
    this.analysers.set(identity, list);
    this.startSpeakingLoop();
    if (list.length === 1) this.publishSpeaking(new Set(this.speaking));
  }

  private removeAnalyser(identity: string, analyser: SpeechAnalyser): void {
    const list = this.analysers.get(identity);
    if (!list) return;
    try {
      analyser.node.disconnect();
    } catch {}
    const remaining = list.filter((entry) => entry !== analyser);
    if (remaining.length > 0) {
      this.analysers.set(identity, remaining);
      return;
    }
    this.analysers.delete(identity);
    this.holdUntil.delete(identity);
    if (this.analysers.size === 0) this.stopSpeakingLoop();
    const ids = new Set(this.speaking);
    ids.delete(identity);
    this.publishSpeaking(ids);
  }

  private dropAnalysers(identity: string): void {
    const list = this.analysers.get(identity);
    if (!list) return;
    for (const analyser of Array.from(list)) {
      this.removeAnalyser(identity, analyser);
    }
  }

  private startSpeakingLoop(): void {
    if (this.speakingTimer !== null) return;
    this.speakingTimer = setInterval(() => {
      if (this.analysers.size === 0) {
        this.stopSpeakingLoop();
        if (this.speaking.size > 0) this.publishSpeaking(new Set());
        return;
      }
      const now = Date.now();
      const ids = new Set<string>();
      for (const [identity, list] of Array.from(this.analysers.entries())) {
        let loudest = 0;
        for (const analyser of list) {
          analyser.node.getFloatTimeDomainData(analyser.buffer);
          let sum = 0;
          for (let i = 0; i < analyser.buffer.length; i++) {
            sum += analyser.buffer[i] * analyser.buffer[i];
          }
          const rms = Math.sqrt(sum / analyser.buffer.length);
          if (rms > loudest) loudest = rms;
        }
        const state = speakingFor(loudest, now, this.holdUntil.get(identity) ?? 0);
        this.holdUntil.set(identity, state.holdUntil);
        if (state.speaking) ids.add(identity);
      }
      if (!this.sameSpeaking(ids)) this.publishSpeaking(ids);
    }, SPEAKING_INTERVAL_MS);
  }

  private stopSpeakingLoop(): void {
    if (this.speakingTimer === null) return;
    clearInterval(this.speakingTimer);
    this.speakingTimer = null;
  }

  getVolume(identity: string): number {
    return this.volumes.get(identity) ?? 1.0;
  }

  supportsBoost(identity: string): boolean {
    if (this.disabled) return false;
    const gains = this.gainNodes.get(identity);
    if (gains && gains.length > 0) return true;
    if (this.elements.has(identity)) return false;
    if (this.ctx) return true;
    if (typeof window === "undefined") return false;
    return !!(
      window.AudioContext ||
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext
    );
  }

  setVolume(identity: string, volume: number): void {
    const v = Math.max(0, Math.min(RemoteAudioGraph.GAIN_MAX, volume));
    this.volumes.set(identity, v);
    const gain = this.effectiveGain(v);
    const gains = this.gainNodes.get(identity);
    if (gains && gains.length > 0) {
      for (const g of gains) g.gain.value = gain;
    } else {
      const els = this.elements.get(identity);
      if (els) for (const el of els) el.volume = this.elementVolume(v);
    }
  }

  attach(track: AudioTrack, identity: string, onEnded: () => void): void {
    const audioElement = track.attach();
    audioElement.volume = 1.0;
    document.body.appendChild(audioElement);
    const list = this.elements.get(identity) ?? [];
    list.push(audioElement);
    this.elements.set(identity, list);

    const desiredUiVolume = this.volumes.get(identity) ?? 1.0;
    const ctx = this.ensureCtx();
    if (ctx) {
      if (ctx.state === "suspended") {
        ctx.resume().catch(() => {});
      }
      try {
        const source = ctx.createMediaElementSource(audioElement);
        const speechAnalyser = ctx.createAnalyser();
        speechAnalyser.fftSize = 512;
        source.connect(speechAnalyser);
        const analyser: SpeechAnalyser = {
          node: speechAnalyser,
          buffer: new Float32Array(speechAnalyser.fftSize),
        };
        this.addAnalyser(identity, analyser);
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
        this.sourceNodes.set(audioElement, { source, gain, identity, analyser });
        const gains = this.gainNodes.get(identity) ?? [];
        gains.push(gain);
        this.gainNodes.set(identity, gains);
      } catch (e) {
        console.warn(
          "[LiveKitService] failed to wire remote gain node, falling back to element volume:",
          e,
        );
        this.dropAnalysers(identity);
        audioElement.volume = this.elementVolume(desiredUiVolume);
      }
    } else {
      audioElement.volume = this.elementVolume(desiredUiVolume);
    }

    track.on(TrackEvent.Ended, () => {
      const nodes = this.sourceNodes.get(audioElement);
      if (nodes) {
        try {
          nodes.gain.disconnect();
          nodes.source.disconnect();
        } catch {}
        if (nodes.analyser) this.removeAnalyser(identity, nodes.analyser);
        const gains = this.gainNodes.get(identity) ?? [];
        const remainingGains = gains.filter((g) => g !== nodes.gain);
        if (remainingGains.length === 0) {
          this.gainNodes.delete(identity);
        } else {
          this.gainNodes.set(identity, remainingGains);
        }
        this.sourceNodes.delete(audioElement);
      }
      audioElement.remove();
      const remaining = (this.elements.get(identity) ?? []).filter(
        (el) => el !== audioElement,
      );
      if (remaining.length === 0) {
        this.elements.delete(identity);
        this.dropAnalysers(identity);
      } else {
        this.elements.set(identity, remaining);
      }
      onEnded();
    });
  }

  async applySink(sinkId: string): Promise<void> {
    const supportsCtxSink =
      typeof window !== "undefined" &&
      typeof (
        (window.AudioContext?.prototype ?? {}) as { setSinkId?: unknown }
      ).setSinkId === "function";
    if (!supportsCtxSink) {
      this.disableGraph();
      return;
    }
    const ctx = this.ctx as
      | (AudioContext & { setSinkId?: (id: string) => Promise<void> })
      | null;
    if (!ctx?.setSinkId) return;
    try {
      await ctx.setSinkId(sinkId);
    } catch (e) {
      console.warn("[LiveKitService] audio context sink switch failed:", e);
    }
  }

  disableGraph(): void {
    this.disabled = true;
    for (const identity of Array.from(this.analysers.keys())) {
      this.dropAnalysers(identity);
    }
    const room = this.roomProvider();
    if (!room) return;
    room.remoteParticipants.forEach((participant) => {
      this.gainNodes.delete(participant.identity);
      this.elements.delete(participant.identity);
      const ui = this.volumes.get(participant.identity) ?? 1.0;
      const rebuilt: HTMLMediaElement[] = [];
      participant.audioTrackPublications.forEach((publication) => {
        const track = publication.track;
        if (!track) return;
        for (const element of Array.from(track.attachedElements)) {
          const nodes = this.sourceNodes.get(element);
          if (nodes) {
            try {
              nodes.source.disconnect();
              nodes.gain.disconnect();
            } catch {}
            this.sourceNodes.delete(element);
          }
          track.detach(element);
          element.remove();
        }
        const fresh = track.attach();
        fresh.volume = this.elementVolume(ui);
        document.body.appendChild(fresh);
        rebuilt.push(fresh);
      });
      if (rebuilt.length > 0) {
        this.elements.set(participant.identity, rebuilt);
      }
    });
  }

  private ensureCtx(): AudioContext | null {
    if (this.disabled) return null;
    if (this.ctx) return this.ctx;
    try {
      const Ctor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) return null;
      this.ctx = new Ctor();
      if (this.speakerId) {
        void this.applySink(this.speakerId);
      }
      if (this.disabled) return null;
      return this.ctx;
    } catch (e) {
      console.warn("[LiveKitService] AudioContext init failed:", e);
      return null;
    }
  }

  private effectiveGain(uiVolume: number): number {
    const clamped = Math.max(0, Math.min(RemoteAudioGraph.GAIN_MAX, uiVolume));
    return this.deafened ? 0 : clamped * this.masterVolume;
  }

  private elementVolume(uiVolume: number): number {
    return this.deafened ? 0 : Math.min(1, uiVolume * this.masterVolume);
  }

  private applyMasterVolume(): void {
    for (const [identity, gains] of Array.from(this.gainNodes.entries())) {
      const ui = this.volumes.get(identity) ?? 1.0;
      for (const g of gains) g.gain.value = this.effectiveGain(ui);
    }
    for (const [identity, els] of Array.from(this.elements.entries())) {
      if (this.gainNodes.has(identity)) continue;
      const ui = this.volumes.get(identity) ?? 1.0;
      for (const el of els) el.volume = this.elementVolume(ui);
    }
  }
}
