import { http } from "../../../../shared/api/http";
import { HIDDEN_LOAD_RETRY_MS, hiddenDmsKnown, isDmHidden, useHiddenDmStore, withHidden } from "../hiddenDmStore";

const act = async () => {
  for (let i = 0; i < 5; i += 1) await Promise.resolve();
};

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

it("refetches when a remote change lands during a fetch", async () => {
  let resolve!: (v: unknown) => void;
  get.mockReturnValueOnce(new Promise((r) => (resolve = r))).mockResolvedValueOnce(reply(["p1", "p9"]));
  const loading = useHiddenDmStore.getState().fetch();
  useHiddenDmStore.getState().applyRemote("p9", true);
  resolve(reply(["p1"]));
  await loading;
  expect(get).toHaveBeenCalledTimes(2);
  expect(useHiddenDmStore.getState()).toMatchObject({ hidden: ["p1", "p9"], loaded: true });
});

it("reconciles after a write that overlapped the initial load", async () => {
  let resolveList!: (v: unknown) => void;
  let resolvePut!: (v: unknown) => void;
  get.mockReturnValueOnce(new Promise((r) => (resolveList = r))).mockResolvedValue(reply(["p1", "p2"]));
  put.mockReturnValue(new Promise((r) => (resolvePut = r)));
  const loading = useHiddenDmStore.getState().fetch();
  const hiding = useHiddenDmStore.getState().setHidden("p2", true);
  resolveList(reply(["p1"]));
  await loading;
  expect(useHiddenDmStore.getState()).toMatchObject({ hidden: ["p2"], loaded: false });
  resolvePut({});
  await hiding;
  await new Promise((r) => setTimeout(r, 0));
  expect(useHiddenDmStore.getState()).toMatchObject({ hidden: ["p1", "p2"], loaded: true });
});

it("keeps retrying the initial load until it succeeds", async () => {
  jest.useFakeTimers();
  try {
    get.mockRejectedValueOnce(new Error("down")).mockResolvedValueOnce(reply(["p1"]));
    useHiddenDmStore.getState().ensureLoaded();
    await act();
    expect(useHiddenDmStore.getState().loaded).toBe(false);
    jest.advanceTimersByTime(HIDDEN_LOAD_RETRY_MS);
    await act();
    expect(useHiddenDmStore.getState()).toMatchObject({ hidden: ["p1"], loaded: true });
  } finally {
    jest.useRealTimers();
  }
});

it("marks the list unknown until a reconnect reload finishes", async () => {
  get.mockResolvedValueOnce(reply(["p1"]));
  await useHiddenDmStore.getState().fetch();
  let resolve!: (v: unknown) => void;
  get.mockReturnValueOnce(new Promise((r) => (resolve = r)));
  useHiddenDmStore.getState().markStale();
  expect(hiddenDmsKnown()).toBe(false);
  resolve(reply(["p1", "p2"]));
  await act();
  expect(hiddenDmsKnown()).toBe(true);
  expect(useHiddenDmStore.getState().hidden).toEqual(["p1", "p2"]);
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

it("sends overlapping writes in the order they were made", async () => {
  const calls: string[] = [];
  let resolvePut!: (v: unknown) => void;
  put.mockImplementation(() => {
    calls.push("put");
    return new Promise((r) => (resolvePut = r));
  });
  del.mockImplementation(() => {
    calls.push("delete");
    return Promise.resolve({});
  });
  const hiding = useHiddenDmStore.getState().setHidden("p1", true);
  const unhiding = useHiddenDmStore.getState().setHidden("p1", false);
  await act();
  expect(calls).toEqual(["put"]);
  resolvePut({});
  await Promise.all([hiding, unhiding]);
  expect(calls).toEqual(["put", "delete"]);
  expect(useHiddenDmStore.getState().hidden).toEqual([]);
});
