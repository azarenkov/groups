import { http } from "../../../shared/api/http";
import { parseBadgeStyle, parseMemberNumber, parseTier, parseValidUntil } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";
import { parseEmojiStatus } from "../lib/emojiStatus";
import type { EmojiStatus } from "../lib/emojiStatus";

export interface MyEntitlements {
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
  validUntil: string | null;
  trial: boolean;
  trialAvailable: boolean;
  emojiStatus: EmojiStatus | null;
}

export interface StartedTrial {
  tier: Tier;
  validUntil: string | null;
}

export interface UserBadge {
  userId: string;
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
  emojiStatus: EmojiStatus | null;
}

class EntitlementsAPI {
  private client = http;

  async getMine(): Promise<MyEntitlements> {
    const response = await this.client.get("/v1/users/me/entitlements");
    const data = response.data.data;
    return {
      tier: parseTier(data.tier),
      badgeStyle: parseBadgeStyle(data.badgeStyle),
      memberNumber: parseMemberNumber(data.memberNumber),
      validUntil: parseValidUntil(data.validUntil),
      trial: data.trial === true,
      trialAvailable: data.trialAvailable === true,
      emojiStatus: parseEmojiStatus(data.emojiStatus),
    };
  }

  async startTrial(): Promise<StartedTrial> {
    const response = await this.client.post("/v1/users/me/trial");
    const data = response.data.data;
    return { tier: parseTier(data.tier), validUntil: parseValidUntil(data.validUntil) };
  }

  async setBadgeStyle(style: BadgeStyle): Promise<BadgeStyle> {
    const response = await this.client.patch("/v1/users/me/badge", { style });
    return parseBadgeStyle(response.data.data.badgeStyle);
  }

  async setEmojiStatus(stickerId: string): Promise<EmojiStatus | null> {
    const response = await this.client.put("/v1/users/me/emoji-status", { stickerId });
    return parseEmojiStatus(response.data.data);
  }

  async clearEmojiStatus(): Promise<void> {
    await this.client.delete("/v1/users/me/emoji-status");
  }

  async lookupBadges(userIds: string[]): Promise<UserBadge[]> {
    const response = await this.client.post("/v1/users/badges", { userIds });
    const items = (response.data.data.badges ?? []) as {
      userId: string;
      tier: string;
      badgeStyle: string;
      memberNumber?: unknown;
      emojiStatus?: unknown;
    }[];
    return items.map((item) => ({
      userId: item.userId,
      tier: parseTier(item.tier),
      badgeStyle: parseBadgeStyle(item.badgeStyle),
      memberNumber: parseMemberNumber(item.memberNumber),
      emojiStatus: parseEmojiStatus(item.emojiStatus),
    }));
  }
}

export const entitlementsAPI = new EntitlementsAPI();
