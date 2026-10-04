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
  useConversationPinsStore.getState().applyRemote({ dms: [] });
  expect(useConversationPinsStore.getState().dms).toEqual(["u2"]);
  finish(reply({ dms: ["u2"], communities: ["c1"] }));
  await pending;
  expect(useConversationPinsStore.getState().communities).toEqual(["c1"]);
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
