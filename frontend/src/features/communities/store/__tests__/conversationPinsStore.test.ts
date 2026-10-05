import { http } from "../../../../shared/api/http";
import { MAX_PINNED_CONVERSATIONS, pinnedFirst, togglePinned, useConversationPinsStore } from "../conversationPinsStore";

jest.mock("../../../../shared/api/http", () => ({
  http: { get: jest.fn(), put: jest.fn() },
}));

const get = http.get as jest.Mock;
const put = http.put as jest.Mock;
const reply = (data: unknown) => ({ data: { data } });

beforeEach(() => {
  get.mockReset();
  put.mockReset();
  useConversationPinsStore.getState().clearAll();
  useConversationPinsStore.getState().applyRemote({ dms: [], communities: [] });
});

it("puts pinned items first in pin order and keeps the rest in place", () => {
  const items = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
  expect(pinnedFirst(items, ["c", "x", "a"]).map((i) => i.id)).toEqual(["c", "a", "b", "d"]);
  expect(pinnedFirst(items, [])).toBe(items);
  expect(togglePinned(["a"], "b")).toEqual(["b", "a"]);
  expect(togglePinned(["a", "b"], "a")).toEqual(["b"]);
});

it("pins optimistically and keeps the server answer", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  const pending = useConversationPinsStore.getState().toggle("dms", "u2");
  expect(useConversationPinsStore.getState().dms).toEqual(["u2"]);
  await Promise.resolve();
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["u2"] }, expect.anything());
  finish(reply({ dms: ["u2"], communities: ["c1"] }));
  await pending;
  expect(useConversationPinsStore.getState().communities).toEqual(["c1"]);
  expect(get).not.toHaveBeenCalled();
});

it("reloads after a write when another device changed pins meanwhile", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  get.mockResolvedValue(reply({ dms: ["u9", "u2"], communities: [] }));
  const pending = useConversationPinsStore.getState().toggle("dms", "u2");
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
  const old = useConversationPinsStore.getState().toggle("dms", "a");
  await Promise.resolve();
  useConversationPinsStore.getState().clearAll();
  useConversationPinsStore.getState().applyRemote({ dms: [], communities: [] });
  finish(reply({ dms: ["a"], communities: [] }));
  await old;
  expect(useConversationPinsStore.getState().dms).toEqual([]);
  put.mockResolvedValueOnce(reply({ dms: ["b"], communities: [] }));
  await useConversationPinsStore.getState().toggle("dms", "b");
  useConversationPinsStore.getState().applyRemote({ dms: ["remote"] });
  expect(useConversationPinsStore.getState().dms).toEqual(["remote"]);
});

it("drops communities the user is no longer in when toggling", async () => {
  useConversationPinsStore.getState().applyRemote({ communities: ["gone", "c1"] });
  put.mockResolvedValue(reply({ dms: [], communities: ["c2", "c1"] }));
  await useConversationPinsStore.getState().toggle("communities", "c2", (id) => id === "c1");
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/communities", { ids: ["c2", "c1"] }, expect.anything());
});

it("reloads from the server when saving fails", async () => {
  put.mockRejectedValue(new Error("boom"));
  get.mockResolvedValue(reply({ dms: [], communities: [] }));
  await expect(useConversationPinsStore.getState().toggle("communities", "c1")).rejects.toThrow("boom");
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledWith("/v1/users/me/pins", expect.anything());
  expect(useConversationPinsStore.getState().communities).toEqual([]);
});

it("refuses to pin more than the limit", async () => {
  const full = Array.from({ length: MAX_PINNED_CONVERSATIONS }, (_, i) => `u${i}`);
  useConversationPinsStore.getState().applyRemote({ dms: full });
  await expect(useConversationPinsStore.getState().toggle("dms", "extra")).rejects.toThrow("PIN_LIMIT_REACHED");
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
  put.mockResolvedValue(reply({ dms: ["new", "old"], communities: [] }));
  await useConversationPinsStore.getState().toggle("dms", "new");
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["new", "old"] }, expect.anything());
});

it("refuses to toggle when the pins cannot be loaded", async () => {
  useConversationPinsStore.getState().clearAll();
  get.mockRejectedValue(new Error("offline"));
  await expect(useConversationPinsStore.getState().toggle("dms", "x")).rejects.toThrow("offline");
  expect(put).not.toHaveBeenCalled();
});

it("never sends a queued write after the account changes", async () => {
  let finish: (v: unknown) => void = () => undefined;
  put.mockImplementationOnce(() => new Promise((resolve) => (finish = resolve)));
  const first = useConversationPinsStore.getState().toggle("dms", "a");
  const second = useConversationPinsStore.getState().toggle("dms", "b");
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
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  const first = useConversationPinsStore.getState().toggle("dms", "a");
  const second = useConversationPinsStore.getState().toggle("dms", "b");
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
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  await useConversationPinsStore.getState().toggle("dms", "new", (id) => id !== "blocked");
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["new", "x"] }, expect.anything());
});

it("goes on when the reconnect fetch loaded pins even if its own load failed", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveReconnect: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveReconnect = resolve)))
    .mockRejectedValueOnce(new Error("toggle load failed"));
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  const reconnect = useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().toggle("dms", "a");
  resolveReconnect(reply({ dms: ["old"], communities: [] }));
  await reconnect;
  await toggle;
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["a", "old"] }, expect.anything());
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
  void useConversationPinsStore.getState().toggle("dms", "a").catch(() => undefined);
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
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  useConversationPinsStore.getState().remoteChanged();
  const toggle = useConversationPinsStore.getState().toggle("dms", "mine");
  resolveReload(reply({ dms: ["other-device"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["mine", "other-device"] }, expect.anything());
});

it("waits for reloads that start while it is already waiting", async () => {
  let resolveA: (v: unknown) => void = () => undefined;
  let resolveB: (v: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveA = resolve)))
    .mockImplementationOnce(() => new Promise((resolve) => (resolveB = resolve)));
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  void useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().toggle("dms", "mine");
  void useConversationPinsStore.getState().fetch();
  resolveA(reply({ dms: ["a"], communities: [] }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(put).not.toHaveBeenCalled();
  resolveB(reply({ dms: ["b"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["mine", "b"] }, expect.anything());
});

it("uses the reconnect load when its own first load fails", async () => {
  useConversationPinsStore.getState().clearAll();
  let resolveReconnect: (v: unknown) => void = () => undefined;
  let rejectOwn: (e: unknown) => void = () => undefined;
  get
    .mockImplementationOnce(() => new Promise((resolve) => (resolveReconnect = resolve)))
    .mockImplementationOnce(() => new Promise((_resolve, reject) => (rejectOwn = reject)));
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  void useConversationPinsStore.getState().fetch();
  const toggle = useConversationPinsStore.getState().toggle("dms", "a");
  rejectOwn(new Error("own failed"));
  await new Promise((resolve) => setTimeout(resolve, 0));
  resolveReconnect(reply({ dms: ["old"], communities: [] }));
  await toggle;
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["a", "old"] }, expect.anything());
});
