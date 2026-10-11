import { http } from "../../../../shared/api/http";
import { isDmHidden, useHiddenDmStore, withHidden } from "../hiddenDmStore";

jest.mock("../../../../shared/api/http", () => ({
  http: { get: jest.fn(), put: jest.fn(), delete: jest.fn() },
}));

const get = http.get as jest.Mock;
const put = http.put as jest.Mock;
const del = http.delete as jest.Mock;
const reply = (peers: string[]) => ({ data: { data: { peers } } });

beforeEach(() => {
  get.mockReset();
  put.mockReset();
  del.mockReset();
  useHiddenDmStore.getState().clearAll();
});

it("adds and removes ids without duplicates", () => {
  expect(withHidden(["a"], "a", true)).toEqual(["a"]);
  expect(withHidden(["a"], "b", true)).toEqual(["a", "b"]);
  expect(withHidden(["a", "b"], "a", false)).toEqual(["b"]);
});

it("loads hidden chats from the server", async () => {
  get.mockResolvedValue(reply(["p1", "p2"]));
  await useHiddenDmStore.getState().fetch();
  expect(get).toHaveBeenCalledWith("/v1/users/me/hidden-dms", expect.anything());
  expect(useHiddenDmStore.getState()).toMatchObject({ hidden: ["p1", "p2"], loaded: true });
  expect(isDmHidden("p1")).toBe(true);
  expect(isDmHidden("p3")).toBe(false);
});

it("hides and unhides optimistically through the API", async () => {
  put.mockResolvedValue({});
  del.mockResolvedValue({});
  const hiding = useHiddenDmStore.getState().setHidden("p1", true);
  expect(useHiddenDmStore.getState().hidden).toEqual(["p1"]);
  await hiding;
  expect(put).toHaveBeenCalledWith("/v1/users/me/hidden-dms/p1", undefined, expect.anything());
  await useHiddenDmStore.getState().setHidden("p1", false);
  expect(del).toHaveBeenCalledWith("/v1/users/me/hidden-dms/p1", expect.anything());
  expect(useHiddenDmStore.getState().hidden).toEqual([]);
});

it("reloads the server state when a write fails", async () => {
  put.mockRejectedValue(new Error("boom"));
  get.mockResolvedValue(reply([]));
  await expect(useHiddenDmStore.getState().setHidden("p1", true)).rejects.toThrow("boom");
  await new Promise((r) => setTimeout(r, 0));
  expect(get).toHaveBeenCalled();
  expect(useHiddenDmStore.getState().hidden).toEqual([]);
});

it("ignores a fetch that started before a remote change", async () => {
  let resolve!: (v: unknown) => void;
  get.mockReturnValue(new Promise((r) => (resolve = r)));
  const loading = useHiddenDmStore.getState().fetch();
  useHiddenDmStore.getState().applyRemote("p9", true);
  resolve(reply([]));
  await loading;
  expect(useHiddenDmStore.getState().hidden).toEqual(["p9"]);
});

it("drops a fetch from a previous session", async () => {
  let resolve!: (v: unknown) => void;
  get.mockReturnValue(new Promise((r) => (resolve = r)));
  const loading = useHiddenDmStore.getState().fetch();
  useHiddenDmStore.getState().clearAll();
  resolve(reply(["old"]));
  await loading;
  expect(useHiddenDmStore.getState()).toMatchObject({ hidden: [], loaded: false });
});
