import { MAX_TRACKED, Sequencer, TYPE_LAG } from "../sequencer";
import type { WsEnvelope } from "../websocket";

const env = (type: string, partition: string | undefined, seq: number | undefined): WsEnvelope =>
  ({ type, data: {}, partition, seq }) as unknown as WsEnvelope;

const setup = (wait = 1000) => {
  const emitted: string[] = [];
  const resynced: string[] = [];
  const s = new Sequencer((e) => emitted.push(`${e.type}#${(e as { seq?: number }).seq ?? "-"}`), wait);
  s.onResync((p) => resynced.push(p));
  return { s, emitted, resynced };
};

beforeEach(() => jest.useFakeTimers());
afterEach(() => jest.useRealTimers());

it("passes events without a sequence straight through", () => {
  const { s, emitted } = setup();
  s.accept(env("a", undefined, undefined));
  expect(emitted).toEqual(["a#-"]);
});

it("drops an event it has already delivered", () => {
  const { s, emitted } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 1));
  s.accept(env("b", "p", 1));
  expect(emitted).toEqual(["a#1", "b#1"]);
});

it("holds an event after a gap and releases it once the gap fills", () => {
  const { s, emitted, resynced } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  expect(emitted).toEqual(["a#1"]);
  s.accept(env("a", "p", 2));
  expect(emitted).toEqual(["a#1", "a#2", "a#3"]);
  jest.advanceTimersByTime(5000);
  expect(resynced).toEqual([]);
});

it("releases held events and asks for a resync when the gap never fills", () => {
  const { s, emitted, resynced } = setup(1000);
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 4));
  s.accept(env("a", "p", 3));
  jest.advanceTimersByTime(1000);
  expect(emitted).toEqual(["a#1", "a#3", "a#4"]);
  expect(resynced).toEqual(["p"]);
  s.accept(env("a", "p", 5));
  expect(emitted).toEqual(["a#1", "a#3", "a#4", "a#5"]);
});

it("still applies a late event that was never delivered", () => {
  const { s, emitted } = setup(1000);
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  jest.advanceTimersByTime(1000);
  s.accept(env("a", "p", 2));
  expect(emitted).toEqual(["a#1", "a#3", "a#2"]);
});

it("keeps partitions independent", () => {
  const { s, emitted } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  s.accept(env("a", "q", 7));
  expect(emitted).toEqual(["a#1", "a#7"]);
});

it("takes a loaded snapshot as the baseline and skips message events it already covers", () => {
  const { s, emitted, resynced } = setup();
  s.accept(env("message.sent", "p", 1));
  s.accept(env("message.sent", "p", 3));
  s.accept(env("message.sent", "p", 5));
  s.baseline("p", 4);
  expect(emitted).toEqual(["message.sent#1", "message.sent#5"]);
  expect(resynced).toEqual(["p"]);
  s.accept(env("message.edited", "p", 2));
  expect(emitted).toEqual(["message.sent#1", "message.sent#5"]);
  jest.advanceTimersByTime(5000);
  expect(resynced).toEqual(["p"]);
});

it("still delivers covered events a history snapshot does not carry", () => {
  const { s, emitted } = setup();
  s.accept(env("message.sent", "p", 1));
  s.accept(env("user.blocked", "p", 3));
  s.baseline("p", 4);
  expect(emitted).toEqual(["message.sent#1", "user.blocked#3"]);
  s.accept(env("message.pin_changed", "p", 2));
  s.accept(env("message.sent", "p", 4));
  s.accept(env("message.sent", "p", 5));
  expect(emitted).toEqual(["message.sent#1", "user.blocked#3", "message.pin_changed#2", "message.sent#5"]);
});

it("starts tracking a partition from a baseline", () => {
  const { s, emitted } = setup();
  s.baseline("p", 10);
  s.accept(env("message.sent", "p", 10));
  s.accept(env("message.sent", "p", 12));
  expect(emitted).toEqual([]);
  s.accept(env("message.sent", "p", 11));
  expect(emitted).toEqual(["message.sent#11", "message.sent#12"]);
});

it("forgets everything on reset", () => {
  const { s, emitted } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  s.reset();
  jest.advanceTimersByTime(5000);
  s.accept(env("a", "p", 1));
  expect(emitted).toEqual(["a#1", "a#1"]);
});

it("keeps dropping old repeats long after they were delivered", () => {
  const { s, emitted } = setup();
  for (let seq = 1; seq <= TYPE_LAG * 5; seq += 1) s.accept(env("a", "p", seq));
  const delivered = emitted.length;
  s.accept(env("a", "p", 3));
  s.accept(env("b", "p", 40));
  expect(emitted.length).toBe(delivered);
});

it("still applies an event that never arrived even when it is far behind", () => {
  const { s, emitted } = setup(1000);
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  jest.advanceTimersByTime(1000);
  for (let seq = 4; seq <= TYPE_LAG * 5; seq += 1) s.accept(env("a", "p", seq));
  s.accept(env("a", "p", 2));
  expect(emitted[emitted.length - 1]).toBe("a#2");
});

it("fills a gap with a skip marker without passing it on", () => {
  const { s, emitted, resynced } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  s.accept(env("sequence.skip", "p", 2));
  expect(emitted).toEqual(["a#1", "a#3"]);
  jest.advanceTimersByTime(5000);
  expect(resynced).toEqual([]);
});

it("accepts another envelope of a sequence that was just delivered", () => {
  const { s, emitted } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 2));
  s.accept(env("b", "p", 2));
  expect(emitted).toEqual(["a#1", "a#2", "b#2"]);
});

it("does not ask for a resync when a baseline arrives without an open gap", () => {
  const { s, resynced } = setup();
  s.accept(env("a", "p", 1));
  s.baseline("p", 3);
  expect(resynced).toEqual([]);
});

it("bounds tracking when a covered sequence never arrives", () => {
  const { s, emitted } = setup();
  s.accept(env("message.sent", "p", 1));
  s.baseline("p", 3);
  for (let seq = 4; seq <= MAX_TRACKED + 10; seq += 1) s.accept(env("message.sent", "p", seq));
  const delivered = emitted.length;
  s.accept(env("message.sent", "p", 5));
  expect(emitted.length).toBe(delivered);
});

it("asks for a resync when the first snapshot is older than the first live event", () => {
  const { s, emitted, resynced } = setup();
  s.accept(env("message.sent", "p", 13));
  s.baseline("p", 10);
  expect(emitted).toEqual(["message.sent#13"]);
  expect(resynced).toEqual(["p"]);
  s.baseline("p", 13);
  expect(resynced).toEqual(["p"]);
});

it("asks for a resync when the first snapshot is just before the first live event", () => {
  const { s, resynced } = setup();
  s.accept(env("message.deleted", "p", 11));
  s.baseline("p", 10);
  expect(resynced).toEqual(["p"]);
});

it("does not resync when the first snapshot reaches the first live event", () => {
  const { s, resynced } = setup();
  s.accept(env("message.sent", "p", 11));
  s.baseline("p", 11);
  s.baseline("p", 12);
  const fresh = setup();
  fresh.s.baseline("q", 5);
  expect(resynced).toEqual([]);
  expect(fresh.resynced).toEqual([]);
});

it("asks for a resync when a later snapshot misses a live deletion", () => {
  const { s, resynced } = setup();
  s.baseline("p", 9);
  s.accept(env("message.sent", "p", 10));
  s.accept(env("message.deleted", "p", 11));
  s.baseline("p", 10);
  expect(resynced).toEqual(["p"]);
});

it("does not resync when a later snapshot misses only new messages", () => {
  const { s, resynced } = setup();
  s.baseline("p", 9);
  s.accept(env("message.sent", "p", 10));
  s.accept(env("message.sent", "p", 11));
  s.baseline("p", 10);
  s.baseline("p", 11);
  expect(resynced).toEqual([]);
});

it("asks for a resync when a later snapshot misses a live voice listen", () => {
  const { s, resynced } = setup();
  s.baseline("p", 9);
  s.accept(env("voice.listened", "p", 10));
  s.baseline("p", 9);
  expect(resynced).toEqual(["p"]);
});

it("remembers a live deletion long after it left the repeat window", () => {
  const { s, resynced } = setup();
  s.baseline("p", 9);
  s.accept(env("message.deleted", "p", 10));
  for (let seq = 11; seq <= 10 + TYPE_LAG * 3; seq += 1) s.accept(env("message.sent", "p", seq));
  s.baseline("p", 9);
  expect(resynced).toEqual(["p"]);
});

it("asks for a resync when a snapshot it does not adopt missed a live edit", () => {
  const { s, resynced } = setup();
  s.baseline("p", 9);
  s.accept(env("message.edited", "p", 10));
  s.checkSnapshot("p", 10);
  expect(resynced).toEqual([]);
  s.checkSnapshot("p", 9);
  s.checkSnapshot("q", 1);
  expect(resynced).toEqual(["p"]);
});

it("still delivers covered clears and deletions so their side effects run", () => {
  const { s, emitted } = setup();
  s.accept(env("message.sent", "p", 1));
  s.baseline("p", 4);
  s.accept(env("dm_conversation.cleared", "p", 2));
  s.accept(env("message.deleted", "p", 3));
  s.accept(env("message.edited", "p", 4));
  s.accept(env("dm_conversation.cleared", "p", 2));
  expect(emitted).toEqual(["message.sent#1", "dm_conversation.cleared#2", "message.deleted#3"]);
});
