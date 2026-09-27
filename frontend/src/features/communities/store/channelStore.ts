import { create } from "zustand";
import { Channel, ChannelKind, ChannelVisibility } from "../../../shared/types/channel";
import { channelAPI } from "../api/channel";

interface ChannelState {
  channelsByCommunity: Record<string, Channel[]>;
  selectedChannelId: string | null;
  isLoading: boolean;
  error: string | null;

  setChannels: (communityId: string, channels: Channel[]) => void;
  setSelectedChannel: (channelId: string | null) => void;
  addChannel: (communityId: string, channel: Channel) => void;
  removeChannel: (communityId: string, channelId: string) => void;
  renameChannel: (communityId: string, channelId: string, name: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  clearChannels: (communityId: string) => void;

  fetchChannels: (communityId: string) => Promise<boolean>;
  createChannel: (
    communityId: string,
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
  deleteChannel: (communityId: string, channelId: string) => Promise<void>;
}

const removedDuringFetch = new Map<string, Set<string>>();
const patchedDuringFetch = new Map<string, Set<string>>();

export const useChannelStore = create<ChannelState>((set, get) => ({
  channelsByCommunity: {},
  selectedChannelId: null,
  isLoading: false,
  error: null,

  setChannels: (communityId, channels) =>
    set((state) => ({
      channelsByCommunity: {
        ...state.channelsByCommunity,
        [communityId]: channels,
      },
    })),

  setSelectedChannel: (channelId) => set({ selectedChannelId: channelId }),

  addChannel: (communityId, channel) =>
    set((state) => {
      const existing = state.channelsByCommunity[communityId] || [];
      if (existing.some((c) => c.id === channel.id)) return state;
      return {
        channelsByCommunity: {
          ...state.channelsByCommunity,
          [communityId]: [...existing, channel],
        },
      };
    }),

  removeChannel: (communityId, channelId) => {
    removedDuringFetch.get(communityId)?.add(channelId);
    set((state) => ({
      channelsByCommunity: {
        ...state.channelsByCommunity,
        [communityId]: (state.channelsByCommunity[communityId] || []).filter(
          (c) => c.id !== channelId,
        ),
      },
    }));
  },

  renameChannel: (communityId, channelId, name) => {
    patchedDuringFetch.get(communityId)?.add(channelId);
    set((state) => ({
      channelsByCommunity: {
        ...state.channelsByCommunity,
        [communityId]: (state.channelsByCommunity[communityId] || []).map((c) =>
          c.id === channelId ? { ...c, name } : c,
        ),
      },
    }));
  },

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  clearChannels: (communityId) =>
    set((state) => {
      const next = { ...state.channelsByCommunity };
      delete next[communityId];
      return { channelsByCommunity: next };
    }),

  fetchChannels: async (communityId: string) => {
    set({ isLoading: true, error: null });
    try {
      const loaded: Channel[] = [];
      const before = new Set((get().channelsByCommunity[communityId] ?? []).map((c) => c.id));
      const removed = new Set<string>();
      const patched = new Set<string>();
      removedDuringFetch.set(communityId, removed);
      patchedDuringFetch.set(communityId, patched);
      const alive = (current: Channel[]) =>
        loaded
          .filter((c) => !removed.has(c.id))
          .map((c) => (patched.has(c.id) ? (current.find((x) => x.id === c.id) ?? c) : c));
      const mergeWithCached = () =>
        set((state) => {
          const fresh = alive(state.channelsByCommunity[communityId] ?? []);
          const seen = new Set(fresh.map((c) => c.id));
          const cached = (state.channelsByCommunity[communityId] ?? []).filter((c) => !seen.has(c.id));
          return {
            channelsByCommunity: {
              ...state.channelsByCommunity,
              [communityId]: [...fresh, ...cached],
            },
          };
        });
      const first = await channelAPI.listByCommunity(communityId);
      loaded.push(...first.channels);
      mergeWithCached();
      const totalPages = first.pagination?.totalPages ?? 1;
      for (let p = 2; p <= totalPages; p++) {
        const next = await channelAPI.listByCommunity(communityId, p);
        const known = new Set(loaded.map((c) => c.id));
        loaded.push(...next.channels.filter((c) => !known.has(c.id)));
        mergeWithCached();
      }
      set((state) => {
        const fresh = alive(state.channelsByCommunity[communityId] ?? []);
        const seen = new Set(fresh.map((c) => c.id));
        const addedMeanwhile = (state.channelsByCommunity[communityId] ?? []).filter(
          (c) => !seen.has(c.id) && !before.has(c.id),
        );
        return {
          channelsByCommunity: { ...state.channelsByCommunity, [communityId]: [...addedMeanwhile, ...fresh] },
          isLoading: false,
        };
      });
      if (removedDuringFetch.get(communityId) === removed) removedDuringFetch.delete(communityId);
      if (patchedDuringFetch.get(communityId) === patched) patchedDuringFetch.delete(communityId);
      return true;
    } catch (error) {
      set({
        error:
          error instanceof Error ? error.message : "Failed to fetch channels",
        isLoading: false,
      });
      return false;
    }
  },

  createChannel: async (
    communityId: string,
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => {
    set({ isLoading: true, error: null });
    try {
      const channel = await channelAPI.create(communityId, {
        name,
        kind,
        visibility,
        password,
      });
      get().addChannel(communityId, channel);
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

  deleteChannel: async (communityId: string, channelId: string) => {
    set({ isLoading: true, error: null });
    try {
      await channelAPI.delete(communityId, channelId);
      get().removeChannel(communityId, channelId);
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
