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
  removeServer: (serverId: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  setPagination: (pagination: ListServersResponse["pagination"]) => void;

  fetchServers: (page?: number, perPage?: number) => Promise<void>;
  createServer: (name: string, description?: string) => Promise<Server>;
  deleteServer: (serverId: string) => Promise<void>;
}

export const useServerStore = create<ServerState>((set, get) => ({
  servers: [],
  selectedServerId: null,
  isLoading: false,
  error: null,
  pagination: null,

  setServers: (servers) => set({ servers }),

  setSelectedServer: (serverId) => set({ selectedServerId: serverId }),

  addServer: (server) =>
    set((state) => ({ servers: [server, ...state.servers] })),

  removeServer: (serverId) =>
    set((state) => ({
      servers: state.servers.filter((s) => s.id !== serverId),
    })),

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  setPagination: (pagination) => set({ pagination }),

  fetchServers: async (page = 1, perPage = 20) => {
    set({ isLoading: true, error: null });
    try {
      const response = await serverAPI.listServers(page, perPage);
      set({
        servers: response.servers,
        pagination: response.pagination,
        isLoading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch servers",
        isLoading: false,
      });
    }
  },

  createServer: async (name: string, description?: string) => {
    set({ isLoading: true, error: null });
    try {
      const server = await serverAPI.createServer({ name, description });
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
