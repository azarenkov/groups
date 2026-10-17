import { voiceHandlers } from "../voiceHandlers";
import { reportVoiceCue, resetVoiceCues } from "../../lib/voiceCuePlayer";
import { SETTLE_MS } from "../../lib/voiceCues";
import { playVoiceCue } from "../../../../shared/lib/notificationSound";
import { CONNECTED_CHIME, PEER_JOINED_CHIME, PEER_LEFT_CHIME } from "../../../../shared/lib/chime";
import { useAuthStore } from "../../../auth/store/authStore";
import { useCallStore } from "../../store/callStore";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playVoiceCue: jest.fn(), playNotificationSound: jest.fn() }));
jest.mock("../../../settings/store/notificationSettingsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationSettingsStore");
  return { ...actual, voiceCueAllowed: jest.fn(() => true) };
});

const play = playVoiceCue as jest.Mock;
const allowed = jest.requireMock("../../../settings/store/notificationSettingsStore").voiceCueAllowed as jest.Mock;
const channel = { kind: "channel" as const, id: "v1" };

beforeEach(() => {
  jest.useFakeTimers();
  jest.setSystemTime(10_000);
  play.mockClear();
  allowed.mockReturnValue(true);
  resetVoiceCues();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
  useVoicePresenceStore.getState().clearAll();
  useCallStore.setState({ activeCallId: null });
});

afterEach(() => jest.useRealTimers());

const settle = () => jest.setSystemTime(Date.now() + SETTLE_MS);

it("plays a cue when the user connects and when others come and go afterwards", () => {
  reportVoiceCue({ type: "connected", room: channel, reconnect: false });
  expect(play).toHaveBeenLastCalledWith(CONNECTED_CHIME);
  settle();
  voiceHandlers["channel.participant.joined"]!({ channelId: "v1", userId: "u2", userName: "Bob" });
  expect(play).toHaveBeenLastCalledWith(PEER_JOINED_CHIME);
  voiceHandlers["channel.participant.left"]!({ channelId: "v1", userId: "u2" });
  expect(play).toHaveBeenLastCalledWith(PEER_LEFT_CHIME);
  expect(play).toHaveBeenCalledTimes(3);
});

it("does not announce people who were already there or who arrive while reconnecting", () => {
  useVoicePresenceStore.getState().setMembers("v1", [
    { id: "u2", name: "Bob", isMuted: false, cameraOn: false, screenOn: false },
    { id: "u3", name: "Eve", isMuted: false, cameraOn: false, screenOn: false },
  ]);
  reportVoiceCue({ type: "connected", room: channel, reconnect: true });
  voiceHandlers["channel.participant.joined"]!({ channelId: "v1", userId: "u4", userName: "Kim" });
  voiceHandlers["channel.participant.joined"]!({ channelId: "v1", userId: "me", userName: "Me" });
  expect(play).not.toHaveBeenCalled();
});

it("keeps quiet for calls the user is not in and for other channels", () => {
  reportVoiceCue({ type: "connected", room: channel, reconnect: false });
  settle();
  play.mockClear();
  voiceHandlers["channel.participant.joined"]!({ channelId: "other", userId: "u2", userName: "Bob" });
  voiceHandlers["call.participant.joined"]!({ callId: "c1", userId: "u2", userName: "Bob" });
  expect(play).not.toHaveBeenCalled();
});

it("stays silent when join and leave sounds are switched off", () => {
  allowed.mockReturnValue(false);
  reportVoiceCue({ type: "connected", room: channel, reconnect: false });
  settle();
  voiceHandlers["channel.participant.joined"]!({ channelId: "v1", userId: "u2", userName: "Bob" });
  expect(play).not.toHaveBeenCalled();
});
