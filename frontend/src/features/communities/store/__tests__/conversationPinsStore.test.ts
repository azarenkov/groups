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
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["u2"] });
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
  await useConversationPinsStore.getState().toggle("communities", "c2", new Set(["c1", "c2"]));
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/communities", { ids: ["c2", "c1"] });
});

it("reloads from the server when saving fails", async () => {
  put.mockRejectedValue(new Error("boom"));
  get.mockResolvedValue(reply({ dms: [], communities: [] }));
  await expect(useConversationPinsStore.getState().toggle("communities", "c1")).rejects.toThrow("boom");
  await Promise.resolve();
  await Promise.resolve();
  expect(get).toHaveBeenCalledWith("/v1/users/me/pins");
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
  expect(put).toHaveBeenCalledWith("/v1/users/me/pins/dms", { ids: ["new", "old"] });
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

it("lets two early toggles share the initial load", async () => {
  useConversationPinsStore.getState().clearAll();
  const gets: ((v: unknown) => void)[] = [];
  get.mockImplementation(() => new Promise((resolve) => gets.push(resolve)));
  put.mockImplementation((_url: string, body: { ids: string[] }) => Promise.resolve(reply({ dms: body.ids, communities: [] })));
  const first = useConversationPinsStore.getState().toggle("dms", "a");
  const second = useConversationPinsStore.getState().toggle("dms", "b");
  expect(gets).toHaveLength(2);
  gets[0]({ data: { data: { dms: ["old"], communities: [] } } });
  gets[1]({ data: { data: { dms: ["old"], communities: [] } } });
  await first;
  await second;
  expect(useConversationPinsStore.getState().dms).toEqual(["b", "a", "old"]);
});

it("forgets a community locally without saving", () => {
  useConversationPinsStore.getState().applyRemote({ communities: ["c1", "c2"] });
  useConversationPinsStore.getState().forget("communities", "c1");
  expect(useConversationPinsStore.getState().communities).toEqual(["c2"]);
  expect(put).not.toHaveBeenCalled();
});
