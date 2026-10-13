import { queryClient, queryKeys } from "../../../../shared/query/queryClient";
import { searchHistoryAPI, type HistoryItem } from "../../api/searchHistory";
import { useAuthStore } from "../../../auth/store/authStore";
import {
  clearSearches,
  clearStoredHistory,
  installHistoryPersistence,
  readStoredHistory,
  resetSearchHistorySession,
  recordSearch,
  removeSearch,
  storeHistory,
} from "../searchHistory";

jest.mock("../../api/searchHistory", () => {
  const actual = jest.requireActual("../../api/searchHistory");
  return {
    ...actual,
    searchHistoryAPI: { list: jest.fn(), record: jest.fn(), remove: jest.fn(), clear: jest.fn() },
  };
});

const api = searchHistoryAPI as jest.Mocked<typeof searchHistoryAPI>;
const key = queryKeys.searchHistory("me");
const items = () => queryClient.getQueryData<HistoryItem[]>(key) ?? [];

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

const query = (id: string, text: string): HistoryItem => ({ id, usedAt: "2026-10-13T10:00:00Z", type: "query", text });

beforeEach(() => {
  queryClient.clear();
  clearStoredHistory();
  jest.resetAllMocks();
  useAuthStore.setState({ user: { id: "me", name: "me" } as never });
});

it("puts a new entry on top at once, folds duplicates and adopts the server id", async () => {
  queryClient.setQueryData(key, [query("a", "Rust"), query("b", "go")]);
  let finish: (v: { id: string; usedAt: string }) => void = () => {};
  api.record.mockReturnValue(new Promise((resolve) => (finish = resolve)));
  const done = recordSearch("me", { type: "query", text: "rust" });
  await flush();
  expect(items().map((i) => (i.type === "query" ? i.text : ""))).toEqual(["rust", "go"]);
  expect(items()[0].id).toMatch(/^pending:/);
  finish({ id: "server-1", usedAt: "2026-10-13T11:00:00Z" });
  await done;
  expect(items()[0]).toMatchObject({ id: "server-1", usedAt: "2026-10-13T11:00:00Z", text: "rust" });
});

it("rolls back an optimistic change the server refused", async () => {
  queryClient.setQueryData(key, [query("a", "Rust")]);
  api.clear.mockRejectedValue(new Error("down"));
  await expect(clearSearches("me")).rejects.toThrow("down");
  expect(items().map((i) => i.id)).toEqual(["a"]);
  api.remove.mockRejectedValue(new Error("down"));
  await expect(removeSearch("me", "a")).rejects.toThrow("down");
  expect(items().map((i) => i.id)).toEqual(["a"]);
});

it("deletes an entry removed before its save came back by its server id", async () => {
  queryClient.setQueryData(key, []);
  let finish: (v: { id: string; usedAt: string }) => void = () => {};
  api.record.mockReturnValue(new Promise((resolve) => (finish = resolve)));
  api.remove.mockResolvedValue();
  const saving = recordSearch("me", { type: "person", userId: "u1", name: "Bob" });
  await flush();
  const tempId = items()[0].id;
  const removing = removeSearch("me", tempId);
  await flush();
  expect(items()).toEqual([]);
  finish({ id: "server-7", usedAt: "2026-10-13T11:00:00Z" });
  await saving;
  await removing;
  expect(api.remove).toHaveBeenCalledWith("server-7");
  expect(items()).toEqual([]);
});

it("keeps the browser copy per account and without unsaved entries", () => {
  storeHistory("me", [query("a", "Rust"), query("pending:9", "draft")]);
  expect(readStoredHistory("me")?.map((i) => i.id)).toEqual(["a"]);
  expect(readStoredHistory("someone-else")).toBeUndefined();
  clearStoredHistory();
  expect(readStoredHistory("me")).toBeUndefined();
});

it("clears only after earlier saves have reached the server", async () => {
  queryClient.setQueryData(key, []);
  let finish: (v: { id: string; usedAt: string }) => void = () => {};
  api.record.mockReturnValue(new Promise((resolve) => (finish = resolve)));
  api.clear.mockResolvedValue();
  const saving = recordSearch("me", { type: "query", text: "late" });
  const clearing = clearSearches("me");
  await flush();
  expect(items()).toEqual([]);
  expect(api.clear).not.toHaveBeenCalled();
  finish({ id: "server-2", usedAt: "2026-10-13T11:00:00Z" });
  await saving;
  await clearing;
  expect(api.clear).toHaveBeenCalled();
  expect(items()).toEqual([]);
});

it("keeps the browser copy current even when nothing is watching the history", async () => {
  installHistoryPersistence();
  queryClient.setQueryData(key, []);
  api.record.mockResolvedValue({ id: "server-3", usedAt: "2026-10-13T11:00:00Z" });
  await recordSearch("me", { type: "query", text: "offline" });
  expect(readStoredHistory("me")).toEqual([
    expect.objectContaining({ id: "server-3", type: "query", text: "offline" }),
  ]);
});

it("sends a save only after an earlier clear has gone through", async () => {
  queryClient.setQueryData(key, [query("a", "old")]);
  let finishClear: () => void = () => {};
  api.clear.mockReturnValue(new Promise<void>((resolve) => (finishClear = resolve)));
  api.record.mockResolvedValue({ id: "server-4", usedAt: "2026-10-13T11:00:00Z" });
  const clearing = clearSearches("me");
  const saving = recordSearch("me", { type: "query", text: "fresh" });
  await flush();
  expect(items().map((i) => (i.type === "query" ? i.text : ""))).toEqual(["fresh"]);
  expect(api.record).not.toHaveBeenCalled();
  finishClear();
  await clearing;
  await saving;
  expect(api.record).toHaveBeenCalledWith({ type: "query", text: "fresh" });
  expect(items()[0].id).toBe("server-4");
});

it("restores a saved duplicate under its server id when the newer save fails", async () => {
  queryClient.setQueryData(key, []);
  let finishFirst: (v: { id: string; usedAt: string }) => void = () => {};
  let failSecond: (e: Error) => void = () => {};
  api.record
    .mockReturnValueOnce(new Promise((resolve) => (finishFirst = resolve)))
    .mockReturnValueOnce(new Promise((_, reject) => (failSecond = reject)));
  const first = recordSearch("me", { type: "person", userId: "u1", name: "Bob" });
  const second = recordSearch("me", { type: "person", userId: "u1", name: "Bob" });
  finishFirst({ id: "server-5", usedAt: "2026-10-13T11:00:00Z" });
  await first;
  await flush();
  failSecond(new Error("down"));
  await expect(second).rejects.toThrow("down");
  expect(items().map((i) => i.id)).toEqual(["server-5"]);
});

it("drops queued changes once the account behind them signs out", async () => {
  queryClient.setQueryData(key, []);
  let finish: (v: { id: string; usedAt: string }) => void = () => {};
  api.record.mockReturnValue(new Promise((resolve) => (finish = resolve)));
  api.clear.mockResolvedValue();
  const saving = recordSearch("me", { type: "query", text: "mine" });
  const clearing = clearSearches("me");
  await flush();
  resetSearchHistorySession();
  useAuthStore.setState({ user: { id: "someone-else", name: "b" } as never });
  finish({ id: "server-6", usedAt: "2026-10-13T11:00:00Z" });
  await saving;
  await expect(clearing).rejects.toThrow("account changed");
  expect(api.clear).not.toHaveBeenCalled();
});
