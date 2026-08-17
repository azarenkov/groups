import { create } from "zustand";

interface UserNamesState {
  overrides: Record<string, string>;
  setName: (userId: string, name: string) => void;
  clear: () => void;
}

export const useUserNamesStore = create<UserNamesState>()((set) => ({
  overrides: {},
  setName: (userId, name) =>
    set((state) => ({ overrides: { ...state.overrides, [userId]: name } })),
  clear: () => set({ overrides: {} }),
}));
