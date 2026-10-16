import { appPath, appPathFromDeepLink, isAppPath, legacyMessagePath, messageLinkPath, parseAppPath, sameLocation } from "../appLocation";

const me = "11111111-1111-1111-1111-111111111111";
const peer = "22222222-2222-2222-2222-222222222222";
const community = "33333333-3333-3333-3333-333333333333";
const channel = "44444444-4444-4444-4444-444444444444";
const message = "55555555-5555-5555-5555-555555555555";

describe("app locations", () => {
  it("round-trips every location through its path", () => {
    const locations = [
      { kind: "home" as const },
      { kind: "settings" as const, section: "privacy" as const },
      { kind: "dm" as const, peerId: peer },
      { kind: "dm" as const, peerId: peer, messageId: message },
      { kind: "community" as const, communityId: community },
      { kind: "channel" as const, communityId: community, channelId: channel },
      { kind: "channel" as const, communityId: community, channelId: channel, messageId: message },
    ];
    for (const location of locations) expect(parseAppPath(appPath(location), me)).toEqual(location);
  });

  it("opens a shared dm message link on the other participant for both sides", () => {
    const path = messageLinkPath({ kind: "dm", participants: [peer, me], messageId: message });
    expect(parseAppPath(path, me)).toEqual({ kind: "dm", peerId: peer, messageId: message });
    expect(parseAppPath(path, peer)).toEqual({ kind: "dm", peerId: me, messageId: message });
    expect(parseAppPath(path, "66666666-6666-6666-6666-666666666666")).toBeNull();
  });

  it("rejects malformed paths and the bare root", () => {
    expect(parseAppPath("/app", me)).toBeNull();
    expect(parseAppPath("/app/dm/not-an-id", me)).toBeNull();
    expect(parseAppPath(`/app/dm/${me}`, me)).toBeNull();
    expect(parseAppPath(`/app/c/not-an-id`, me)).toBeNull();
    expect(parseAppPath("/app/settings/unknown", me)).toBeNull();
    expect(parseAppPath(`/elsewhere/dm/${peer}`, me)).toBeNull();
  });

  it("recognises app paths without knowing the viewer, for sign-in redirects", () => {
    expect(isAppPath(messageLinkPath({ kind: "dm", participants: [peer, me], messageId: message }))).toBe(true);
    expect(isAppPath(`/app/c/${community}/${channel}`)).toBe(true);
    expect(isAppPath("/app/admin")).toBe(false);
  });

  it("treats a jump to a message in the same chat as the same location", () => {
    expect(sameLocation({ kind: "dm", peerId: peer }, { kind: "dm", peerId: peer, messageId: message })).toBe(true);
    expect(sameLocation({ kind: "home" }, { kind: "dm", peerId: peer })).toBe(false);
  });

  it("turns message links in the previous query format into app addresses", () => {
    expect(legacyMessagePath(new URLSearchParams(`community=${community}&channel=${channel}&message=${message}`))).toBe(
      `/app/c/${community}/${channel}/${message}`,
    );
    const dmPath = legacyMessagePath(new URLSearchParams(`dm=${me}~${peer}&message=${message}`));
    expect(dmPath && parseAppPath(dmPath, me)).toEqual({ kind: "dm", peerId: peer, messageId: message });
    expect(legacyMessagePath(new URLSearchParams("join=abc"))).toBeNull();
  });
});

describe("desktop deep links", () => {
  it("opens app addresses from rift:// links and ignores everything else", () => {
    expect(appPathFromDeepLink(`rift://app/c/${community}/${channel}/${message}/`)).toBe(`/app/c/${community}/${channel}/${message}`);
    expect(appPathFromDeepLink("rift://app/admin")).toBeNull();
    expect(appPathFromDeepLink(`rift://u/${peer}`)).toBeNull();
  });
});
