import type { Community } from "../../../shared/types/community";
import type { Channel } from "../../../shared/types/channel";
import { channelsFullyLoaded, useChannelStore } from "../store/channelStore";
import type { Friendship, FriendshipStatus } from "../../../shared/types/friendship";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useDmPreviewStore, type DmPreview } from "../../dm/store/dmPreviewStore";
import { useCommunityStore } from "../store/communityStore";
import { useConversationPinsStore } from "../store/conversationPinsStore";
import { useHiddenDmStore } from "../../dm/store/hiddenDmStore";

export const SIDEBAR_SNAPSHOT_KEY = "rift.sidebar.v1";
export const SNAPSHOT_SAVE_DELAY_MS = 1000;
const MAX_FRIENDS = 200;
const MAX_PREVIEWS = 100;
const MAX_COMMUNITIES = 200;
const MAX_CHANNELS = 2000;

export interface SidebarSnapshot {
  v: 1;
  userId: string;
  communities: Community[];
  channels?: Record<string, Channel[]>;
  friends: Friendship[];
  statusById: Record<string, FriendshipStatus>;
  previews: (DmPreview & { peerId: string })[];
  pins: { dms: string[]; communities: string[] };
  hiddenDms?: string[];
}

const storage = (): Storage | null => {
  try {
    return typeof window !== "undefined" ? window.localStorage : null;
  } catch {
    return null;
  }
};

export function readSidebarSnapshot(): SidebarSnapshot | null {
  try {
    const raw = storage()?.getItem(SIDEBAR_SNAPSHOT_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as SidebarSnapshot;
    if (parsed?.v !== 1 || typeof parsed.userId !== "string") return null;
    if (!Array.isArray(parsed.communities) || !Array.isArray(parsed.friends) || !Array.isArray(parsed.previews)) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function clearSidebarSnapshot(): void {
  try {
    storage()?.removeItem(SIDEBAR_SNAPSHOT_KEY);
  } catch {
    return;
  }
}

export function hydrateSidebar(expectedUserId?: string | null): boolean {
  const snapshot = readSidebarSnapshot();
  if (!snapshot) return false;
  if (expectedUserId && snapshot.userId !== expectedUserId) {
    clearSidebarSnapshot();
    return false;
  }
  const communities = useCommunityStore.getState();
  if (communities.communities.length === 0 && !communities.isLoading && !communities.pagination) {
    communities.setCommunities(snapshot.communities);
  }
  const friendships = useFriendshipStore.getState();
  if (!friendships.loaded && !friendships.isLoading && friendships.friends.length === 0) {
    useFriendshipStore.setState({ friends: snapshot.friends, statusById: snapshot.statusById ?? {} });
  }
  if (snapshot.channels && typeof snapshot.channels === "object") {
    useChannelStore.getState().hydrate(snapshot.channels);
  }
  useDmPreviewStore.getState().hydrate(snapshot.previews);
  if (snapshot.pins) useConversationPinsStore.getState().hydrate(snapshot.pins);
  if (Array.isArray(snapshot.hiddenDms)) useHiddenDmStore.getState().hydrate(snapshot.hiddenDms);
  return true;
}

export function buildSidebarSnapshot(userId: string): SidebarSnapshot {
  const friendships = useFriendshipStore.getState();
  const previews = Object.entries(useDmPreviewStore.getState().byFriend)
    .map(([peerId, preview]) => ({ ...preview, peerId }))
    .sort((a, b) => (a.createdAt < b.createdAt ? 1 : a.createdAt > b.createdAt ? -1 : 0))
    .slice(0, MAX_PREVIEWS);
  const pins = useConversationPinsStore.getState();
  const hidden = useHiddenDmStore.getState();
  const communities = useCommunityStore.getState().communities.slice(0, MAX_COMMUNITIES);
  const byCommunity = useChannelStore.getState().channelsByCommunity;
  const channels: Record<string, Channel[]> = {};
  let channelCount = 0;
  for (const community of communities) {
    const list = byCommunity[community.id];
    if (!list || !channelsFullyLoaded(community.id) || channelCount + list.length > MAX_CHANNELS) continue;
    channels[community.id] = list;
    channelCount += list.length;
  }
  return {
    v: 1,
    userId,
    communities,
    channels,
    friends: friendships.friends.filter((f) => f.status === "accepted").slice(0, MAX_FRIENDS),
    statusById: Object.fromEntries(
      Object.entries(friendships.statusById).filter(([, status]) => status === "accepted"),
    ),
    previews,
    pins: { dms: pins.dms, communities: pins.communities },
    hiddenDms: hidden.loaded ? hidden.hidden : readSidebarSnapshot()?.hiddenDms,
  };
}

export function startSidebarSnapshots(userId: string): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const schedule = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      const friendsReady = useFriendshipStore.getState().loaded;
      const previewsReady = useDmPreviewStore.getState().loaded;
      const communitiesReady = useCommunityStore.getState().loaded;
      if (!friendsReady || !previewsReady || !communitiesReady) return;
      try {
        storage()?.setItem(SIDEBAR_SNAPSHOT_KEY, JSON.stringify(buildSidebarSnapshot(userId)));
      } catch {
        return;
      }
    }, SNAPSHOT_SAVE_DELAY_MS);
  };
  const unsubscribers = [
    useCommunityStore.subscribe(schedule),
    useChannelStore.subscribe(schedule),
    useFriendshipStore.subscribe(schedule),
    useDmPreviewStore.subscribe(schedule),
    useConversationPinsStore.subscribe(schedule),
    useHiddenDmStore.subscribe(schedule),
  ];
  schedule();
  return () => {
    if (timer) clearTimeout(timer);
    unsubscribers.forEach((off) => off());
  };
}
