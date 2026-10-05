import React from "react";
import { act, render } from "@testing-library/react";

import { PULL_THRESHOLD, REFRESH_TIMEOUT_MS, resist, springStep, usePullToRefresh } from "../usePullToRefresh";

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
        onWheel={pull.onPullWheel}
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

const settleAll = () =>
  act(() => {
    jest.advanceTimersByTime(2000);
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
  expect(body().style.transform).toBe(`translate3d(0, ${-resist(40 * 0.9)}px, 0)`);
  expect(body().style.willChange).toBe("transform");
  expect(renders).toBe(before);
  act(() => {
    body().dispatchEvent(new TouchEvent("touchcancel", { bubbles: true }));
  });
  frame();
  expect(body().style.transform).not.toBe("");
  settleAll();
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
  settleAll();
  expect(body().style.transform).toBe(`translate3d(0, ${-PULL_THRESHOLD}px, 0)`);
  await act(async () => {
    jest.advanceTimersByTime(REFRESH_TIMEOUT_MS);
    await Promise.resolve();
    await Promise.resolve();
  });
  settleAll();
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
  settleAll();
  await pullAndRelease();
  expect(onRefresh).toHaveBeenCalledTimes(1);
  settleAll();
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

it("eases the pull towards its limit", () => {
  expect(resist(0)).toBe(0);
  expect(resist(50)).toBeLessThan(50);
  expect(resist(10_000)).toBeLessThanOrEqual(96);
  expect(resist(200)).toBeGreaterThan(resist(100));
});


const wheel = (deltaY: number) =>
  act(() => {
    const event = new WheelEvent("wheel", { bubbles: true, deltaY });
    Object.defineProperty(event, "timeStamp", { value: Date.now() });
    body().dispatchEvent(event);
  });

it("does not let trackpad momentum start a second pull after a refresh", async () => {
  const onRefresh = jest.fn(() => Promise.resolve());
  mount(onRefresh);
  for (let i = 0; i < 10; i += 1) {
    wheel(40);
    act(() => {
      jest.advanceTimersByTime(16);
    });
  }
  expect(onRefresh).toHaveBeenCalledTimes(1);
  for (let i = 0; i < 80; i += 1) {
    wheel(8);
    await act(async () => {
      jest.advanceTimersByTime(16);
      await Promise.resolve();
    });
  }
  frame();
  expect(indicator().classList.contains("refreshing")).toBe(false);
  expect(body().style.transform).toBe("");
  expect(onRefresh).toHaveBeenCalledTimes(1);
  act(() => {
    jest.advanceTimersByTime(500);
  });
  wheel(40);
  frame();
  const shown = -parseFloat(body().style.transform.replace("translate3d(0, ", ""));
  expect(shown).toBeGreaterThan(0);
  expect(shown).toBeLessThan(resist(40 * 0.7));
});

it("springs smoothly to its target without overshooting", () => {
  let position = 0;
  let velocity = 0;
  let previous = 0;
  let steps = 0;
  for (; steps < 200; steps += 1) {
    const next = springStep(position, velocity, PULL_THRESHOLD, 1 / 60);
    expect(next.position).toBeGreaterThanOrEqual(previous);
    expect(next.position).toBeLessThanOrEqual(PULL_THRESHOLD);
    previous = next.position;
    position = next.position;
    velocity = next.velocity;
    if (next.done) break;
  }
  expect(position).toBe(PULL_THRESHOLD);
  expect(steps).toBeGreaterThan(10);
  expect(steps).toBeLessThan(60);
});

it("eases back even when the finger lifts before the last move was painted", () => {
  mount(() => Promise.resolve());
  touch("touchStart", 400);
  touch("touchMove", 360);
  act(() => {
    body().dispatchEvent(new TouchEvent("touchend", { bubbles: true }));
  });
  frame();
  const shown = -parseFloat(body().style.transform.replace("translate3d(0, ", ""));
  expect(shown).toBeGreaterThan(0);
  expect(shown).toBeLessThan(resist(40 * 0.9));
  settleAll();
  expect(body().style.transform).toBe("");
});

it("jumps straight back without animating when reduced motion is on", () => {
  document.documentElement.classList.add("reduce-motion");
  try {
    mount(() => Promise.resolve());
    touch("touchStart", 400);
    touch("touchMove", 360);
    frame();
    act(() => {
      body().dispatchEvent(new TouchEvent("touchcancel", { bubbles: true }));
    });
    frame();
    expect(body().style.transform).toBe("");
  } finally {
    document.documentElement.classList.remove("reduce-motion");
  }
});
