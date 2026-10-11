import { create } from "zustand";

export type DmDetailsTab = "profile" | "shared";
export type ChannelDetailsTab = "channel" | "members" | "shared";

const STORAGE_KEY = "rift.detailsPanel";

interface DetailsPanelState {
  open: boolean;
  dmTab: DmDetailsTab;
  channelTab: ChannelDetailsTab;
  setOpen: (open: boolean) => void;
  toggle: () => void;
  showDm: (tab: DmDetailsTab) => void;
  showChannel: (tab: ChannelDetailsTab) => void;
}

const readOpen = (): boolean => {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === "open";
  } catch {
    return false;
  }
};

const writeOpen = (open: boolean) => {
  try {
    window.localStorage.setItem(STORAGE_KEY, open ? "open" : "closed");
  } catch {
    return;
  }
};

export const useDetailsPanelStore = create<DetailsPanelState>()((set, get) => ({
  open: readOpen(),
  dmTab: "profile",
  channelTab: "members",
  setOpen: (open) => {
    writeOpen(open);
    set({ open });
  },
  toggle: () => get().setOpen(!get().open),
  showDm: (dmTab) => {
    writeOpen(true);
    set({ open: true, dmTab });
  },
  showChannel: (channelTab) => {
    writeOpen(true);
    set({ open: true, channelTab });
  },
}));
