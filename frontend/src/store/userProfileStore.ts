import { create } from "zustand";

interface ProfileEntry {
  bio: string;
  at: string;
}

interface UserProfileState {
  bios: Record<string, ProfileEntry>;
  apply: (userId: string, bio: string, at: string) => boolean;
  clear: () => void;
}

const pad = (value: string, length: number) => value.padStart(length, "0");

export const profileVersion = (iso?: string | null): string => {
  if (!iso) return "";
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(Z|[+-]\d{2}:\d{2})?$/.exec(iso);
  if (!match) {
    const t = new Date(iso).getTime();
    return Number.isFinite(t) ? pad(String(t), 15) + "000000" : "";
  }
  const base = Date.UTC(
    Number(match[1]),
    Number(match[2]) - 1,
    Number(match[3]),
    Number(match[4]),
    Number(match[5]),
    Number(match[6]),
  );
  let offset = 0;
  if (match[8] && match[8] !== "Z") {
    const sign = match[8].startsWith("-") ? -1 : 1;
    const [h, m] = match[8].slice(1).split(":").map(Number);
    offset = sign * (h * 60 + m) * 60_000;
  }
  const seconds = Math.floor((base - offset) / 1000);
  const nanos = (match[7] ?? "").padEnd(9, "0");
  return pad(String(seconds), 12) + "." + nanos;
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
