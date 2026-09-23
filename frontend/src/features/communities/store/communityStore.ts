import { create } from "zustand";
import { Community, ListCommunitiesResponse } from "../../../shared/types/community";
import { communityAPI } from "../api/community";

interface CommunityState {
  communities: Community[];
  selectedCommunityId: string | null;
  isLoading: boolean;
  error: string | null;
  pagination: {
    total: number;
    page: number;
    perPage: number;
    totalPages: number;
  } | null;

  setCommunities: (communities: Community[]) => void;
  setSelectedCommunity: (communityId: string | null) => void;
  addCommunity: (community: Community) => void;
  patchCommunity: (
    communityId: string,
    patch: Partial<Pick<Community, "name" | "description" | "ownerId">>,
  ) => void;
  removeCommunity: (communityId: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  setPagination: (pagination: ListCommunitiesResponse["pagination"]) => void;

  fetchCommunities: (page?: number, perPage?: number) => Promise<void>;
  createCommunity: (
    name: string,
    description?: string,
    accentColor?: string,
  ) => Promise<Community>;
  deleteCommunity: (communityId: string) => Promise<void>;
}

let removedDuringFetch = new Set<string>();
let patchedDuringFetch = new Set<string>();

export const useCommunityStore = create<CommunityState>((set, get) => ({
  communities: [],
  selectedCommunityId: null,
  isLoading: false,
  error: null,
  pagination: null,

  setCommunities: (communities) => set({ communities }),

  setSelectedCommunity: (communityId) => set({ selectedCommunityId: communityId }),

  addCommunity: (community) =>
    set((state) =>
      state.communities.some((s) => s.id === community.id)
        ? state
        : { communities: [community, ...state.communities] },
    ),

  patchCommunity: (communityId, patch) => {
    patchedDuringFetch.add(communityId);
    set((state) => ({
      communities: state.communities.map((s) =>
        s.id === communityId ? { ...s, ...patch } : s,
      ),
    }));
  },

  removeCommunity: (communityId) => {
    removedDuringFetch.add(communityId);
    set((state) => ({
      communities: state.communities.filter((s) => s.id !== communityId),
    }));
  },

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  setPagination: (pagination) => set({ pagination }),

  fetchCommunities: async (page = 1, perPage = 50) => {
    set({ isLoading: true, error: null });
    try {
      const loaded: Community[] = [];
      const before = new Set(get().communities.map((s) => s.id));
      removedDuringFetch = new Set();
      patchedDuringFetch = new Set();
      const alive = (current: Community[]) =>
        loaded
          .filter((s) => !removedDuringFetch.has(s.id))
          .map((s) => (patchedDuringFetch.has(s.id) ? (current.find((c) => c.id === s.id) ?? s) : s));
      const mergeWithCached = () =>
        set((state) => {
          const fresh = alive(state.communities);
          const seen = new Set(fresh.map((s) => s.id));
          const cached = state.communities.filter((s) => !seen.has(s.id));
          return { communities: [...fresh, ...cached] };
        });
      const first = await communityAPI.listCommunities(page, perPage);
      loaded.push(...first.communities);
      set({ pagination: first.pagination });
      mergeWithCached();
      const totalPages = first.pagination?.totalPages ?? 1;
      for (let p = page + 1; p <= totalPages; p++) {
        const next = await communityAPI.listCommunities(p, perPage);
        const known = new Set(loaded.map((s) => s.id));
        loaded.push(...next.communities.filter((s) => !known.has(s.id)));
        mergeWithCached();
      }
      set((state) => {
        const fresh = alive(state.communities);
        const seen = new Set(fresh.map((s) => s.id));
        const addedMeanwhile = state.communities.filter((s) => !seen.has(s.id) && !before.has(s.id));
        return { communities: [...addedMeanwhile, ...fresh], isLoading: false };
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch communities",
        isLoading: false,
      });
    }
  },

  createCommunity: async (
    name: string,
    description?: string,
    accentColor?: string,
  ) => {
    set({ isLoading: true, error: null });
    try {
      const community = await communityAPI.createCommunity({
        name,
        description,
        accentColor,
      });
      get().addCommunity(community);
      set({ isLoading: false });
      return community;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to create community",
        isLoading: false,
      });
      throw error;
    }
  },

  deleteCommunity: async (communityId: string) => {
    set({ isLoading: true, error: null });
    try {
      await communityAPI.deleteCommunity(communityId);
      get().removeCommunity(communityId);
      set({ isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to delete community",
        isLoading: false,
      });
      throw error;
    }
  },
}));
