import type { WsEnvelope } from "../../../shared/ws/websocket";
import type { ChannelMessage } from "../../../shared/types/channel";
import { useAuthStore } from "../../auth/store/authStore";
import { dmAPI } from "../api/dm";
import { clearedBefore, dmClearedFor } from "./dmCleared";

const watermarks = new Map<string, string>();
const generations = new Map<string, number>();
const clearedAway = new Set<string>();
const LIST_ATTEMPTS = 3;

const str = (v: unknown): string | undefined => (typeof v === "string" ? v : undefined);

const isGone = (err: unknown): boolean => {
  const e = err as { status?: number; response?: { status?: number } } | null;
  return e?.status === 404 || e?.response?.status === 404;
};

export function clearWatermarkGate(env: WsEnvelope): Promise<boolean> | null {
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
  return dmAPI.getMessage(peerId, id).then(
    () => useAuthStore.getState().user?.id === selfId,
    (err) => !isGone(err) && useAuthStore.getState().user?.id === selfId,
  );
}

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
  try {
    await dmAPI.getMessage(peerId, message.id);
  } catch (err) {
    if (isGone(err)) clearedAway.add(message.id);
  }
  return message;
}

export function clearWatermarks(): void {
  watermarks.clear();
  generations.clear();
  clearedAway.clear();
}
