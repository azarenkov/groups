import { create } from "zustand";

interface ProfileModalState {
  userId: string | null;
  open: (userId: string) => void;
  close: () => void;
}

export const useProfileModalStore = create<ProfileModalState>()((set) => ({
  userId: null,
  open: (userId) => set({ userId }),
  close: () => set({ userId: null }),
}));
