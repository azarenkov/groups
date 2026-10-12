import type { DeliveryGate, WsEnvelope } from "../../../shared/ws/websocket";
import type { ChannelMessage } from "../../../shared/types/channel";
import { useAuthStore } from "../../auth/store/authStore";
import { dmAPI } from "../api/dm";
import { refetchDmPreviews } from "../store/dmPreviewStore";
import { clearedBefore, dmClearedFor } from "./dmCleared";

const watermarks = new Map<string, string>();
const generations = new Map<string, number>();
const clearedAway = new Set<string>();
const checkingLanes = new Map<string, string>();
const LIST_ATTEMPTS = 3;
export const CONFIRM_RETRY_MS = [500, 1500, 4000];
let sessionGen = 0;
let gateGen = 0;

const resyncListeners = new Set<(peerId: string) => void>();

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function confirmExists(peerId: string, messageId: string, alive: () => boolean): Promise<boolean | null> {
  for (let attempt = 0; ; attempt += 1) {
    if (!alive()) return null;
    try {
      await dmAPI.getMessage(peerId, messageId);
      return true;
    } catch (err) {
      if (isGone(err)) return false;
    }
    if (!alive() || attempt >= CONFIRM_RETRY_MS.length) return null;
    await sleep(CONFIRM_RETRY_MS[attempt]);
  }
}

export function onDmResync(listener: (peerId: string) => void): () => void {
  resyncListeners.add(listener);
  return () => {
    resyncListeners.delete(listener);
  };
}

function resyncDm(peerId: string): void {
  refetchDmPreviews(true);
  void import("../../chat/store/unreadStore").then(({ refetchUnread }) => refetchUnread().catch(() => {}));
  for (const listener of resyncListeners) {
    try {
      listener(peerId);
    } catch (err) {
      console.warn("[clearWatermarks] resync listener threw:", err);
    }
  }
}

const sessionAlive = () => {
  const gen = sessionGen;
  return () => gen === sessionGen;
};

const gateAlive = () => {
  const session = sessionAlive();
  const gen = gateGen;
  return () => session() && gen === gateGen;
};

const str = (v: unknown): string | undefined => (typeof v === "string" ? v : undefined);

const isGone = (err: unknown): boolean => {
  const e = err as { status?: number; response?: { status?: number } } | null;
  return e?.status === 404 || e?.response?.status === 404;
};

function dmLane(env: WsEnvelope): string | null {
  const selfId = useAuthStore.getState().user?.id;
  if (!selfId) return null;
  const d = (env.data ?? {}) as Record<string, unknown>;
  if (env.type === "dm_conversation.cleared") {
    const cleared = dmClearedFor(d, selfId);
    return cleared ? `dm:${cleared.peerId}` : null;
  }
  if (env.type.startsWith("dm_message.")) {
    const sender = str(d.senderId);
    const recipient = str(d.recipientId);
    if (sender && recipient) return `dm:${sender === selfId ? recipient : sender}`;
    const id = str(d.id);
    const checking = id ? checkingLanes.get(id) : undefined;
    if (checking) return checking;
    return sender && sender !== selfId ? `dm:${sender}` : null;
  }
  if (env.type === "dm.read") {
    const reader = str(d.readerId);
    const peer = reader === selfId ? str(d.peerId) : reader;
    return peer ? `dm:${peer}` : null;
  }
  if (env.type === "message.pin_changed" && Array.isArray(d.participants)) {
    const peer = d.participants.map(String).find((p) => p !== selfId);
    return peer ? `dm:${peer}` : null;
  }
  if (env.type === "message.hidden" || env.type === "voice.listened") {
    const id = str(d.messageId);
    return id ? (checkingLanes.get(id) ?? null) : null;
  }
  return null;
}

function checkStale(env: WsEnvelope): Promise<boolean> | null {
  const selfId = useAuthStore.getState().user?.id;
  if (!selfId) return null;
  if (env.type === "dm_conversation.cleared") {
    const cleared = dmClearedFor(env.data, selfId);
    if (cleared) {
      generations.set(cleared.peerId, clearGeneration(cleared.peerId) + 1);
      const prev = watermarks.get(cleared.peerId);
      if (!prev || !clearedBefore(cleared.clearedAt, prev)) watermarks.set(cleared.peerId, cleared.clearedAt);
    }
    return null;
  }
  if (env.type !== "dm_message.sent") return null;
  const d = (env.data ?? {}) as Record<string, unknown>;
  const sender = str(d.senderId);
  const recipient = str(d.recipientId);
  const id = str(d.id);
  const at = str(d.occurredAt);
  if (!sender || !recipient || !id || !at) return null;
  const peerId = sender === selfId ? recipient : sender;
  const mark = watermarks.get(peerId);
  if (!mark || !clearedBefore(at, mark)) return null;
  checkingLanes.set(id, `dm:${peerId}`);
  const alive = gateAlive();
  return confirmExists(peerId, id, alive).then((exists) => {
    checkingLanes.delete(id);
    if (exists === null && alive()) resyncDm(peerId);
    return exists === true && useAuthStore.getState().user?.id === selfId;
  });
}

function resetChecks(): void {
  gateGen += 1;
  checkingLanes.clear();
}

export const clearWatermarkGate: DeliveryGate = { lane: dmLane, check: checkStale, reset: resetChecks };

export const clearGeneration = (peerId: string): number => generations.get(peerId) ?? 0;

export const wasClearedAway = (messageId: string): boolean => clearedAway.has(messageId);

export async function loadAcrossClears<T>(peerId: string, load: () => Promise<T>): Promise<T> {
  for (let attempt = 1; ; attempt += 1) {
    const gen = clearGeneration(peerId);
    const result = await load();
    if (clearGeneration(peerId) === gen || attempt >= LIST_ATTEMPTS) return result;
  }
}

export async function sendAcrossClears(peerId: string, send: () => Promise<ChannelMessage>): Promise<ChannelMessage> {
  const gen = clearGeneration(peerId);
  const message = await send();
  if (clearGeneration(peerId) === gen) return message;
  const alive = sessionAlive();
  const exists = await confirmExists(peerId, message.id, alive);
  if (exists === false) clearedAway.add(message.id);
  else if (exists === null && alive()) resyncDm(peerId);
  return message;
}

export function clearWatermarks(): void {
  sessionGen += 1;
  watermarks.clear();
  generations.clear();
  clearedAway.clear();
  checkingLanes.clear();
}
