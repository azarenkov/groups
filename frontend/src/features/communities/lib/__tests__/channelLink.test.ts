import { channelServesLink } from "../channelLink";

const link = { kind: "channel" as const, communityId: "c", channelId: "ch" };

it("accepts any channel for a plain channel link", () => {
  expect(channelServesLink({ id: "ch", kind: "voice" }, link)).toBe(true);
  expect(channelServesLink({ id: "other", kind: "text" }, link)).toBe(false);
});

it("accepts only text channels for a message link", () => {
  expect(channelServesLink({ id: "ch", kind: "text" }, { ...link, messageId: "m" })).toBe(true);
  expect(channelServesLink({ id: "ch", kind: "voice" }, { ...link, messageId: "m" })).toBe(false);
});
