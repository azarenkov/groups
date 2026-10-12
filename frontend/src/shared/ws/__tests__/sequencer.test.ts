import { Sequencer, TYPE_LAG } from "../sequencer";
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

it("takes a loaded snapshot as the baseline and skips events it already covers", () => {
  const { s, emitted, resynced } = setup();
  s.accept(env("a", "p", 1));
  s.accept(env("a", "p", 3));
  s.accept(env("a", "p", 5));
  s.baseline("p", 4);
  expect(emitted).toEqual(["a#1", "a#5"]);
  s.accept(env("a", "p", 2));
  expect(emitted).toEqual(["a#1", "a#5"]);
  jest.advanceTimersByTime(5000);
  expect(resynced).toEqual([]);
});

it("starts tracking a partition from a baseline", () => {
  const { s, emitted } = setup();
  s.baseline("p", 10);
  s.accept(env("a", "p", 10));
  s.accept(env("a", "p", 12));
  expect(emitted).toEqual([]);
  s.accept(env("a", "p", 11));
  expect(emitted).toEqual(["a#11", "a#12"]);
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
