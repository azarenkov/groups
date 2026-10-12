import type { WsEnvelope } from "./websocket";

export const GAP_WAIT_MS = 1500;
export const SEEN_WINDOW = 256;

interface PartitionState {
  last: number;
  covered: number;
  seen: Map<number, Set<string>>;
  pending: Map<number, WsEnvelope[]>;
  timer: ReturnType<typeof setTimeout> | null;
}

const sequenceOf = (env: WsEnvelope): { partition: string; seq: number } | null => {
  const { partition, seq } = env as { partition?: unknown; seq?: unknown };
  return typeof partition === "string" && typeof seq === "number" && Number.isFinite(seq) ? { partition, seq } : null;
};

export class Sequencer {
  private partitions = new Map<string, PartitionState>();
  private resyncListeners = new Set<(partition: string) => void>();

  constructor(
    private readonly emit: (env: WsEnvelope) => void,
    private readonly gapWaitMs: number = GAP_WAIT_MS,
  ) {}

  onResync(listener: (partition: string) => void): () => void {
    this.resyncListeners.add(listener);
    return () => {
      this.resyncListeners.delete(listener);
    };
  }

  accept(env: WsEnvelope): void {
    const sequence = sequenceOf(env);
    if (!sequence) {
      this.emit(env);
      return;
    }
    const { partition, seq } = sequence;
    let state = this.partitions.get(partition);
    if (!state) {
      state = this.track(partition, seq);
      this.deliver(state, env, seq);
      return;
    }
    if (seq <= state.covered || seq <= state.last - SEEN_WINDOW || this.wasSeen(state, env, seq)) return;
    if (seq <= state.last) {
      this.deliver(state, env, seq);
      return;
    }
    if (seq === state.last + 1) {
      this.deliver(state, env, seq);
      state.last = seq;
      this.drain(state);
      return;
    }
    const held = state.pending.get(seq) ?? [];
    state.pending.set(seq, [...held, env]);
    if (!state.timer) state.timer = setTimeout(() => this.giveUp(partition), this.gapWaitMs);
  }

  baseline(partition: string, seq: number): void {
    const state = this.partitions.get(partition) ?? this.track(partition, seq);
    state.covered = Math.max(state.covered, seq);
    state.last = Math.max(state.last, seq);
    for (const pendingSeq of Array.from(state.pending.keys())) {
      if (pendingSeq <= state.covered) state.pending.delete(pendingSeq);
    }
    this.drain(state);
  }

  reset(): void {
    for (const state of this.partitions.values()) {
      if (state.timer) clearTimeout(state.timer);
    }
    this.partitions.clear();
  }

  private track(partition: string, seq: number): PartitionState {
    const state: PartitionState = {
      last: seq,
      covered: 0,
      seen: new Map(),
      pending: new Map(),
      timer: null,
    };
    this.partitions.set(partition, state);
    return state;
  }

  private wasSeen(state: PartitionState, env: WsEnvelope, seq: number): boolean {
    return state.seen.get(seq)?.has(env.type) ?? false;
  }

  private deliver(state: PartitionState, env: WsEnvelope, seq: number): void {
    if (this.wasSeen(state, env, seq)) return;
    const types = state.seen.get(seq) ?? new Set<string>();
    types.add(env.type);
    state.seen.set(seq, types);
    this.emit(env);
    this.forgetOld(state);
  }

  private forgetOld(state: PartitionState): void {
    const floor = state.last - SEEN_WINDOW;
    for (const seq of state.seen.keys()) {
      if (seq <= floor) state.seen.delete(seq);
    }
  }

  private drain(state: PartitionState): void {
    for (;;) {
      const next = state.pending.get(state.last + 1);
      if (!next) break;
      state.pending.delete(state.last + 1);
      state.last += 1;
      for (const env of next) this.deliver(state, env, state.last);
    }
    if (state.pending.size === 0 && state.timer) {
      clearTimeout(state.timer);
      state.timer = null;
    }
  }

  private giveUp(partition: string): void {
    const state = this.partitions.get(partition);
    if (!state) return;
    state.timer = null;
    const held = Array.from(state.pending.keys()).sort((a, b) => a - b);
    for (const seq of held) {
      const envs = state.pending.get(seq) ?? [];
      state.pending.delete(seq);
      state.last = Math.max(state.last, seq);
      for (const env of envs) this.deliver(state, env, seq);
    }
    for (const listener of this.resyncListeners) {
      try {
        listener(partition);
      } catch (err) {
        console.warn("[ws] resync listener threw:", err);
      }
    }
  }
}
