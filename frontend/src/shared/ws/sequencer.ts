import type { WsEnvelope } from "./websocket";

export const GAP_WAIT_MS = 1500;
export const SKIP_TYPE = "sequence.skip";
export const TYPE_LAG = 64;
export const MAX_TRACKED = 4096;
export const SNAPSHOT_TYPES: ReadonlySet<string> = new Set([
  "message.sent",
  "message.edited",
  "message.deleted",
  "message.hidden",
  "dm_message.sent",
  "dm_message.edited",
  "dm_message.deleted",
  "dm_conversation.cleared",
]);

interface PartitionState {
  last: number;
  covered: number;
  firstLive: number;
  lastOverwrite: number;
  contiguous: number;
  seen: Map<number, Set<string>>;
  pending: Map<number, WsEnvelope[]>;
  timer: ReturnType<typeof setTimeout> | null;
}

const SNAPSHOT_OVERWRITES: ReadonlySet<string> = new Set([
  "message.edited",
  "message.deleted",
  "message.hidden",
  "dm_message.edited",
  "dm_message.deleted",
  "dm_conversation.cleared",
  "voice.listened",
]);

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
    if (seq <= state.contiguous - TYPE_LAG || this.wasSeen(state, env, seq)) return;
    if (seq <= state.covered && SNAPSHOT_TYPES.has(env.type)) {
      this.markSeen(state, env, seq);
      return;
    }
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
    const known = this.partitions.get(partition);
    const state = known ?? this.track(partition, seq);
    if (!known) state.contiguous = seq;
    const gapOpen = state.timer !== null;
    const missedBeforeLive = state.covered === 0 && seq < state.firstLive;
    const overtakenByLive = this.changedAfter(state, seq);
    state.covered = Math.max(state.covered, seq);
    state.last = Math.max(state.last, seq);
    const covered = Array.from(state.pending.keys())
      .filter((pendingSeq) => pendingSeq <= state.covered)
      .sort((a, b) => a - b);
    for (const pendingSeq of covered) {
      const envs = state.pending.get(pendingSeq) ?? [];
      state.pending.delete(pendingSeq);
      for (const env of envs) {
        if (SNAPSHOT_TYPES.has(env.type)) this.markSeen(state, env, pendingSeq);
        else this.deliver(state, env, pendingSeq);
      }
    }
    this.drain(state);
    if (missedBeforeLive || overtakenByLive || (gapOpen && state.timer === null)) this.notifyResync(partition);
  }

  checkSnapshot(partition: string, seq: number): void {
    const state = this.partitions.get(partition);
    if (state && this.changedAfter(state, seq)) this.notifyResync(partition);
  }

  position(partition: string): number | undefined {
    return this.partitions.get(partition)?.last;
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
      firstLive: seq,
      lastOverwrite: 0,
      contiguous: seq - 1,
      seen: new Map(),
      pending: new Map(),
      timer: null,
    };
    this.partitions.set(partition, state);
    return state;
  }

  private changedAfter(state: PartitionState, seq: number): boolean {
    return state.lastOverwrite > seq;
  }

  private wasSeen(state: PartitionState, env: WsEnvelope, seq: number): boolean {
    return state.seen.get(seq)?.has(env.type) ?? false;
  }

  private deliver(state: PartitionState, env: WsEnvelope, seq: number): void {
    if (this.wasSeen(state, env, seq)) return;
    this.markSeen(state, env, seq);
    if (env.type !== SKIP_TYPE) this.emit(env);
  }

  private markSeen(state: PartitionState, env: WsEnvelope, seq: number): void {
    if (SNAPSHOT_OVERWRITES.has(env.type)) state.lastOverwrite = Math.max(state.lastOverwrite, seq);
    const types = state.seen.get(seq) ?? new Set<string>();
    types.add(env.type);
    state.seen.set(seq, types);
    this.advance(state);
  }

  private advance(state: PartitionState): void {
    while (state.seen.has(state.contiguous + 1)) state.contiguous += 1;
    if (state.seen.size > MAX_TRACKED) {
      const oldest = Math.min(...Array.from(state.seen.keys()).filter((seq) => seq > state.contiguous));
      if (Number.isFinite(oldest)) state.contiguous = oldest;
      while (state.seen.has(state.contiguous + 1)) state.contiguous += 1;
    }
    const floor = state.contiguous - TYPE_LAG;
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
    this.notifyResync(partition);
  }

  private notifyResync(partition: string): void {
    for (const listener of this.resyncListeners) {
      try {
        listener(partition);
      } catch (err) {
        console.warn("[ws] resync listener threw:", err);
      }
    }
  }
}
