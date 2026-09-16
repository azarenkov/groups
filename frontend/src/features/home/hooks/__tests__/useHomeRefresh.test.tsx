import { renderHook } from "@testing-library/react";

import { useHomeRefresh } from "../useHomeRefresh";

const mockScheduleHomeRefresh = jest.fn();
const mockFetchHome = jest.fn();

jest.mock("../../store/homeStore", () => ({
  scheduleHomeRefresh: (...args: unknown[]) => mockScheduleHomeRefresh(...args),
  useHomeStore: (selector: (state: { fetch: () => void }) => unknown) =>
    selector({ fetch: () => mockFetchHome() }),
}));

describe("useHomeRefresh", () => {
  beforeEach(() => {
    mockScheduleHomeRefresh.mockClear();
    mockFetchHome.mockClear();
  });

  it("ignores renders that repeat the same watched values", () => {
    const presence = { a: [] };
    const online = { b: true };
    const friends = [{ id: "1" }];
    const { rerender } = renderHook(
      (props: { watched: unknown[] }) =>
        useHomeRefresh({ hasSession: false, homeActive: true, ...props }),
      { initialProps: { watched: [presence, online, friends] } },
    );

    rerender({ watched: [presence, online, friends] });
    rerender({ watched: [presence, online, friends] });

    expect(mockScheduleHomeRefresh).not.toHaveBeenCalled();
  });

  it("schedules a refresh when a watched value changes", () => {
    const presence = { a: [] };
    const online = { b: true };
    const friends = [{ id: "1" }];
    const { rerender } = renderHook(
      (props: { watched: unknown[] }) =>
        useHomeRefresh({ hasSession: false, homeActive: true, ...props }),
      { initialProps: { watched: [presence, online, friends] } },
    );

    rerender({ watched: [presence, { b: false }, friends] });

    expect(mockScheduleHomeRefresh).toHaveBeenCalledTimes(1);
  });

  it("stays quiet while the home view is closed", () => {
    const { rerender } = renderHook(
      (props: { watched: unknown[]; homeActive: boolean }) =>
        useHomeRefresh({ hasSession: false, ...props }),
      { initialProps: { watched: [{ a: 1 }], homeActive: false } },
    );

    rerender({ watched: [{ a: 2 }], homeActive: false });

    expect(mockScheduleHomeRefresh).not.toHaveBeenCalled();
  });
});
