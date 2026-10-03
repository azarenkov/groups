import React from "react";
import { act, render } from "@testing-library/react";

import { PULL_THRESHOLD, REFRESH_TIMEOUT_MS, usePullToRefresh } from "../usePullToRefresh";

let renders = 0;

const Harness: React.FC<{ onRefresh: () => Promise<void>; convId?: string }> = ({ onRefresh, convId = "c1" }) => {
  renders += 1;
  const bodyRef = React.useRef<HTMLDivElement>(null);
  const pull = usePullToRefresh({ bodyRef, enabled: true, convId, onRefresh });
  return (
    <>
      <div ref={pull.indicatorRef} className="pull-refresh">
        <span />
      </div>
      <div
        ref={bodyRef}
        data-testid="body"
        onTouchStart={pull.onPullTouchStart}
        onTouchMove={pull.onPullTouchMove}
        onTouchEnd={pull.onPullTouchEnd}
        onTouchCancel={pull.onPullTouchCancel}
      />
    </>
  );
};

const body = () => document.querySelector<HTMLDivElement>("[data-testid=body]")!;
const indicator = () => document.querySelector<HTMLDivElement>(".pull-refresh")!;

const touch = (type: "touchStart" | "touchMove", y: number) => {
  const event = new TouchEvent(type.toLowerCase(), {
    bubbles: true,
    touches: [{ clientY: y } as Touch],
  });
  act(() => {
    body().dispatchEvent(event);
  });
};

const frame = () =>
  act(() => {
    jest.advanceTimersByTime(20);
  });

beforeEach(() => {
  jest.useFakeTimers();
  renders = 0;
});

afterEach(() => {
  jest.useRealTimers();
});

const mount = (onRefresh: () => Promise<void>) => {
  const view = render(<Harness onRefresh={onRefresh} />);
  Object.defineProperty(body(), "scrollHeight", { configurable: true, value: 500 });
  Object.defineProperty(body(), "clientHeight", { configurable: true, value: 500 });
  return view;
};

const pullAndRelease = () => {
  touch("touchStart", 400);
  touch("touchMove", 200);
  act(() => {
    body().dispatchEvent(new TouchEvent("touchend", { bubbles: true }));
  });
  frame();
};

const outlast = () =>
  act(async () => {
    jest.advanceTimersByTime(REFRESH_TIMEOUT_MS);
    await Promise.resolve();
    await Promise.resolve();
  });

it("follows the finger without re-rendering and springs back when the touch is cancelled", () => {
  mount(() => Promise.resolve());
  const before = renders;
  touch("touchStart", 400);
  touch("touchMove", 360);
  frame();
  expect(body().style.transform).toBe("translateY(-18px)");
  expect(renders).toBe(before);
  act(() => {
    body().dispatchEvent(new TouchEvent("touchcancel", { bubbles: true }));
  });
  frame();
  expect(body().style.transform).toBe("");
  expect(indicator().style.opacity).toBe("0");
});

it("stops spinning when a refresh never answers", async () => {
  const onRefresh = jest.fn(() => new Promise<void>(() => undefined));
  mount(onRefresh);
  touch("touchStart", 400);
  touch("touchMove", 200);
  act(() => {
    body().dispatchEvent(new TouchEvent("touchend", { bubbles: true }));
  });
  frame();
  expect(onRefresh).toHaveBeenCalledTimes(1);
  expect(indicator().classList.contains("refreshing")).toBe(true);
  expect(body().style.transform).toBe(`translateY(${-PULL_THRESHOLD}px)`);
  await act(async () => {
    jest.advanceTimersByTime(REFRESH_TIMEOUT_MS);
    await Promise.resolve();
    await Promise.resolve();
  });
  frame();
  expect(indicator().classList.contains("refreshing")).toBe(false);
  expect(body().style.transform).toBe("");
});

it("does not start another refresh while a slow one is still running", async () => {
  let finish: () => void = () => undefined;
  const onRefresh = jest.fn(() => new Promise<void>((resolve) => (finish = resolve)));
  mount(onRefresh);
  const pullAndRelease = async () => {
    touch("touchStart", 400);
    touch("touchMove", 200);
    act(() => {
      body().dispatchEvent(new TouchEvent("touchend", { bubbles: true }));
    });
    frame();
  };
  await pullAndRelease();
  await act(async () => {
    jest.advanceTimersByTime(REFRESH_TIMEOUT_MS);
    await Promise.resolve();
    await Promise.resolve();
  });
  frame();
  await pullAndRelease();
  expect(onRefresh).toHaveBeenCalledTimes(1);
  expect(body().style.transform).toBe("");
  await act(async () => {
    finish();
    await Promise.resolve();
    await Promise.resolve();
  });
  await pullAndRelease();
  expect(onRefresh).toHaveBeenCalledTimes(2);
});

it("keeps a slow chat locked after refreshing another chat", async () => {
  const pending: Record<string, () => void> = {};
  let current = "c1";
  const onRefresh = jest.fn(() => new Promise<void>((resolve) => (pending[current] = resolve)));
  const { rerender } = mount(onRefresh);
  pullAndRelease();
  await outlast();
  frame();

  current = "c2";
  rerender(<Harness onRefresh={onRefresh} convId="c2" />);
  frame();
  pullAndRelease();
  expect(onRefresh).toHaveBeenCalledTimes(2);
  await act(async () => {
    pending.c2();
    await Promise.resolve();
    await Promise.resolve();
  });
  await outlast();
  frame();

  current = "c1";
  rerender(<Harness onRefresh={onRefresh} convId="c1" />);
  frame();
  pullAndRelease();
  expect(onRefresh).toHaveBeenCalledTimes(2);
});

