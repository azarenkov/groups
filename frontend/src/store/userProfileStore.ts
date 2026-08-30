import { create } from "zustand";

interface ProfileEntry {
  bio: string;
  at: number;
}

interface UserProfileState {
  bios: Record<string, ProfileEntry>;
  apply: (userId: string, bio: string, at: number) => boolean;
  clear: () => void;
}

const toMillis = (iso?: string | null): number => {
  if (!iso) return 0;
  const t = new Date(iso).getTime();
  return Number.isFinite(t) ? t : 0;
};

export const useUserProfileStore = create<UserProfileState>()((set, get) => ({
  bios: {},
  apply: (userId, bio, at) => {
    const current = get().bios[userId];
    if (current && current.at > at) return false;
    if (current && current.at === at && current.bio === bio) return false;
    set((s) => ({ bios: { ...s.bios, [userId]: { bio, at } } }));
    return true;
  },
  clear: () => set({ bios: {} }),
}));

export const profileTimestamp = toMillis;
