import { installWsBootstrap } from "../websocketBootstrap";
import { ws, type WsEnvelope } from "../websocket";
import { queryClient, queryKeys } from "../../query/queryClient";
import { GAP_WAIT_MS } from "../sequencer";

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

it("marks cached pins stale when a conversation has to be resynced", () => {
  installWsBootstrap();
  jest.useFakeTimers();
  queryClient.setQueryData(queryKeys.pins("dm:peer"), []);
  ws.receive(sequenced(1));
  ws.receive(sequenced(3));
  expect(queryClient.getQueryState(queryKeys.pins("dm:peer"))?.isInvalidated).toBe(false);
  jest.advanceTimersByTime(GAP_WAIT_MS);
  expect(queryClient.getQueryState(queryKeys.pins("dm:peer"))?.isInvalidated).toBe(true);
  queryClient.removeQueries({ queryKey: queryKeys.pins("dm:peer") });
});
