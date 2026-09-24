import { create } from "zustand";

type DmOpener = (userId: string, name?: string) => void;

interface DmOpenerState {
  open: DmOpener | null;
  register: (open: DmOpener | null) => void;
}

export const useDmOpenerStore = create<DmOpenerState>()((set) => ({
  open: null,
  register: (open) => set({ open }),
}));
