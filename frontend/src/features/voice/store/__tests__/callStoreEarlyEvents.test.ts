import { useCallStore } from "../callStore";
import { callAPI } from "../../api/call";
import { playNotificationSound } from "../../../../shared/lib/notificationSound";
import { DECLINED_CHIME } from "../../../../shared/lib/chime";
import "../../ws/voiceHandlers";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playNotificationSound: jest.fn() }));
jest.mock("../../../settings/store/notificationSettingsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationSettingsStore");
  return { ...actual, callSoundAllowed: () => true };
});
jest.mock("../../api/call", () => ({
  callAPI: { start: jest.fn(), leave: jest.fn(), accept: jest.fn(), join: jest.fn() },
}));
jest.mock("../../lib/callSession", () => ({
  awaitVoiceSettingsSyncedFor: jest.fn(),
  connectToRoom: jest.fn(),
  getAuthedIdentity: jest.fn(),
  getAuthedUserId: jest.fn(),
  leaveAsInitiator: jest.fn(),
}));
jest.mock("../voiceChannelStore", () => ({ leaveVoiceChannel: jest.fn().mockResolvedValue(true) }));
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
    useCallStore.getState().participantDeclined("c1", "peer", "Peer", { inviterId: undefined, selfId: "me" });
    useCallStore.getState().callEnded("c1");
    return { callId: "c1", token: "tok", url: "wss://lk", participants: [] } as never;
  });
  await useCallStore.getState().start(["peer"]);
  const state = useCallStore.getState();
  expect(state.activeCallId).toBeNull();
  expect(state.callNotice).toEqual({ kind: "declined", userId: "peer", name: "Peer" });
  expect(api.leave).toHaveBeenCalledWith("c1");
  expect(playNotificationSound).toHaveBeenCalledWith(DECLINED_CHIME);
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

it("decides who placed a buffered legacy decline once ringing is known", async () => {
  api.start.mockImplementation(async () => {
    useCallStore.getState().participantDeclined("c3", "peer", "Peer", { inviterId: undefined, selfId: "me" });
    return { callId: "c3", token: "tok", url: "wss://lk", participants: [] } as never;
  });
  await useCallStore.getState().start(["peer"]);
  expect(useCallStore.getState().callNotice).toEqual({ kind: "declined", userId: "peer", name: "Peer" });
});

it("clears call notices when the session resets", () => {
  useCallStore.setState({ callNotice: { kind: "missed", userId: "peer", name: "Peer" }, callError: "x", endedCallId: "c9" });
  useCallStore.getState().resetNotices();
  expect(useCallStore.getState()).toMatchObject({ callNotice: null, callError: null, endedCallId: null });
});

it("applies a call end that arrived while accepting the call", async () => {
  useCallStore.setState({ incoming: { callId: "c4", callerId: "peer", callerName: "Peer", video: false } as never });
  api.accept.mockImplementation(async () => {
    useCallStore.getState().callEnded("c4");
    return { token: "tok", url: "wss://lk", participants: [] } as never;
  });
  await useCallStore.getState().accept();
  expect(useCallStore.getState().activeCallId).toBeNull();
});

it("forgets the previous ended call once another call becomes active", async () => {
  useCallStore.setState({ endedCallId: "old" });
  api.start.mockResolvedValue({ callId: "c5", token: "tok", url: "wss://lk", participants: [] } as never);
  await useCallStore.getState().start(["peer"]);
  expect(useCallStore.getState().endedCallId).toBeNull();
});
