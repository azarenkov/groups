import { voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { useHiddenDmStore } from "../../../dm/store/hiddenDmStore";

jest.mock("../../../social/store/blockStore", () => {
  const actual = jest.requireActual("../../../social/store/blockStore");
  return { ...actual, isBlockedEitherWay: () => false };
});

beforeEach(() => {
  useCallStore.setState({ incoming: null, activeCallId: null });
  useHiddenDmStore.setState({ hidden: [], loaded: true });
});

it("rings for a call from a visible chat", () => {
  voiceHandlers["call.incoming"]!({ callId: "c1", callerId: "peer", callerName: "Peer" });
  expect(useCallStore.getState().incoming?.callId).toBe("c1");
});

it("does not ring for a call from a hidden chat", () => {
  useHiddenDmStore.setState({ hidden: ["peer"] });
  voiceHandlers["call.incoming"]!({ callId: "c2", callerId: "peer", callerName: "Peer" });
  expect(useCallStore.getState().incoming).toBeNull();
});
