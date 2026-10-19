import { http } from "../../../shared/api/http";


export interface ActiveFriend {
  userId: string;
  userName: string;
  channelId: string;
  channelName: string;
  communityId: string;
  communityName: string;
  joinedAt: string;
  isMuted: boolean;
  cameraOn: boolean;
  screenOn: boolean;
  profileUpdatedAt: string | null;
}

export interface PrivacySettings {
  dmPolicy: "all" | "friends" | "no_one";
  callPolicy: "all" | "friends" | "no_one";
  acceptFriendRequests: boolean;
  showLocalTime: boolean;
}

export interface PublicProfile {
  id: string;
  name: string;
  bio: string;
  profileUpdatedAt: string | null;
  active: boolean;
  timezone: string;
  memberSince: string | null;
  acceptsCalls: boolean;
  acceptsMessages: boolean;
  acceptsFriendRequests: boolean;
  blocked: boolean;
  blockedBy: boolean;
  mutualCommunities: { id: string; name: string; accentColor: string | null }[];
  emojiStatus?: unknown;
  voiceRoom: { channelId: string; channelName: string; communityId: string; communityName: string } | null;
}

export interface RecentRoom {
  channelId: string;
  channelName: string;
  communityId: string;
  communityName: string;
  liveCount: number;
  lastJoin: string;
}

export interface HomeOverview {
  activeFriends: ActiveFriend[];
  recentRooms: RecentRoom[];
}

class HomeAPI {
  private client = http;

  async privacy(): Promise<PrivacySettings> {
    const response = await this.client.get("/v1/users/me/privacy");
    return response.data.data;
  }

  async updatePrivacy(patch: Partial<PrivacySettings>): Promise<PrivacySettings> {
    const response = await this.client.patch("/v1/users/me/privacy", patch);
    return response.data.data;
  }

  async profile(userId: string): Promise<PublicProfile> {
    const response = await this.client.get(`/v1/users/${userId}/profile`);
    return response.data.data;
  }

  async overview(): Promise<HomeOverview> {
    const response = await this.client.get("/v1/users/me/home");
    return response.data.data;
  }
}

export const homeAPI = new HomeAPI();
