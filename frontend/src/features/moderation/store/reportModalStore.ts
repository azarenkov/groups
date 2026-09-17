import { create } from "zustand";

export interface ReportTarget {
  type: "user" | "server";
  id: string;
  name: string;
  messageId?: string;
}

interface ReportModalState {
  target: ReportTarget | null;
  open: (target: ReportTarget) => void;
  close: () => void;
}

export const useReportModalStore = create<ReportModalState>()((set) => ({
  target: null,
  open: (target) => set({ target }),
  close: () => set({ target: null }),
}));

export const openReport = (target: ReportTarget) =>
  useReportModalStore.getState().open(target);
