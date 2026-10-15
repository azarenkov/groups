import { voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { playNotificationSound } from "../../../../shared/lib/notificationSound";
import { DECLINED_CHIME } from "../../../../shared/lib/chime";
import { useAuthStore } from "../../../auth/store/authStore";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playNotificationSound: jest.fn() }));
jest.mock("../../../settings/store/notificationSettingsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationSettingsStore");
  return { ...actual, callSoundAllowed: () => true };
});

const sound = playNotificationSound as jest.Mock;

beforeEach(() => {
  sound.mockReset();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
  useCallStore.setState({ activeCallId: null, endedCallId: null, ringing: [], callNotice: null });
});

it("tells the caller who declined and plays the refusal sound", () => {
  useCallStore.setState({ activeCallId: "c1", ringing: ["peer"] });
  voiceHandlers["call.declined"]!({ callId: "c1", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().ringing).toEqual([]);
  expect(useCallStore.getState().callNotice).toEqual({ kind: "declined", userId: "peer", name: "Peer" });
  expect(sound).toHaveBeenCalledWith(DECLINED_CHIME);
});

it("still tells the caller when the call already ended before the decline arrived, even after a reload", () => {
  useCallStore.setState({ activeCallId: "c2", ringing: ["peer"] });
  voiceHandlers["call.ended"]!({ callId: "c2" });
  voiceHandlers["call.declined"]!({ callId: "c2", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().activeCallId).toBeNull();
  expect(useCallStore.getState().callNotice?.kind).toBe("declined");
});

it("ignores declines for calls this user is not making", () => {
  useCallStore.setState({ activeCallId: "c3" });
  voiceHandlers["call.declined"]!({ callId: "other", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().callNotice).toBeNull();
  expect(sound).not.toHaveBeenCalled();
});

it("tells only the participant who placed the declined invite", () => {
  useCallStore.setState({ activeCallId: "c4", ringing: ["peer"] });
  voiceHandlers["call.declined"]!({ callId: "c4", userId: "peer", userName: "Peer", inviterId: "someone-else" });
  expect(useCallStore.getState().ringing).toEqual([]);
  expect(useCallStore.getState().callNotice).toBeNull();
  expect(sound).not.toHaveBeenCalled();
  voiceHandlers["call.declined"]!({ callId: "c4", userId: "peer", userName: "Peer", inviterId: "me" });
  expect(useCallStore.getState().callNotice?.kind).toBe("declined");
});

it("ignores a late decline for an earlier call once another call is active", () => {
  useCallStore.setState({ activeCallId: "new", endedCallId: "old", ringing: ["peer"] });
  voiceHandlers["call.declined"]!({ callId: "old", userId: "peer", userName: "Peer", inviterId: "me" });
  expect(useCallStore.getState().ringing).toEqual(["peer"]);
  expect(useCallStore.getState().callNotice).toBeNull();
});
