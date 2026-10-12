import { channelPartition, dmPartition } from "../partitions";

const a = "0b410a13-a242-46dc-ba1e-cf8692f61af3";
const b = "7f2c9d10-1111-4abc-8def-0123456789ab";

it("matches the server's DM pair key from either side", () => {
  expect(dmPartition(a, b)).toBe("messaging:746d9703-b353-0c60-37f1-cea5d7919358");
  expect(dmPartition(b, a)).toBe(dmPartition(a, b));
});

it("uses the channel id for a channel", () => {
  expect(channelPartition(a.toUpperCase())).toBe(`messaging:${a}`);
});

it("rejects ids that are not uuids", () => {
  expect(dmPartition("me", b)).toBeNull();
  expect(channelPartition("general")).toBeNull();
});
