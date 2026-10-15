import { useCallStore } from "../callStore";
import { callAPI } from "../../api/call";

jest.mock("../../api/call", () => ({
  callAPI: { start: jest.fn(), leave: jest.fn() },
}));
jest.mock("../../lib/callSession", () => ({
  awaitVoiceSettingsSyncedFor: jest.fn(),
  connectToRoom: jest.fn(),
  getAuthedIdentity: jest.fn(),
  getAuthedUserId: jest.fn(),
  leaveAsInitiator: jest.fn(),
}));
jest.mock("../../lib/livekit", () => ({
  getLiveKitService: () => ({ disconnect: jest.fn().mockResolvedValue(undefined), getActiveRoomName: () => null }),
}));

const api = callAPI as jest.Mocked<typeof callAPI>;
const session = jest.requireMock("../../lib/callSession");

beforeEach(() => {
  session.awaitVoiceSettingsSyncedFor.mockResolvedValue(true);
  session.connectToRoom.mockResolvedValue(undefined);
  session.getAuthedIdentity.mockResolvedValue({ userId: "me", accessToken: "t" });
  session.getAuthedUserId.mockResolvedValue("me");
  session.leaveAsInitiator.mockResolvedValue(undefined);
  useCallStore.setState({ activeCallId: null, endedCallId: null, connecting: false, callNotice: null, ringing: [] });
  api.leave.mockResolvedValue(undefined);
});

it("applies a decline and the call end that arrived before the start response", async () => {
  api.start.mockImplementation(async () => {
    useCallStore.getState().participantDeclined("c1", "peer", "Peer", true);
    useCallStore.getState().callEnded("c1");
    return { callId: "c1", token: "tok", url: "wss://lk", participants: [] } as never;
  });
  await useCallStore.getState().start(["peer"]);
  const state = useCallStore.getState();
  expect(state.activeCallId).toBeNull();
  expect(state.callNotice).toEqual({ kind: "declined", userId: "peer", name: "Peer" });
  expect(api.leave).toHaveBeenCalledWith("c1");
});

it("ignores early events that belong to another call", async () => {
  api.start.mockImplementation(async () => {
    useCallStore.getState().callEnded("other");
    return { callId: "c2", token: "tok", url: "wss://lk", participants: [] } as never;
  });
  await useCallStore.getState().start(["peer"]);
  expect(useCallStore.getState().activeCallId).toBe("c2");
  expect(useCallStore.getState().callNotice).toBeNull();
});
