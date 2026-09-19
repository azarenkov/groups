import { friendshipAPI } from "../../social/api/friendship";
import { useProfileModalStore } from "../../social/store/profileModalStore";

const knownIds = new Map<string, string>();

const key = (name: string) => name.toLowerCase();

export const rememberMentionTarget = (name: string | null | undefined, userId: string) => {
  if (!name || !userId) return;
  knownIds.set(key(name), userId);
};

export const resolveMentionTarget = async (name: string): Promise<string | null> => {
  const cached = knownIds.get(key(name));
  if (cached) return cached;
  const users = await friendshipAPI.searchUsers(name, 8);
  const exact = users.find((u) => key(u.name) === key(name));
  if (!exact) return null;
  knownIds.set(key(name), exact.id);
  return exact.id;
};

export const openMentionedProfile = async (name: string) => {
  const userId = await resolveMentionTarget(name).catch(() => null);
  if (userId) useProfileModalStore.getState().open(userId);
};

export const clearMentionTargets = () => knownIds.clear();
