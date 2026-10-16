import {
  RECONNECT_WINDOW_MS,
  forgetVoiceSession,
  freshVoiceSession,
  rememberVoiceSession,
  touchVoiceSession,
} from "../sessionHint";

const call = { kind: "call" as const, callId: "c1" };
const channel = { kind: "channel" as const, communityId: "k1", channelId: "v1" };

beforeEach(() => sessionStorage.clear());

it("returns a session left within the reconnect window to the same user", () => {
  rememberVoiceSession("me", call, 1_000);
  touchVoiceSession(5_000);
  expect(freshVoiceSession("me", 5_000 + RECONNECT_WINDOW_MS)).toEqual(call);
  expect(freshVoiceSession("someone-else", 6_000)).toBeNull();
});

it("forgets a session older than the reconnect window", () => {
  rememberVoiceSession("me", channel, 1_000);
  expect(freshVoiceSession("me", 1_001 + RECONNECT_WINDOW_MS)).toBeNull();
  expect(freshVoiceSession("me", 2_000)).toBeNull();
});

it("only forgets the session it was asked about", () => {
  rememberVoiceSession("me", channel, 1_000);
  forgetVoiceSession(call);
  expect(freshVoiceSession("me", 2_000)).toEqual(channel);
  forgetVoiceSession({ ...channel });
  expect(freshVoiceSession("me", 2_000)).toBeNull();
});
