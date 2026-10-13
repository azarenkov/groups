const SOURCE = "messaging";

const hexOf = (id: string): string | null => {
  const hex = id.replace(/-/g, "").toLowerCase();
  return /^[0-9a-f]{32}$/.test(hex) ? hex : null;
};

const formatUuid = (hex: string): string =>
  `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;

const xorHex = (a: string, b: string): string => {
  let out = "";
  for (let i = 0; i < 32; i += 1) out += (parseInt(a[i], 16) ^ parseInt(b[i], 16)).toString(16);
  return out;
};

export function dmPartition(selfId: string, peerId: string): string | null {
  const a = hexOf(selfId);
  const b = hexOf(peerId);
  if (!a || !b) return null;
  return `${SOURCE}:${formatUuid(xorHex(a, b))}`;
}

export function peerOfDmPartition(partition: string, selfId: string): string | null {
  if (!partition.startsWith(`${SOURCE}:`)) return null;
  const key = hexOf(partition.slice(SOURCE.length + 1));
  const self = hexOf(selfId);
  if (!key || !self) return null;
  return formatUuid(xorHex(key, self));
}

export function channelPartition(channelId: string): string | null {
  const hex = hexOf(channelId);
  return hex ? `${SOURCE}:${formatUuid(hex)}` : null;
}
