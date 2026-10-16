import React from "react";
import { act, render } from "@testing-library/react";

import { useVoiceSession } from "../useVoiceSession";
import { RECONNECT_WINDOW_MS, freshVoiceSession, rememberVoiceSession } from "../../lib/sessionHint";
import type { Channel } from "../../../../shared/types/channel";
import type { Community } from "../../../../shared/types/community";

jest.mock("../../lib/livekit", () => ({ getLiveKitService: () => ({ getLocalSessionId: () => null }) }));
jest.mock("../../store/callStore", () => {
  const { create } = jest.requireActual("zustand");
  const useCallStore = create(() => ({ activeCallId: null, leave: async () => undefined }));
  return { useCallStore };
});

let session: ReturnType<typeof useVoiceSession> | null = null;
const Harness: React.FC = () => {
  session = useVoiceSession();
  return null;
};

const community = { id: "k1" } as Community;
const channel = { id: "v1", kind: "voice", visibility: "public" } as Channel;

beforeEach(() => jest.useFakeTimers());
afterEach(() => jest.useRealTimers());

it("keeps trying to rejoin a channel while reconnecting and gives up after the window", () => {
  render(<Harness />);
  act(() => session!.startVoiceJoin(community, channel, undefined, true));
  const first = session!.voiceJoinAttempt;
  act(() => session!.handleVoiceJoinFailed());
  expect(session!.voiceJoinFailed).toBe(false);
  act(() => jest.advanceTimersByTime(3000));
  expect(session!.voiceJoinAttempt).toBe(first + 1);
  expect(session!.voiceReconnecting).toBe(true);

  act(() => jest.advanceTimersByTime(RECONNECT_WINDOW_MS));
  act(() => session!.handleVoiceJoinFailed());
  expect(session!.voiceJoinFailed).toBe(true);
  expect(session!.voiceReconnecting).toBe(false);
});

it("reports a failed regular join right away", () => {
  render(<Harness />);
  act(() => session!.startVoiceJoin(community, channel));
  act(() => session!.handleVoiceJoinFailed());
  expect(session!.voiceJoinFailed).toBe(true);
});

it("keeps the channel the tab is returning to until the user really leaves it", () => {
  jest.useRealTimers();
  sessionStorage.clear();
  render(<Harness />);
  act(() => session!.startVoiceJoin(community, channel, undefined, true));
  rememberVoiceSession("me", { kind: "channel", communityId: "k1", channelId: "v1" });
  act(() => session!.reconnectVoice());
  expect(freshVoiceSession("me")).toEqual({ kind: "channel", communityId: "k1", channelId: "v1" });
  act(() => session!.handleVoiceLeave());
  expect(freshVoiceSession("me")).toBeNull();
});
