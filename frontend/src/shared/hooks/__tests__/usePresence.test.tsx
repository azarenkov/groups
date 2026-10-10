import { act, renderHook } from "@testing-library/react";

import { usePresence } from "../usePresence";

const setMotion = (reduced: boolean) => {
  window.matchMedia = ((query: string) => ({ matches: reduced && query.includes("reduce") })) as unknown as typeof window.matchMedia;
};

afterEach(() => {
  jest.useRealTimers();
  delete (window as { matchMedia?: unknown }).matchMedia;
});

it("plays enter and exit before unmounting", () => {
  jest.useFakeTimers();
  setMotion(false);
  const { result, rerender } = renderHook(({ open }) => usePresence(open, 200), { initialProps: { open: false } });
  expect(result.current).toBeNull();
  rerender({ open: true });
  expect(result.current).toBe("enter");
  act(() => jest.advanceTimersByTime(200));
  expect(result.current).toBe("shown");
  rerender({ open: false });
  expect(result.current).toBe("exit");
  act(() => jest.advanceTimersByTime(200));
  expect(result.current).toBeNull();
});

it("switches at once when motion is reduced", () => {
  setMotion(true);
  const { result, rerender } = renderHook(({ open }) => usePresence(open, 200), { initialProps: { open: true } });
  expect(result.current).toBe("shown");
  rerender({ open: false });
  expect(result.current).toBeNull();
});
