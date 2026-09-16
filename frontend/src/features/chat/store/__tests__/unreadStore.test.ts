import { useUnreadStore } from "../unreadStore";

describe("unread store", () => {
  beforeEach(() => {
    useUnreadStore.getState().clearAll();
  });

  it("counts new messages per channel and per dm", () => {
    const store = useUnreadStore.getState();
    store.bumpChannel("c1", "s1");
    store.bumpChannel("c1", "s1");
    store.bumpDm("f1");
    expect(useUnreadStore.getState().byChannel.c1).toBe(2);
    expect(useUnreadStore.getState().byDm.f1).toBe(1);
  });

  it("sums a server from the channels that belong to it", () => {
    const store = useUnreadStore.getState();
    store.setAll({ c1: 2, c2: 3, other: 5 }, {}, { c1: "s1", c2: "s1", other: "s2" });
    expect(useUnreadStore.getState().serverUnread("s1")).toBe(5);
    expect(useUnreadStore.getState().serverUnread("s2")).toBe(5);
    expect(useUnreadStore.getState().serverUnread("missing")).toBe(0);
  });

  it("clears a single scope and everything at once", () => {
    const store = useUnreadStore.getState();
    store.bumpChannel("c1", "s1");
    store.bumpDm("f1");
    store.resetChannel("c1");
    expect(useUnreadStore.getState().byChannel.c1).toBeUndefined();
    expect(useUnreadStore.getState().byDm.f1).toBe(1);
    useUnreadStore.getState().clearAll();
    expect(useUnreadStore.getState().byDm).toEqual({});
  });
});
