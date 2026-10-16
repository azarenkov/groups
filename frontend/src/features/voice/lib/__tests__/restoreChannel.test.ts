import type { Channel } from "../../../../shared/types/channel";
import type { Community } from "../../../../shared/types/community";
import { resolveChannelRestore } from "../restoreChannel";

const community = { id: "k1", name: "Owls" } as Community;
const voice = { id: "v1", kind: "voice", visibility: "public" } as Channel;
const target = { communityId: "k1", channelId: "v1" };

it("waits until the community and its channels are known", () => {
  expect(resolveChannelRestore(target, { list: [], loaded: false }, { list: undefined, loaded: false })).toEqual({ status: "wait" });
  expect(resolveChannelRestore(target, { list: [community], loaded: true }, { list: [], loaded: false })).toEqual({ status: "wait" });
});

it("rejoins a public voice channel that is still there", () => {
  expect(resolveChannelRestore(target, { list: [community], loaded: true }, { list: [voice], loaded: true })).toEqual({
    status: "join",
    community,
    channel: voice,
  });
});

it("gives up when the community or channel is gone or needs a password", () => {
  expect(resolveChannelRestore(target, { list: [], loaded: true }, { list: undefined, loaded: false })).toEqual({ status: "drop" });
  expect(resolveChannelRestore(target, { list: [community], loaded: true }, { list: [], loaded: true })).toEqual({ status: "drop" });
  const locked = { ...voice, visibility: "private" } as Channel;
  expect(resolveChannelRestore(target, { list: [community], loaded: true }, { list: [locked], loaded: true })).toEqual({ status: "drop" });
});
