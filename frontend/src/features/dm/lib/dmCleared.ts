export interface DmCleared {
  peerId: string;
  clearedAt: string;
  scope: "me" | "everyone";
}

export function dmClearedFor(data: unknown, selfId: string | undefined): DmCleared | null {
  if (!selfId || !data || typeof data !== "object") return null;
  const d = data as { participants?: unknown; clearedAt?: unknown; scope?: unknown };
  const participants = Array.isArray(d.participants)
    ? d.participants.filter((p): p is string => typeof p === "string")
    : [];
  if (participants.length !== 2 || !participants.includes(selfId)) return null;
  const peerId = participants.find((p) => p !== selfId);
  if (!peerId || typeof d.clearedAt !== "string") return null;
  return { peerId, clearedAt: d.clearedAt, scope: d.scope === "everyone" ? "everyone" : "me" };
}

const subMillis = (value: string): number => {
  const fraction = /\.(\d+)/.exec(value)?.[1] ?? "";
  return Number(fraction.slice(3, 9).padEnd(6, "0"));
};

const millis = (value: string): number => Date.parse(value.replace(/(\.\d{3})\d+/, "$1"));

export const clearedBefore = (createdAt: string, clearedAt: string): boolean => {
  const created = millis(createdAt);
  const cleared = millis(clearedAt);
  if (!Number.isFinite(created) || !Number.isFinite(cleared)) return false;
  if (created !== cleared) return created < cleared;
  return subMillis(createdAt) <= subMillis(clearedAt);
};

const appliedClears = new Map<string, string>();

export function noteDmCleared(peerId: string, clearedAt: string): void {
  const prev = appliedClears.get(peerId);
  if (!prev || clearedBefore(prev, clearedAt)) appliedClears.set(peerId, clearedAt);
}

export const clearedAway = (peerId: string, createdAt: string): boolean => {
  const mark = appliedClears.get(peerId);
  return !!mark && clearedBefore(createdAt, mark);
};

export function forgetDmClears(): void {
  appliedClears.clear();
}

export function admitDmEvent(env: { type: string; data?: unknown }, selfId: string | undefined): boolean {
  if (!selfId) return true;
  if (env.type === "dm_conversation.cleared") {
    const cleared = dmClearedFor(env.data, selfId);
    if (cleared) noteDmCleared(cleared.peerId, cleared.clearedAt);
    return true;
  }
  if (env.type !== "dm_message.sent") return true;
  const d = (env.data ?? {}) as { senderId?: unknown; recipientId?: unknown; occurredAt?: unknown };
  if (typeof d.senderId !== "string" || typeof d.recipientId !== "string" || typeof d.occurredAt !== "string") return true;
  const peerId = d.senderId === selfId ? d.recipientId : d.senderId;
  return !clearedAway(peerId, d.occurredAt);
}
