export type Tier = "free" | "pro" | "premium";

export const TIER_RANK: Record<Tier, number> = { free: 0, pro: 1, premium: 2 };

export const parseTier = (value: unknown): Tier =>
  value === "pro" || value === "premium" ? value : "free";

export const hasTier = (current: Tier, required: Tier) =>
  TIER_RANK[current] >= TIER_RANK[required];

export const BADGE_STYLES = [
  "tint",
  "solid",
  "neutral",
  "outline",
  "ink",
  "etched",
  "numbered",
] as const;

export type BadgeStyle = (typeof BADGE_STYLES)[number];

export const DEFAULT_BADGE_STYLE: BadgeStyle = "tint";

export const parseBadgeStyle = (value: unknown): BadgeStyle =>
  (BADGE_STYLES as readonly string[]).includes(value as string)
    ? (value as BadgeStyle)
    : DEFAULT_BADGE_STYLE;

export interface StreamQuality {
  id: "720p60" | "1080p60" | "1440p60" | "2160p60";
  label: string;
  width: number;
  height: number;
  frameRate: number;
  maxBitrate: number;
  requires: Tier;
}

export const STREAM_QUALITIES: StreamQuality[] = [
  { id: "720p60", label: "720p 60", width: 1280, height: 720, frameRate: 60, maxBitrate: 4_000_000, requires: "free" },
  { id: "1080p60", label: "1080p 60", width: 1920, height: 1080, frameRate: 60, maxBitrate: 8_000_000, requires: "pro" },
  { id: "1440p60", label: "1440p 60", width: 2560, height: 1440, frameRate: 60, maxBitrate: 12_000_000, requires: "pro" },
  { id: "2160p60", label: "4K 60", width: 3840, height: 2160, frameRate: 60, maxBitrate: 20_000_000, requires: "premium" },
];

export const DEFAULT_STREAM_QUALITY: StreamQuality["id"] = "720p60";

export const streamQualityById = (id: string | null | undefined): StreamQuality =>
  STREAM_QUALITIES.find((q) => q.id === id) ?? STREAM_QUALITIES[0];

export const bestAllowedQuality = (
  wanted: StreamQuality["id"],
  tier: Tier,
): StreamQuality => {
  const target = streamQualityById(wanted);
  if (hasTier(tier, target.requires)) return target;
  const allowed = STREAM_QUALITIES.filter((q) => hasTier(tier, q.requires));
  return allowed[allowed.length - 1] ?? STREAM_QUALITIES[0];
};
