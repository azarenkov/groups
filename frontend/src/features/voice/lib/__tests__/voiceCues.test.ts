import { SETTLE_MS, idleVoiceCues, nextVoiceCue, type VoiceCueState } from "../voiceCues";

const call = { kind: "call" as const, id: "c1" };
const channel = { kind: "channel" as const, id: "v1" };

const connect = (state: VoiceCueState, now: number, reconnect = false) =>
  nextVoiceCue(state, { type: "connected", room: channel, reconnect }, now);

it("plays the connect cue for the user and the disconnect cue when they leave", () => {
  const joined = connect(idleVoiceCues, 1_000);
  expect(joined.cue).toBe("connected");
  const left = nextVoiceCue(joined.state, { type: "disconnected", room: channel, reconnecting: false }, 5_000);
  expect(left.cue).toBe("disconnected");
  expect(left.state).toEqual(idleVoiceCues);
});

it("stays quiet while reconnecting and for a moment after connecting", () => {
  const dropped = nextVoiceCue(connect(idleVoiceCues, 1_000).state, { type: "disconnected", room: channel, reconnecting: true }, 2_000);
  expect(dropped.cue).toBeNull();
  const back = connect(dropped.state, 3_000, true);
  expect(back.cue).toBeNull();
  const early = nextVoiceCue(back.state, { type: "peerJoined", room: channel, userId: "u2", selfId: "me" }, 3_000 + SETTLE_MS - 1);
  expect(early.cue).toBeNull();
  const later = nextVoiceCue(back.state, { type: "peerJoined", room: channel, userId: "u2", selfId: "me" }, 3_000 + SETTLE_MS);
  expect(later.cue).toBe("peerJoined");
});

it("announces other people only in the room the user is in", () => {
  const state = connect(idleVoiceCues, 1_000).state;
  const now = 1_000 + SETTLE_MS;
  expect(nextVoiceCue(state, { type: "peerLeft", room: channel, userId: "u2", selfId: "me" }, now).cue).toBe("peerLeft");
  expect(nextVoiceCue(state, { type: "peerJoined", room: channel, userId: "me", selfId: "me" }, now).cue).toBeNull();
  expect(nextVoiceCue(state, { type: "peerJoined", room: call, userId: "u2", selfId: "me" }, now).cue).toBeNull();
  expect(nextVoiceCue(idleVoiceCues, { type: "peerJoined", room: channel, userId: "u2", selfId: "me" }, now).cue).toBeNull();
});

it("ignores a disconnect from a room the user already left", () => {
  const state = connect(idleVoiceCues, 1_000).state;
  const other = nextVoiceCue(state, { type: "disconnected", room: call, reconnecting: false }, 2_000);
  expect(other.cue).toBeNull();
  expect(other.state).toBe(state);
});

it("keeps other people quiet while the media link is being re-established", () => {
  const state = connect(idleVoiceCues, 1_000).state;
  const now = 1_000 + SETTLE_MS;
  const suspended = nextVoiceCue(state, { type: "suspended", roomId: "v1" }, now).state;
  expect(nextVoiceCue(suspended, { type: "peerJoined", room: channel, userId: "u2", selfId: "me" }, now).cue).toBeNull();
  const resumed = nextVoiceCue(suspended, { type: "resumed", roomId: "v1" }, now + 100).state;
  expect(nextVoiceCue(resumed, { type: "peerLeft", room: channel, userId: "u2", selfId: "me" }, now + 200).cue).toBeNull();
  expect(nextVoiceCue(resumed, { type: "peerLeft", room: channel, userId: "u2", selfId: "me" }, now + 100 + SETTLE_MS).cue).toBe("peerLeft");
  expect(nextVoiceCue(state, { type: "suspended", roomId: "other" }, now).state).toBe(state);
});
