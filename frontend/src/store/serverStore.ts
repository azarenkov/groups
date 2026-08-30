import { create } from "zustand";
import { Server, ListServersResponse } from "../types/server";
import { serverAPI } from "../api/server";

interface ServerState {
  servers: Server[];
  selectedServerId: string | null;
  isLoading: boolean;
  error: string | null;
  pagination: {
    total: number;
    page: number;
    perPage: number;
    totalPages: number;
  } | null;

  setServers: (servers: Server[]) => void;
  setSelectedServer: (serverId: string | null) => void;
  addServer: (server: Server) => void;
  patchServer: (
    serverId: string,
    patch: Partial<Pick<Server, "name" | "description" | "ownerId">>,
  ) => void;
  removeServer: (serverId: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  setPagination: (pagination: ListServersResponse["pagination"]) => void;

  fetchServers: (page?: number, perPage?: number) => Promise<void>;
  createServer: (
    name: string,
    description?: string,
    accentColor?: string,
  ) => Promise<Server>;
  deleteServer: (serverId: string) => Promise<void>;
}

let removedDuringFetch = new Set<string>();
let patchedDuringFetch = new Set<string>();

export const useServerStore = create<ServerState>((set, get) => ({
  servers: [],
  selectedServerId: null,
  isLoading: false,
  error: null,
  pagination: null,

  setServers: (servers) => set({ servers }),

  setSelectedServer: (serverId) => set({ selectedServerId: serverId }),

  addServer: (server) =>
    set((state) =>
      state.servers.some((s) => s.id === server.id)
        ? state
        : { servers: [server, ...state.servers] },
    ),

  patchServer: (serverId, patch) => {
    patchedDuringFetch.add(serverId);
    set((state) => ({
      servers: state.servers.map((s) =>
        s.id === serverId ? { ...s, ...patch } : s,
      ),
    }));
  },

  removeServer: (serverId) => {
    removedDuringFetch.add(serverId);
    set((state) => ({
      servers: state.servers.filter((s) => s.id !== serverId),
    }));
  },

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  setPagination: (pagination) => set({ pagination }),

  fetchServers: async (page = 1, perPage = 50) => {
    set({ isLoading: true, error: null });
    try {
      const loaded: Server[] = [];
      const before = new Set(get().servers.map((s) => s.id));
      removedDuringFetch = new Set();
      patchedDuringFetch = new Set();
      const alive = (current: Server[]) =>
        loaded
          .filter((s) => !removedDuringFetch.has(s.id))
          .map((s) => (patchedDuringFetch.has(s.id) ? (current.find((c) => c.id === s.id) ?? s) : s));
      const mergeWithCached = () =>
        set((state) => {
          const fresh = alive(state.servers);
          const seen = new Set(fresh.map((s) => s.id));
          const cached = state.servers.filter((s) => !seen.has(s.id));
          return { servers: [...fresh, ...cached] };
        });
      const first = await serverAPI.listServers(page, perPage);
      loaded.push(...first.servers);
      set({ pagination: first.pagination });
      mergeWithCached();
      const totalPages = first.pagination?.totalPages ?? 1;
      for (let p = page + 1; p <= totalPages; p++) {
        const next = await serverAPI.listServers(p, perPage);
        const known = new Set(loaded.map((s) => s.id));
        loaded.push(...next.servers.filter((s) => !known.has(s.id)));
        mergeWithCached();
      }
      set((state) => {
        const fresh = alive(state.servers);
        const seen = new Set(fresh.map((s) => s.id));
        const addedMeanwhile = state.servers.filter((s) => !seen.has(s.id) && !before.has(s.id));
        return { servers: [...addedMeanwhile, ...fresh], isLoading: false };
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch servers",
        isLoading: false,
      });
    }
  },

  createServer: async (
    name: string,
    description?: string,
    accentColor?: string,
  ) => {
    set({ isLoading: true, error: null });
    try {
      const server = await serverAPI.createServer({
        name,
        description,
        accentColor,
      });
      get().addServer(server);
      set({ isLoading: false });
      return server;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to create server",
        isLoading: false,
      });
      throw error;
    }
  },

  deleteServer: async (serverId: string) => {
    set({ isLoading: true, error: null });
    try {
      await serverAPI.deleteServer(serverId);
      get().removeServer(serverId);
      set({ isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to delete server",
        isLoading: false,
      });
      throw error;
    }
  },
}));
