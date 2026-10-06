import { communityAPI } from "../../api/community";
import { useCommunityStore } from "../communityStore";
import { channelsFullyLoaded, useChannelStore } from "../channelStore";
import { channelAPI } from "../../api/channel";

jest.mock("../../api/community", () => ({ communityAPI: { listCommunities: jest.fn() } }));
jest.mock("../../api/channel", () => ({ channelAPI: { listByCommunity: jest.fn() } }));

const listCommunities = communityAPI.listCommunities as jest.Mock;
const listByCommunity = channelAPI.listByCommunity as jest.Mock;

it("ignores a community load that finishes after signing out", async () => {
  let finish: (v: unknown) => void = () => undefined;
  listCommunities.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const loading = useCommunityStore.getState().fetchCommunities();
  useCommunityStore.getState().clearAll();
  finish({ communities: [{ id: "a-only" }], pagination: { totalPages: 1 } });
  await loading;
  expect(useCommunityStore.getState()).toMatchObject({ communities: [], loaded: false });
});

it("marks communities loaded only after a full load", async () => {
  useCommunityStore.getState().clearAll();
  listCommunities.mockResolvedValueOnce({ communities: [{ id: "c1" }], pagination: { totalPages: 1 } });
  await useCommunityStore.getState().fetchCommunities();
  expect(useCommunityStore.getState()).toMatchObject({ loaded: true });
});

it("forgets every channel list and loaded flag on sign out", async () => {
  listByCommunity.mockResolvedValueOnce({ channels: [{ id: "ch1" }], pagination: { totalPages: 1 } });
  await useChannelStore.getState().fetchChannels("c1");
  expect(channelsFullyLoaded("c1")).toBe(true);
  useChannelStore.getState().clearAll();
  expect(channelsFullyLoaded("c1")).toBe(false);
  expect(useChannelStore.getState().channelsByCommunity).toEqual({});
});
