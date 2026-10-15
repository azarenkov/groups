import { voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { playNotificationSound } from "../../../../shared/lib/notificationSound";
import { DECLINED_CHIME } from "../../../../shared/lib/chime";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playNotificationSound: jest.fn() }));
jest.mock("../../../settings/store/notificationSettingsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationSettingsStore");
  return { ...actual, callSoundAllowed: () => true };
});

const sound = playNotificationSound as jest.Mock;

beforeEach(() => {
  sound.mockReset();
  useCallStore.setState({ activeCallId: null, outgoingCallId: null, ringing: [], callNotice: null });
});

it("tells the caller who declined and plays the refusal sound", () => {
  useCallStore.setState({ activeCallId: "c1", outgoingCallId: "c1", ringing: ["peer"] });
  voiceHandlers["call.declined"]!({ callId: "c1", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().ringing).toEqual([]);
  expect(useCallStore.getState().callNotice).toEqual({ kind: "declined", userId: "peer", name: "Peer" });
  expect(sound).toHaveBeenCalledWith(DECLINED_CHIME);
});

it("still tells the caller when the call already ended before the decline arrived", () => {
  useCallStore.setState({ activeCallId: "c2", outgoingCallId: "c2", ringing: ["peer"] });
  voiceHandlers["call.ended"]!({ callId: "c2" });
  voiceHandlers["call.declined"]!({ callId: "c2", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().activeCallId).toBeNull();
  expect(useCallStore.getState().callNotice?.kind).toBe("declined");
});

it("ignores declines for calls this user is not making", () => {
  useCallStore.setState({ activeCallId: "c3", outgoingCallId: "c3" });
  voiceHandlers["call.declined"]!({ callId: "other", userId: "peer", userName: "Peer" });
  expect(useCallStore.getState().callNotice).toBeNull();
  expect(sound).not.toHaveBeenCalled();
});
