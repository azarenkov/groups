import { dropHeldCalls, voiceHandlers } from "../voiceHandlers";
import { useCallStore } from "../../store/callStore";
import { useHiddenDmStore } from "../../../dm/store/hiddenDmStore";

jest.mock("../../../social/store/blockStore", () => {
  const actual = jest.requireActual("../../../social/store/blockStore");
  return { ...actual, isBlockedEitherWay: () => false };
});

beforeEach(() => {
  useCallStore.setState({ incoming: null, activeCallId: null });
  useHiddenDmStore.setState({ hidden: [], loaded: true, syncing: false });
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

it("holds a call until hidden chats are known and then rings only for visible chats", () => {
  useHiddenDmStore.setState({ hidden: [], loaded: false });
  voiceHandlers["call.incoming"]!({ callId: "c3", callerId: "peer", callerName: "Peer" });
  voiceHandlers["call.incoming"]!({ callId: "c4", callerId: "friend", callerName: "Friend" });
  expect(useCallStore.getState().incoming).toBeNull();
  useHiddenDmStore.setState({ hidden: ["peer"], loaded: true });
  expect(useCallStore.getState().incoming?.callId).toBe("c4");
});

it("forgets a held call that ends before hidden chats load", () => {
  useHiddenDmStore.setState({ hidden: [], loaded: false });
  voiceHandlers["call.incoming"]!({ callId: "c5", callerId: "peer", callerName: "Peer" });
  voiceHandlers["call.ended"]!({ callId: "c5" });
  useHiddenDmStore.setState({ loaded: true });
  expect(useCallStore.getState().incoming).toBeNull();
});

it("holds a call while hidden chats are being rechecked after a remote change", () => {
  useHiddenDmStore.setState({ hidden: [], loaded: true, syncing: true });
  voiceHandlers["call.incoming"]!({ callId: "c6", callerId: "peer", callerName: "Peer" });
  expect(useCallStore.getState().incoming).toBeNull();
  useHiddenDmStore.setState({ hidden: ["peer"], syncing: false });
  expect(useCallStore.getState().incoming).toBeNull();
});

it("forgets held calls once their socket session is gone", () => {
  useHiddenDmStore.setState({ hidden: [], loaded: false });
  voiceHandlers["call.incoming"]!({ callId: "c7", callerId: "peer", callerName: "Peer" });
  dropHeldCalls();
  useHiddenDmStore.setState({ loaded: true, syncing: false });
  expect(useCallStore.getState().incoming).toBeNull();
});
