import { http } from "../../../../shared/api/http";
import { MAX_PINNED_CONVERSATIONS, PINS_LOAD_MAX_DELAY_MS, pinnedFirst, withPinned, useConversationPinsStore } from "../conversationPinsStore";

jest.mock("../../../../shared/api/http", () => ({
  http: { get: jest.fn(), put: jest.fn(), delete: jest.fn() },
}));

const get = http.get as jest.Mock;
const put = http.put as jest.Mock;
const del = http.delete as jest.Mock;
const reply = (data: unknown) => ({ data: { data } });

const fakeServer = (dms: string[], communities: string[] = []) => {
  const state: Record<string, string[]> = { dms: [...dms], communities: [...communities] };
  const target = (url: string) => url.split("/").slice(-2) as [string, string];
  put.mockImplementation((url: string) => {
    const [kind, id] = target(url);
    state[kind] = [id, ...state[kind].filter((x) => x !== id)];
    return Promise.resolve(reply({ dms: [...state.dms], communities: [...state.communities] }));
  });
  del.mockImplementation((url: string) => {
    const [kind, id] = target(url);
    state[kind] = state[kind].filter((x) => x !== id);
    return Promise.resolve(reply({ dms: [...state.dms], communities: [...state.communities] }));
  });
};

const pinUrl = (kind: string, id: string) => `/v1/users/me/pins/${kind}/${id}`;

beforeEach(() => {
  get.mockReset();
  put.mockReset();
  del.mockReset();
  useConversationPinsStore.getState().clearAll();
  useConversationPinsStore.getState().applyRemote({ dms: [], communities: [] });
});

it("puts pinned items first in pin order and keeps the rest in place", () => {
  const items = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
  expect(pinnedFirst(items, ["c", "x", "a"]).map((i) => i.id)).toEqual(["c", "a", "b", "d"]);
  expect(pinnedFirst(items, [])).toBe(items);
  expect(withPinned(["a"], "b", true)).toEqual(["b", "a"]);
  expect(withPinned(["a"], "a", true)).toEqual(["a"]);
  expect(withPinned(["a", "b"], "a", false)).toEqual(["b"]);
});

it("pins optimistically and keeps the server answer", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  const pending = useConversationPinsStore.getState().setPinned("dms", "u2", true);
  expect(useConversationPinsStore.getState().dms).toEqual(["u2"]);
  await Promise.resolve();
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "u2"), undefined, expect.anything());
  finish(reply({ dms: ["u2"], communities: ["c1"] }));
  await pending;
  expect(useConversationPinsStore.getState().communities).toEqual(["c1"]);
  expect(get).not.toHaveBeenCalled();
});

it("reloads after a write when another device changed pins meanwhile", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  get.mockResolvedValue(reply({ dms: ["u9", "u2"], communities: [] }));
  const pending = useConversationPinsStore.getState().setPinned("dms", "u2", true);
  await Promise.resolve();
  useConversationPinsStore.getState().applyRemote({ dms: ["u9"] });
  expect(useConversationPinsStore.getState().dms).toEqual(["u2"]);
  finish(reply({ dms: ["u2"], communities: [] }));
  await pending;
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledTimes(1);
  expect(useConversationPinsStore.getState().dms).toEqual(["u9", "u2"]);
});

it("ignores an older fetch that finishes after a newer update", async () => {
  let resolveOld: (v: unknown) => void = () => undefined;
  get.mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve)));
  const old = useConversationPinsStore.getState().fetch();
  useConversationPinsStore.getState().applyRemote({ dms: ["fresh"] });
  resolveOld(reply({ dms: ["stale"], communities: [] }));
  await old;
  expect(useConversationPinsStore.getState().dms).toEqual(["fresh"]);
});

it("does not let a write from the previous account affect the next one", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const old = useConversationPinsStore.getState().setPinned("dms", "a", true);
  await Promise.resolve();
  useConversationPinsStore.getState().clearAll();
  useConversationPinsStore.getState().applyRemote({ dms: [], communities: [] });
  finish(reply({ dms: ["a"], communities: [] }));
  await old;
  expect(useConversationPinsStore.getState().dms).toEqual([]);
  put.mockResolvedValueOnce(reply({ dms: ["b"], communities: [] }));
  await useConversationPinsStore.getState().setPinned("dms", "b", true);
  useConversationPinsStore.getState().applyRemote({ dms: ["remote"] });
  expect(useConversationPinsStore.getState().dms).toEqual(["remote"]);
});

it("drops communities the user is no longer in when toggling", async () => {
  useConversationPinsStore.getState().applyRemote({ communities: ["gone", "c1"] });
  fakeServer([], ["gone", "c1"]);
  await useConversationPinsStore.getState().setPinned("communities", "c2", true, (id) => id === "c1");
  expect(del).toHaveBeenCalledWith(pinUrl("communities", "gone"), expect.anything());
  expect(put).toHaveBeenCalledWith(pinUrl("communities", "c2"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().communities).toEqual(["c2", "c1"]);
});

it("reloads from the server when saving fails", async () => {
  put.mockRejectedValue(new Error("boom"));
  get.mockResolvedValue(reply({ dms: [], communities: [] }));
  await expect(useConversationPinsStore.getState().setPinned("communities", "c1", true)).rejects.toThrow("boom");
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledWith("/v1/users/me/pins", expect.anything());
  expect(useConversationPinsStore.getState().communities).toEqual([]);
});

it("refuses to pin more than the limit", async () => {
  const full = Array.from({ length: MAX_PINNED_CONVERSATIONS }, (_, i) => `u${i}`);
  useConversationPinsStore.getState().applyRemote({ dms: full });
  await expect(useConversationPinsStore.getState().setPinned("dms", "extra", true)).rejects.toThrow("PIN_LIMIT_REACHED");
  expect(put).not.toHaveBeenCalled();
  expect(useConversationPinsStore.getState().dms).toEqual(full);
});

it("applies pins changed on another device", () => {
  useConversationPinsStore.getState().applyRemote({ dms: ["u1"], communities: ["c1"] });
  expect(useConversationPinsStore.getState()).toMatchObject({ dms: ["u1"], communities: ["c1"], loaded: true });
});

it("loads the current pins before the first toggle so nothing is lost", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockResolvedValue(reply({ dms: ["old"], communities: [] }));
  fakeServer(["old"]);
  await useConversationPinsStore.getState().setPinned("dms", "new", true);
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "new"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["new", "old"]);
});

it("refuses to toggle when the pins cannot be loaded", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockRejectedValue(new Error("offline"));
  await expect(useConversationPinsStore.getState().setPinned("dms", "x", true)).rejects.toThrow("offline");
  expect(put).not.toHaveBeenCalled();
});

it("never sends a queued write after the account changes", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const first = useConversationPinsStore.getState().setPinned("dms", "a", true);
  const second = useConversationPinsStore.getState().setPinned("dms", "b", true);
  await Promise.resolve();
  await Promise.resolve();
  useConversationPinsStore.getState().clearAll();
  finish(reply({ dms: ["a"], communities: [] }));
  await first;
  await expect(second).rejects.toThrow("SESSION_CHANGED");
  expect(put).toHaveBeenCalledTimes(1);
});

it("lets two early toggles share one initial load", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockResolvedValueOnce(reply({ dms: ["old"], communities: [] })).mockRejectedValueOnce(new Error("second"));
  fakeServer(["old"]);
  const first = useConversationPinsStore.getState().setPinned("dms", "a", true);
  const second = useConversationPinsStore.getState().setPinned("dms", "b", true);
  await first;
  await second;
  expect(get).toHaveBeenCalledTimes(1);
  expect(useConversationPinsStore.getState().dms).toEqual(["b", "a", "old"]);
});

it("forgets a community locally without saving", () => {
  useConversationPinsStore.getState().applyRemote({ communities: ["c1", "c2"] });
  useConversationPinsStore.getState().forget("communities", "c1");
  expect(useConversationPinsStore.getState().communities).toEqual(["c2"]);
  expect(put).not.toHaveBeenCalled();
});

it("filters with the loaded list, not the one seen before loading", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockResolvedValue(reply({ dms: ["x", "blocked"], communities: [] }));
  fakeServer(["x", "blocked"]);
  await useConversationPinsStore.getState().setPinned("dms", "new", true, (id) => id !== "blocked");
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "new"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["new", "x"]);
});

it("goes on when the reconnect fetch loaded pins even if its own load failed", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveReconnect: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveReconnect = resolve)))
    .mockRejectedValueOnce(new Error("toggle load failed"));
  fakeServer(["old"]);
  const reconnect = useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().setPinned("dms", "a", true);
  resolveReconnect(reply({ dms: ["old"], communities: [] }));
  await reconnect;
  await toggle;
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "a"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["a", "old"]);
});

it("keeps an older successful fetch when a newer one fails", async () => {
  let resolveOld: (v: unknown) => void = () => undefined;
  let rejectNew: (e: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve)))
    .mockImplementationOnce(() => new Promise((_resolve, reject) => (rejectNew = reject)));
  const older = useConversationPinsStore.getState().fetch();
  const newer = useConversationPinsStore.getState().fetch();
  rejectNew(new Error("offline"));
  await expect(newer).rejects.toThrow("offline");
  resolveOld(reply({ dms: ["kept"], communities: [] }));
  await older;
  expect(useConversationPinsStore.getState().dms).toEqual(["kept"]);
});

it("aborts requests of the previous account", async () => {
  put.mockImplementation(() => new Promise(() => undefined));
  void useConversationPinsStore.getState().setPinned("dms", "a", true).catch(() => undefined);
  await Promise.resolve();
  await Promise.resolve();
  const signal = put.mock.calls[0][2].signal as AbortSignal;
  expect(signal.aborted).toBe(false);
  useConversationPinsStore.getState().clearAll();
  expect(signal.aborted).toBe(true);
});

it("reloads from the server on a remote change instead of trusting the event order", async () => {
  get.mockResolvedValue(reply({ dms: ["current"], communities: [] }));
  useConversationPinsStore.getState().remoteChanged();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(get).toHaveBeenCalledTimes(1);
  expect(useConversationPinsStore.getState().dms).toEqual(["current"]);
});

it("waits for a running reload before building the next pin list", async () => {
  let resolveReload: (v: unknown) => void = () => undefined;
  get.mockImplementationOnce(() => new Promise((resolve) => (resolveReload = resolve)));
  fakeServer(["other-device"]);
  useConversationPinsStore.getState().remoteChanged();
  const toggle = useConversationPinsStore.getState().setPinned("dms", "mine", true);
  resolveReload(reply({ dms: ["other-device"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "mine"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["mine", "other-device"]);
});

it("waits for reloads that start while it is already waiting", async () => {
  let resolveA: (v: unknown) => void = () => undefined;
  let resolveB: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveA = resolve)))
    .mockImplementationOnce(() => new Promise((resolve) => (resolveB = resolve)));
  fakeServer(["b"]);
  void useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().setPinned("dms", "mine", true);
  void useConversationPinsStore.getState().fetch();
  resolveA(reply({ dms: ["a"], communities: [] }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(put).not.toHaveBeenCalled();
  resolveB(reply({ dms: ["b"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "mine"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["mine", "b"]);
});

it("uses the reconnect load when its own first load fails", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveReconnect: (v: unknown) => void = () => undefined;
  let rejectOwn: (e: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveReconnect = resolve)))
    .mockImplementationOnce(() => new Promise((_resolve, reject) => (rejectOwn = reject)));
  fakeServer(["old"]);
  void useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().setPinned("dms", "a", true);
  rejectOwn(new Error("own failed"));
  await new Promise((resolve) => setTimeout(resolve, 0));
  resolveReconnect(reply({ dms: ["old"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "a"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["a", "old"]);
});

it("keeps a chat pinned when Pin was chosen before the list loaded", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockResolvedValue(reply({ dms: ["a", "b"], communities: [] }));
  await useConversationPinsStore.getState().setPinned("dms", "a", true);
  expect(put).not.toHaveBeenCalled();
  expect(useConversationPinsStore.getState().dms).toEqual(["a", "b"]);
});

it("does not bring back a community forgotten while a write was running", async () => {
  useConversationPinsStore.getState().applyRemote({ communities: ["left"] });
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  get.mockResolvedValue(reply({ dms: [], communities: ["c1"] }));
  const pending = useConversationPinsStore.getState().setPinned("communities", "c1", true);
  await new Promise((resolve) => setTimeout(resolve, 0));
  useConversationPinsStore.getState().forget("communities", "left");
  finish(reply({ dms: [], communities: ["c1", "left"] }));
  await pending;
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(useConversationPinsStore.getState().communities).toEqual(["c1"]);
});

it("lets a newer reload win over the first toggle's load", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveInitial: (v: unknown) => void = () => undefined;
  let resolveReload: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveInitial = resolve)))
    .mockImplementationOnce(() => new Promise((resolve) => (resolveReload = resolve)));
  fakeServer(["newer", "old"]);
  const toggle = useConversationPinsStore.getState().setPinned("dms", "mine", true);
  void useConversationPinsStore.getState().fetch();
  resolveInitial(reply({ dms: ["old"], communities: [] }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  resolveReload(reply({ dms: ["newer", "old"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith(pinUrl("dms", "mine"), undefined, expect.anything());
  expect(useConversationPinsStore.getState().dms).toEqual(["mine", "newer", "old"]);
});

it("drops a load that started before leaving a community and reloads", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveOld: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve)))
    .mockResolvedValueOnce(reply({ dms: [], communities: ["kept"] }));
  const old = useConversationPinsStore.getState().fetch();
  useConversationPinsStore.getState().forget("communities", "left");
  resolveOld(reply({ dms: [], communities: ["left", "kept"] }));
  await old;
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(useConversationPinsStore.getState().communities).toEqual(["kept"]);
});

it("keeps retrying the first load until it succeeds", async () => {
  useConversationPinsStore.getState().clearAll();
  jest.useFakeTimers();
  get
    .mockRejectedValueOnce(new Error("down"))
    .mockRejectedValueOnce(new Error("down"))
    .mockResolvedValueOnce(reply({ dms: ["a"], communities: [] }));
  const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
  useConversationPinsStore.getState().ensureLoaded();
  useConversationPinsStore.getState().ensureLoaded();
  for (let i = 0; i < 5; i += 1) {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    jest.advanceTimersByTime(PINS_LOAD_MAX_DELAY_MS);
  }
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledTimes(3);
  expect(useConversationPinsStore.getState()).toMatchObject({ loaded: true, dms: ["a"] });
  warn.mockRestore();
  jest.useRealTimers();
});

it("retries reloading after a remote change until it works", async () => {
  jest.useFakeTimers();
  const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
  get.mockRejectedValueOnce(new Error("down")).mockResolvedValueOnce(reply({ dms: ["remote"], communities: [] }));
  useConversationPinsStore.getState().remoteChanged();
  for (let i = 0; i < 4; i += 1) {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    jest.advanceTimersByTime(PINS_LOAD_MAX_DELAY_MS);
  }
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledTimes(2);
  expect(useConversationPinsStore.getState().dms).toEqual(["remote"]);
  warn.mockRestore();
  jest.useRealTimers();
});

it("keeps retrying to reconcile after a failed save", async () => {
  jest.useFakeTimers();
  const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
  put.mockRejectedValueOnce(new Error("save failed"));
  get.mockRejectedValueOnce(new Error("down")).mockResolvedValueOnce(reply({ dms: [], communities: [] }));
  await expect(useConversationPinsStore.getState().setPinned("dms", "x", true)).rejects.toThrow("save failed");
  expect(useConversationPinsStore.getState().dms).toEqual(["x"]);
  for (let i = 0; i < 4; i += 1) {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    jest.advanceTimersByTime(PINS_LOAD_MAX_DELAY_MS);
  }
  await Promise.resolve();
  await Promise.resolve();
  expect(useConversationPinsStore.getState().dms).toEqual([]);
  warn.mockRestore();
  jest.useRealTimers();
});

it("unpins a single chat without sending the rest of the list", async () => {
  useConversationPinsStore.getState().applyRemote({ dms: ["a", "b"] });
  fakeServer(["a", "b", "from-other-device"]);
  await useConversationPinsStore.getState().setPinned("dms", "a", false);
  expect(del).toHaveBeenCalledWith(pinUrl("dms", "a"), expect.anything());
  expect(put).not.toHaveBeenCalled();
  expect(useConversationPinsStore.getState().dms).toEqual(["b", "from-other-device"]);
});
