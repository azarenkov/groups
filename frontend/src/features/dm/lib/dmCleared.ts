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
