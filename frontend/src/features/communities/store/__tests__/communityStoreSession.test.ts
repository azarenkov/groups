import { communityAPI } from "../../api/community";
import { useCommunityStore } from "../communityStore";
import { channelsFullyLoaded, useChannelStore } from "../channelStore";
import { channelAPI } from "../../api/channel";

jest.mock("../../api/community", () => ({ communityAPI: { listCommunities: jest.fn(), createCommunity: jest.fn() } }));
jest.mock("../../api/channel", () => ({ channelAPI: { listByCommunity: jest.fn(), create: jest.fn() } }));

const listCommunities = communityAPI.listCommunities as jest.Mock;
const listByCommunity = channelAPI.listByCommunity as jest.Mock;
const createCommunity = communityAPI.createCommunity as jest.Mock;
const createChannel = channelAPI.create as jest.Mock;

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

it("keeps a community created before signing out out of the next account", async () => {
  useCommunityStore.getState().clearAll();
  let finish: (v: unknown) => void = () => undefined;
  createCommunity.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const creating = useCommunityStore.getState().createCommunity("A only");
  useCommunityStore.getState().clearAll();
  finish({ id: "a-only", name: "A only" });
  await creating;
  expect(useCommunityStore.getState().communities).toEqual([]);
  expect(useCommunityStore.getState().isLoading).toBe(false);
});

it("keeps a channel created before signing out out of the next account", async () => {
  useChannelStore.getState().clearAll();
  let finish: (v: unknown) => void = () => undefined;
  createChannel.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const creating = useChannelStore.getState().createChannel("c1", "general", "text", "public");
  useChannelStore.getState().clearAll();
  finish({ id: "ch-a", communityId: "c1", name: "general" });
  await creating;
  expect(useChannelStore.getState().channelsByCommunity).toEqual({});
});
