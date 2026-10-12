import { installWsBootstrap } from "../websocketBootstrap";
import { ws, type WsEnvelope } from "../websocket";
import { queryClient, queryKeys } from "../../query/queryClient";
import { GAP_WAIT_MS } from "../sequencer";
import { useFriendshipStore } from "../../../features/social/store/friendshipStore";
import { useBlockStore } from "../../../features/social/store/blockStore";
import { useHiddenDmStore } from "../../../features/dm/store/hiddenDmStore";
import { useAuthStore } from "../../../features/auth/store/authStore";
import { friendshipAPI } from "../../../features/social/api/friendship";
import { dmPartition } from "../../../features/chat/lib/partitions";
import { useDmPreviewStore } from "../../../features/dm/store/dmPreviewStore";

jest.mock("../../../features/dm/store/dmPreviewStore", () => {
  const actual = jest.requireActual("../../../features/dm/store/dmPreviewStore");
  return { ...actual, refetchDmPreviews: jest.fn() };
});

const sequenced = (seq: number) =>
  ({ type: "voice.unknown", data: {}, partition: "messaging:p", seq }) as unknown as WsEnvelope;

afterEach(() => {
  ws.disconnect();
  jest.useRealTimers();
});

it("refreshes pins and pair relations when a conversation has to be resynced", () => {
  installWsBootstrap();
  jest.useFakeTimers();
  const friends = jest.spyOn(useFriendshipStore.getState(), "fetchFriends").mockResolvedValue();
  const blocks = jest.spyOn(useBlockStore.getState(), "fetch").mockResolvedValue();
  const hidden = jest.spyOn(useHiddenDmStore.getState(), "resync").mockImplementation(() => {});
  queryClient.setQueryData(queryKeys.pins("dm:peer"), []);
  ws.receive(sequenced(1));
  ws.receive(sequenced(3));
  expect(queryClient.getQueryState(queryKeys.pins("dm:peer"))?.isInvalidated).toBe(false);
  jest.advanceTimersByTime(GAP_WAIT_MS);
  expect(queryClient.getQueryState(queryKeys.pins("dm:peer"))?.isInvalidated).toBe(true);
  expect(friends).toHaveBeenCalled();
  expect(blocks).toHaveBeenCalled();
  expect(hidden).toHaveBeenCalled();
  friends.mockRestore();
  blocks.mockRestore();
  hidden.mockRestore();
  queryClient.removeQueries({ queryKey: queryKeys.pins("dm:peer") });
});

it("rereads the block relation with the peer of a resynced DM", async () => {
  installWsBootstrap();
  jest.useFakeTimers();
  const self = "0b410a13-a242-46dc-ba1e-cf8692f61af3";
  const peer = "7f2c9d10-1111-4abc-8def-0123456789ab";
  useAuthStore.setState({ user: { id: self, name: "Me" } } as never);
  useDmPreviewStore.setState({ byFriend: { [peer]: { messageId: "m", senderId: peer, content: "", createdAt: "2026-10-12T00:00:00Z" } } } as never);
  const relation = jest.spyOn(friendshipAPI, "blockRelation").mockResolvedValue({ blocked: false, blockedBy: true });
  const partition = dmPartition(self, peer) as string;
  const at = (seq: number) => ({ type: "voice.unknown", data: {}, partition, seq }) as unknown as WsEnvelope;
  ws.receive(at(1));
  ws.receive(at(3));
  jest.advanceTimersByTime(GAP_WAIT_MS);
  expect(relation).toHaveBeenCalledWith(peer);
  jest.useRealTimers();
  await Promise.resolve();
  await Promise.resolve();
  expect(useBlockStore.getState().blockedBy[peer]).toBe(true);
  relation.mockRestore();
  useBlockStore.getState().markBlockedBy(peer, false);
});
