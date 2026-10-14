import { useDmPreviewStore } from "../dmPreviewStore";

const preview = (createdAt: string) => ({
  messageId: "m1",
  senderId: "peer",
  content: "secret",
  callStatus: null,
  hasAttachments: false,
  createdAt,
});

beforeEach(() => useDmPreviewStore.getState().clear());

it("removes a preview that the clear covers", () => {
  useDmPreviewStore.getState().push("peer", preview("2026-10-11T09:00:00Z"));
  useDmPreviewStore.getState().dropCleared("peer", "2026-10-11T10:00:00Z");
  expect(useDmPreviewStore.getState().byFriend.peer).toBeUndefined();
});

it("keeps a preview of a message sent after the clear", () => {
  useDmPreviewStore.getState().push("peer", preview("2026-10-11T10:00:05Z"));
  useDmPreviewStore.getState().dropCleared("peer", "2026-10-11T10:00:00Z");
  expect(useDmPreviewStore.getState().byFriend.peer?.content).toBe("secret");
});
