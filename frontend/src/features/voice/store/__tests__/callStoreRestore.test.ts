import { useCallStore } from "../callStore";
import { callAPI } from "../../api/call";
import { useAuthStore } from "../../../auth/store/authStore";
import { RECONNECT_WINDOW_MS, freshVoiceSession, rememberVoiceSession } from "../../lib/sessionHint";
import { loadMediaState } from "../../lib/mediaStateSync";

jest.mock("../../api/call", () => ({
  callAPI: { getActive: jest.fn(), leave: jest.fn() },
}));
jest.mock("../../lib/mediaStateSync", () => ({ loadMediaState: jest.fn() }));
jest.mock("../voiceSettingsStore", () => ({ voiceSettingsReadyFor: () => Promise.resolve() }));
jest.mock("../../lib/callSession", () => ({
  awaitVoiceSettingsSyncedFor: jest.fn(),
  connectToRoom: jest.fn(),
  getAuthedIdentity: jest.fn(),
  getAuthedUserId: jest.fn(),
  leaveAsInitiator: jest.fn(),
}));
jest.mock("../voiceChannelStore", () => ({ leaveVoiceChannel: jest.fn() }));
const mockService = {
  isConnected: jest.fn(),
  isConnectingNow: jest.fn(),
  disconnect: jest.fn(),
  getActiveRoomName: jest.fn(),
  setDeafened: jest.fn(),
  setMicrophoneMuted: jest.fn(),
};
jest.mock("../../lib/livekit", () => ({ getLiveKitService: () => mockService }));

const api = callAPI as jest.Mocked<typeof callAPI>;
const connectToRoom = jest.requireMock("../../lib/callSession").connectToRoom as jest.Mock;
const media = loadMediaState as jest.Mock;

const active = {
  callId: "c1",
  startedAt: "2026-10-16T10:00:00Z",
  token: "tok",
  url: "wss://lk",
  participants: [{ userId: "peer", userName: "Peer" }],
};

beforeEach(() => {
  sessionStorage.clear();
  useAuthStore.setState({ user: { id: "me", name: "me" } as never });
  useCallStore.setState({ activeCallId: null, connecting: false, reconnecting: false, micMuted: false, deafened: false });
  mockService.isConnected.mockReturnValue(false);
  mockService.isConnectingNow.mockReturnValue(false);
  mockService.disconnect.mockResolvedValue(undefined);
  mockService.getActiveRoomName.mockReturnValue(null);
  connectToRoom.mockResolvedValue(undefined);
  api.getActive.mockResolvedValue(active);
  media.mockResolvedValue({ micMuted: true, deafened: false });
});

it("returns to the call the tab was in with the mic and sound kept on the server", async () => {
  rememberVoiceSession("me", { kind: "call", callId: "c1" });
  await useCallStore.getState().rehydrate();
  expect(connectToRoom).toHaveBeenCalledWith("tok", "wss://lk", "c1", true);
  const state = useCallStore.getState();
  expect(state.activeCallId).toBe("c1");
  expect(state.micMuted).toBe(true);
  expect(state.reconnecting).toBe(false);
});

it("brings back deafen after rejoining", async () => {
  media.mockResolvedValue({ micMuted: true, deafened: true });
  rememberVoiceSession("me", { kind: "call", callId: "c1" });
  await useCallStore.getState().rehydrate();
  expect(mockService.setDeafened).toHaveBeenCalledWith(true);
  expect(useCallStore.getState().deafened).toBe(true);
});

it("does nothing in a tab that was not in a call or left it too long ago", async () => {
  await useCallStore.getState().rehydrate();
  rememberVoiceSession("me", { kind: "call", callId: "c1" }, Date.now() - RECONNECT_WINDOW_MS - 1);
  await useCallStore.getState().rehydrate();
  expect(api.getActive).not.toHaveBeenCalled();
  expect(connectToRoom).not.toHaveBeenCalled();
  expect(useCallStore.getState().activeCallId).toBeNull();
});

it("forgets the tab's call once the server says it is over", async () => {
  api.getActive.mockResolvedValue(null);
  rememberVoiceSession("me", { kind: "call", callId: "c1" });
  await useCallStore.getState().rehydrate();
  expect(connectToRoom).not.toHaveBeenCalled();
  expect(freshVoiceSession("me")).toBeNull();
});
