import { TrackEvent } from "livekit-client";
import type { AudioTrack, Room } from "livekit-client";

export class RemoteAudioGraph {
  static readonly GAIN_MAX = 6.0;

  private volumes: Map<string, number> = new Map();
  private elements: Map<string, HTMLMediaElement[]> = new Map();
  private gainNodes: Map<string, GainNode[]> = new Map();
  private sourceNodes: WeakMap<
    HTMLMediaElement,
    { source: MediaElementAudioSourceNode; gain: GainNode; identity: string }
  > = new WeakMap();
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
  }

  getVolume(identity: string): number {
    return this.volumes.get(identity) ?? 1.0;
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
        this.sourceNodes.set(audioElement, { source, gain, identity });
        const gains = this.gainNodes.get(identity) ?? [];
        gains.push(gain);
        this.gainNodes.set(identity, gains);
      } catch (e) {
        console.warn(
          "[LiveKitService] failed to wire remote gain node, falling back to element volume:",
          e,
        );
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
