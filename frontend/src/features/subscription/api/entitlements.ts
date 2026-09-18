import { http } from "../../../shared/api/http";
import { parseBadgeStyle, parseMemberNumber, parseTier } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";

export interface MyEntitlements {
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
}

export interface UserBadge {
  userId: string;
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
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
    };
  }

  async setBadgeStyle(style: BadgeStyle): Promise<BadgeStyle> {
    const response = await this.client.patch("/v1/users/me/badge", { style });
    return parseBadgeStyle(response.data.data.badgeStyle);
  }

  async lookupBadges(userIds: string[]): Promise<UserBadge[]> {
    const response = await this.client.post("/v1/users/badges", { userIds });
    const items = (response.data.data.badges ?? []) as {
      userId: string;
      tier: string;
      badgeStyle: string;
      memberNumber?: unknown;
    }[];
    return items.map((item) => ({
      userId: item.userId,
      tier: parseTier(item.tier),
      badgeStyle: parseBadgeStyle(item.badgeStyle),
      memberNumber: parseMemberNumber(item.memberNumber),
    }));
  }
}

export const entitlementsAPI = new EntitlementsAPI();
