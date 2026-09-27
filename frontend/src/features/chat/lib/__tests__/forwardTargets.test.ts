import {
  MAX_FORWARD_TARGETS,
  buildForwardOptions,
  forwardTargetKey,
  toggleForwardTarget,
} from "../forwardTargets";
import type { Channel } from "../../../../shared/types/channel";
import type { Community } from "../../../../shared/types/community";
import type { Friendship } from "../../../../shared/types/friendship";
import type { ForwardTarget } from "../../api/forward";

const friend = (id: string, name: string, status: Friendship["status"] = "accepted"): Friendship => ({
  friendId: id,
  friendName: name,
  status,
  createdAt: "",
  updatedAt: "",
});

const community = (id: string, name: string): Community => ({
  id,
  name,
  ownerId: "o",
  description: null,
  accentColor: null,
  createdAt: "",
  updatedAt: "",
});

const channel = (id: string, name: string, kind: Channel["kind"] = "text"): Channel => ({
  id,
  name,
  ownerId: "o",
  kind,
  visibility: "public",
  maxParticipants: 0,
  createdAt: "",
});

describe("buildForwardOptions", () => {
  const friends = [friend("f1", "Alice"), friend("f2", "Bob", "pending_incoming")];
  const communities = [community("c1", "Gamers")];
  const channels = { c1: [channel("t1", "general"), channel("v1", "lobby", "voice")] };

  it("lists accepted friends and text channels only", () => {
    const { dms, channels: list } = buildForwardOptions(friends, communities, channels, "");
    expect(dms.map((o) => o.label)).toEqual(["Alice"]);
    expect(list.map((o) => o.label)).toEqual(["#general"]);
    expect(list[0].sublabel).toBe("Gamers");
  });

  it("filters by friend, channel or community name", () => {
    expect(buildForwardOptions(friends, communities, channels, "ali").dms).toHaveLength(1);
    expect(buildForwardOptions(friends, communities, channels, "ali").channels).toHaveLength(0);
    expect(buildForwardOptions(friends, communities, channels, "GAM").channels).toHaveLength(1);
    expect(buildForwardOptions(friends, communities, channels, "gen").channels).toHaveLength(1);
  });
});

describe("toggleForwardTarget", () => {
  const dm: ForwardTarget = { kind: "dm", friendId: "f1" };

  it("adds and removes a target", () => {
    const added = toggleForwardTarget([], dm);
    expect(added).toEqual([dm]);
    expect(toggleForwardTarget(added, { kind: "dm", friendId: "f1" })).toEqual([]);
  });

  it("caps the number of targets", () => {
    const full: ForwardTarget[] = Array.from({ length: MAX_FORWARD_TARGETS }, (_, i) => ({
      kind: "dm",
      friendId: `f${i + 10}`,
    }));
    expect(toggleForwardTarget(full, dm)).toBe(full);
  });

  it("keys channel and dm targets distinctly", () => {
    expect(forwardTargetKey(dm)).not.toBe(
      forwardTargetKey({ kind: "channel", communityId: "f1", channelId: "f1" }),
    );
  });
});
