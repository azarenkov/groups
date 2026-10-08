import { ApiError } from "../../lib/apiError";
import { QUERY_MAX_RETRIES, installQuerySync, queryClient, shouldRetryQuery } from "../queryClient";

const openListeners: (() => void)[] = [];
jest.mock("../../ws/websocket", () => ({
  ws: {
    onOpen: (fn: () => void) => {
      openListeners.push(fn);
      return () => undefined;
    },
  },
}));

it("retries network and server errors but not client errors", () => {
  expect(shouldRetryQuery(0, new Error("network"))).toBe(true);
  expect(shouldRetryQuery(0, new ApiError("x", 503))).toBe(true);
  expect(shouldRetryQuery(0, new ApiError("x", 429))).toBe(true);
  expect(shouldRetryQuery(0, new ApiError("x", 403))).toBe(false);
  expect(shouldRetryQuery(QUERY_MAX_RETRIES, new Error("network"))).toBe(false);
});

it("refreshes every active query after the socket reconnects", () => {
  const spy = jest.spyOn(queryClient, "invalidateQueries").mockResolvedValue();
  installQuerySync();
  openListeners.forEach((fn) => fn());
  expect(spy).toHaveBeenCalledWith();
  spy.mockRestore();
});
