import { parseStreamCeiling } from "../streamCeilingStore";

it("reads the ceiling from a LiveKit attribute or an event payload", () => {
  const ceiling = { width: 1280, height: 720, frameRate: 60, maxBitrate: 4_000_000 };
  expect(parseStreamCeiling(JSON.stringify(ceiling))).toEqual(ceiling);
  expect(parseStreamCeiling({ ...ceiling, roomId: "r" })).toEqual(ceiling);
});

it("ignores a missing or malformed ceiling", () => {
  expect(parseStreamCeiling(undefined)).toBeNull();
  expect(parseStreamCeiling("not json")).toBeNull();
  expect(parseStreamCeiling({ width: 1280, height: 720, frameRate: 60 })).toBeNull();
  expect(parseStreamCeiling({ width: -1, height: 720, frameRate: 60, maxBitrate: 1 })).toBeNull();
});
