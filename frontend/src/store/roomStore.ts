import { create } from "zustand";
import { Room, RoomType } from "../types/room";
import { roomAPI } from "../api/room";

interface RoomState {
  roomsByServer: Record<string, Room[]>;
  selectedRoomId: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  setRooms: (serverId: string, rooms: Room[]) => void;
  setSelectedRoom: (roomId: string | null) => void;
  addRoom: (serverId: string, room: Room) => void;
  removeRoom: (serverId: string, roomId: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  clearRooms: (serverId: string) => void;

  // Async actions
  fetchRooms: (serverId: string) => Promise<void>;
  createRoom: (
    serverId: string,
    name: string,
    roomType: RoomType,
    password?: string,
  ) => Promise<void>;
  deleteRoom: (serverId: string, roomId: string) => Promise<void>;
}

export const useRoomStore = create<RoomState>((set, get) => ({
  roomsByServer: {},
  selectedRoomId: null,
  isLoading: false,
  error: null,

  setRooms: (serverId, rooms) =>
    set((state) => ({
      roomsByServer: {
        ...state.roomsByServer,
        [serverId]: rooms,
      },
    })),

  setSelectedRoom: (roomId) => set({ selectedRoomId: roomId }),

  addRoom: (serverId, room) =>
    set((state) => ({
      roomsByServer: {
        ...state.roomsByServer,
        [serverId]: [...(state.roomsByServer[serverId] || []), room],
      },
    })),

  removeRoom: (serverId, roomId) =>
    set((state) => ({
      roomsByServer: {
        ...state.roomsByServer,
        [serverId]: (state.roomsByServer[serverId] || []).filter(
          (r) => r.id !== roomId,
        ),
      },
    })),

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error }),

  clearError: () => set({ error: null }),

  clearRooms: (serverId) =>
    set((state) => {
      const newRoomsByServer = { ...state.roomsByServer };
      delete newRoomsByServer[serverId];
      return { roomsByServer: newRoomsByServer };
    }),

  fetchRooms: async (serverId: string) => {
    set({ isLoading: true, error: null });
    try {
      const response = await roomAPI.listRoomsByServer(serverId);
      set((state) => ({
        roomsByServer: {
          ...state.roomsByServer,
          [serverId]: response.rooms,
        },
        isLoading: false,
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to fetch rooms",
        isLoading: false,
      });
    }
  },

  createRoom: async (
    serverId: string,
    name: string,
    roomType: RoomType,
    password?: string,
  ) => {
    set({ isLoading: true, error: null });
    try {
      const room = await roomAPI.createRoom({
        serverId,
        name,
        roomType,
        password,
      });
      get().addRoom(serverId, room);
      set({ isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to create room",
        isLoading: false,
      });
      throw error;
    }
  },

  deleteRoom: async (serverId: string, roomId: string) => {
    set({ isLoading: true, error: null });
    try {
      await roomAPI.deleteRoom(roomId);
      get().removeRoom(serverId, roomId);
      set({ isLoading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "Failed to delete room",
        isLoading: false,
      });
      throw error;
    }
  },
}));
