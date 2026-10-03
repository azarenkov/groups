import { create } from "zustand";

export interface ReportTarget {
  type: "user" | "community" | "stickerPack";
  id: string;
  name: string;
  messageId?: string;
}

interface ReportModalState {
  target: ReportTarget | null;
  opening: number;
  open: (target: ReportTarget) => void;
  close: () => void;
}

export const useReportModalStore = create<ReportModalState>()((set) => ({
  target: null,
  opening: 0,
  open: (target) => set((s) => ({ target, opening: s.opening + 1 })),
  close: () => set({ target: null }),
}));

export const openReport = (target: ReportTarget) =>
  useReportModalStore.getState().open(target);
