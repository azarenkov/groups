import { installWsBootstrap } from "../websocketBootstrap";
import { ws, type WsEnvelope } from "../websocket";
import { queryClient, queryKeys } from "../../query/queryClient";
import { GAP_WAIT_MS } from "../sequencer";
import { useFriendshipStore } from "../../../features/social/store/friendshipStore";
import { useBlockStore } from "../../../features/social/store/blockStore";
import { useHiddenDmStore } from "../../../features/dm/store/hiddenDmStore";

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
