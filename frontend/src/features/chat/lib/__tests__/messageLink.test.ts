import { messageLink, parseMessageLink } from "../messageLink";

const origin = "https://rift.example";

describe("message links", () => {
  it("round-trips a channel message", () => {
    const link = messageLink(origin, { kind: "channel", communityId: "c1", channelId: "ch1", messageId: "m1" });
    expect(link).toBe("https://rift.example/app?community=c1&channel=ch1&message=m1");
    expect(parseMessageLink(new URL(link).searchParams, "me")).toEqual({
      kind: "channel",
      communityId: "c1",
      channelId: "ch1",
      messageId: "m1",
    });
  });

  it("opens a dm link on the other participant for both sides", () => {
    const link = messageLink(origin, { kind: "dm", participants: ["bob", "ann"], messageId: "m2" });
    const params = new URL(link).searchParams;
    expect(parseMessageLink(params, "ann")).toEqual({ kind: "dm", peerId: "bob", messageId: "m2" });
    expect(parseMessageLink(params, "bob")).toEqual({ kind: "dm", peerId: "ann", messageId: "m2" });
  });

  it("ignores a dm link for someone outside the conversation", () => {
    const link = messageLink(origin, { kind: "dm", participants: ["bob", "ann"], messageId: "m2" });
    expect(parseMessageLink(new URL(link).searchParams, "eve")).toBeNull();
  });

  it("ignores urls without a message", () => {
    expect(parseMessageLink(new URLSearchParams("join=abc"), "me")).toBeNull();
  });
});
