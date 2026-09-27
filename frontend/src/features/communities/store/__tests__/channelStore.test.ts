import { useChannelStore } from "../channelStore";
import { channelAPI } from "../../api/channel";

jest.mock("../../api/channel", () => ({
  channelAPI: { listByCommunity: jest.fn() },
}));

const listByCommunity = channelAPI.listByCommunity as jest.Mock;

describe("channelStore.fetchChannels", () => {
  beforeEach(() => {
    useChannelStore.setState({ channelsByCommunity: {}, isLoading: false, error: null });
    listByCommunity.mockReset();
  });

  it("does not bring back channels of a community cleared while the request was in flight", async () => {
    let resolve: (value: unknown) => void = () => {};
    listByCommunity.mockReturnValue(new Promise((r) => (resolve = r)));
    const pending = useChannelStore.getState().fetchChannels("c1");
    useChannelStore.getState().clearChannels("c1");
    resolve({ channels: [{ id: "x", name: "general" }], pagination: { totalPages: 1 } });
    await expect(pending).resolves.toBe(false);
    expect(useChannelStore.getState().channelsByCommunity.c1).toBeUndefined();
  });

  it("reports success once every page is loaded", async () => {
    listByCommunity.mockResolvedValue({ channels: [{ id: "x", name: "general" }], pagination: { totalPages: 1 } });
    await expect(useChannelStore.getState().fetchChannels("c2")).resolves.toBe(true);
    expect(useChannelStore.getState().channelsByCommunity.c2).toHaveLength(1);
  });

  it("reports failure when a later page fails", async () => {
    listByCommunity
      .mockResolvedValueOnce({ channels: [{ id: "x", name: "general" }], pagination: { totalPages: 2 } })
      .mockRejectedValueOnce(new Error("boom"));
    await expect(useChannelStore.getState().fetchChannels("c3")).resolves.toBe(false);
  });
});
