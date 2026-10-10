import { RoomEvent, Track } from "livekit-client";
import type { Room } from "livekit-client";

import { attachRoomListeners } from "../roomEvents";
import type { RoomEventHandlers } from "../roomEvents";

const fakeRoom = () => {
  const listeners = new Map<string, (...args: unknown[]) => void>();
  const room = { on: (event: string, cb: (...args: unknown[]) => void) => listeners.set(event, cb) };
  return { room: room as unknown as Room, emit: (event: string, ...args: unknown[]) => listeners.get(event)?.(...args) };
};

const handlers = (): RoomEventHandlers =>
  new Proxy({}, { get: (target: Record<string, jest.Mock>, key: string) => (target[key] ??= jest.fn()) }) as unknown as RoomEventHandlers;

const mic = (isMuted: boolean) => ({ source: Track.Source.Microphone, isMuted });

it("reports a remote microphone being muted and unmuted", () => {
  const { room, emit } = fakeRoom();
  const h = handlers();
  attachRoomListeners(room, h);
  emit(RoomEvent.TrackMuted, mic(true), { identity: "bob", isLocal: false });
  expect(h.onParticipantPatch).toHaveBeenLastCalledWith("bob", { isMuted: true });
  emit(RoomEvent.TrackUnmuted, mic(false), { identity: "bob", isLocal: false });
  expect(h.onParticipantPatch).toHaveBeenLastCalledWith("bob", { isMuted: false });
});

it("ignores the local participant and other sources", () => {
  const { room, emit } = fakeRoom();
  const h = handlers();
  attachRoomListeners(room, h);
  emit(RoomEvent.TrackMuted, mic(true), { identity: "me", isLocal: true });
  emit(RoomEvent.TrackMuted, { source: Track.Source.Camera, isMuted: true }, { identity: "bob", isLocal: false });
  expect(h.onParticipantPatch).not.toHaveBeenCalled();
});
