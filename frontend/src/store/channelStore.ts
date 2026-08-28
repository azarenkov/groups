import { create } from "zustand";
import { Channel, ChannelKind, ChannelVisibility } from "../types/channel";
import { channelAPI } from "../api/channel";

interface ChannelState {
  channelsByServer: Record<string, Channel[]>;
  selectedChannelId: string | null;
  isLoading: boolean;
  error: string | null;

  setChannels: (serverId: string, channels: Channel[]) => void;
  setSelectedChannel: (channelId: string | null) => void;
  addChannel: (serverId: string, channel: Channel) => void;
  removeChannel: (serverId: string, channelId: string) => void;
  renameChannel: (serverId: string, channelId: string, name: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  clearChannels: (serverId: string) => void;

  fetchChannels: (serverId: string) => Promise<void>;
  createChannel: (
    serverId: string,
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
  deleteChannel: (serverId: string, channelId: string) => Promise<void>;
}

const removedDuringFetch = new Map<string, Set<string>>();

export const useChannelStore = create<ChannelState>((set, get) => ({
  channelsByServer: {},
  selectedChannelId: null,
  isLoading: false,
  error: null,

  setChannels: (serverId, channels) =>
    set((state) => ({
      channelsByServer: {
        ...state.channelsByServer,
        [serverId]: channels,
      },
    })),

  setSelectedChannel: (channelId) => set({ selectedChannelId: channelId }),

  addChannel: (serverId, channel) =>
    set((state) => {
      const existing = state.channelsByServer[serverId] || [];
      if (existing.some((c) => c.id === channel.id)) return state;
      return {
        channelsByServer: {
          ...state.channelsByServer,
          [serverId]: [...existing, channel],
        },
      };
    }),

  removeChannel: (serverId, channelId) => {
    removedDuringFetch.get(serverId)?.add(channelId);
    set((state) => ({
      channelsByServer: {
        ...state.channelsByServer,
        [serverId]: (state.channelsByServer[serverId] || []).filter(
          (c) => c.id !== channelId,
        ),
      },
    }));
  },

  renameChannel: (serverId, channelId, name) =>
    set((state) => ({
      channelsByServer: {
        ...state.channelsByServer,
        [serverId]: (state.channelsByServer[serverId] || []).map((c) =>
          c.id === channelId ? { ...c, name } : c,
        ),
      },
    })),

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  clearChannels: (serverId) =>
    set((state) => {
      const next = { ...state.channelsByServer };
      delete next[serverId];
      return { channelsByServer: next };
    }),

  fetchChannels: async (serverId: string) => {
    set({ isLoading: true, error: null });
    try {
      const loaded: Channel[] = [];
      const before = new Set((get().channelsByServer[serverId] ?? []).map((c) => c.id));
      const removed = new Set<string>();
      removedDuringFetch.set(serverId, removed);
      const alive = () => loaded.filter((c) => !removed.has(c.id));
      const mergeWithCached = () =>
        set((state) => {
          const fresh = alive();
          const seen = new Set(fresh.map((c) => c.id));
          const cached = (state.channelsByServer[serverId] ?? []).filter((c) => !seen.has(c.id));
          return {
            channelsByServer: {
              ...state.channelsByServer,
              [serverId]: [...fresh, ...cached],
            },
          };
        });
      const first = await channelAPI.listByServer(serverId);
      loaded.push(...first.channels);
      mergeWithCached();
      const totalPages = first.pagination?.totalPages ?? 1;
      for (let p = 2; p <= totalPages; p++) {
        const next = await channelAPI.listByServer(serverId, p);
        const known = new Set(loaded.map((c) => c.id));
        loaded.push(...next.channels.filter((c) => !known.has(c.id)));
        mergeWithCached();
      }
      set((state) => {
        const fresh = alive();
        const seen = new Set(fresh.map((c) => c.id));
        const addedMeanwhile = (state.channelsByServer[serverId] ?? []).filter(
          (c) => !seen.has(c.id) && !before.has(c.id),
        );
        return {
          channelsByServer: { ...state.channelsByServer, [serverId]: [...addedMeanwhile, ...fresh] },
          isLoading: false,
        };
      });
      if (removedDuringFetch.get(serverId) === removed) removedDuringFetch.delete(serverId);
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch channels",
        isLoading: false,
      });
    }
  },

  createChannel: async (
    serverId: string,
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => {
    set({ isLoading: true, error: null });
    try {
      const channel = await channelAPI.create(serverId, {
        name,
        kind,
        visibility,
        password,
      });
      get().addChannel(serverId, channel);
      set({ isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to create channel",
        isLoading: false,
      });
      throw error;
    }
  },

  deleteChannel: async (serverId: string, channelId: string) => {
    set({ isLoading: true, error: null });
    try {
      await channelAPI.delete(serverId, channelId);
      get().removeChannel(serverId, channelId);
      set({ isLoading: false });
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to delete channel",
        isLoading: false,
      });
      throw error;
    }
  },
}));
