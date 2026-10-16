import { DisconnectReason } from "livekit-client";

import { isRecoverableDisconnect } from "../reconnect";

it("tries to return after a dropped connection or a server restart", () => {
  expect(isRecoverableDisconnect(DisconnectReason.SIGNAL_CLOSE)).toBe(true);
  expect(isRecoverableDisconnect(DisconnectReason.UNKNOWN_REASON)).toBe(true);
  expect(isRecoverableDisconnect(DisconnectReason.SERVER_SHUTDOWN)).toBe(true);
});

it("stays out after leaving, being removed or being replaced by another tab", () => {
  expect(isRecoverableDisconnect(DisconnectReason.CLIENT_INITIATED)).toBe(false);
  expect(isRecoverableDisconnect(DisconnectReason.DUPLICATE_IDENTITY)).toBe(false);
  expect(isRecoverableDisconnect(DisconnectReason.PARTICIPANT_REMOVED)).toBe(false);
  expect(isRecoverableDisconnect(DisconnectReason.ROOM_DELETED)).toBe(false);
  expect(isRecoverableDisconnect(DisconnectReason.ROOM_CLOSED)).toBe(false);
});
