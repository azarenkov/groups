import React from "react";
import { render } from "@testing-library/react";

import { useAutoGrow } from "../useAutoGrow";

let notify: (() => void) | null = null;

class FakeResizeObserver {
  constructor(callback: () => void) {
    notify = callback;
  }
  observe() {}
  disconnect() {}
}

const Field = () => {
  const ref = React.useRef<HTMLTextAreaElement>(null);
  useAutoGrow(ref, "some text");
  return <textarea ref={ref} data-testid="field" />;
};

describe("useAutoGrow", () => {
  const original = (globalThis as { ResizeObserver?: unknown }).ResizeObserver;
  beforeEach(() => {
    (globalThis as { ResizeObserver?: unknown }).ResizeObserver = FakeResizeObserver;
  });
  afterEach(() => {
    (globalThis as { ResizeObserver?: unknown }).ResizeObserver = original;
    notify = null;
  });

  it("refits the height when the width changes, not when only the height does", () => {
    let width = 300;
    const { getByTestId } = render(<Field />);
    const field = getByTestId("field") as HTMLTextAreaElement;
    Object.defineProperty(field, "clientWidth", { get: () => width, configurable: true });
    notify?.();
    field.style.height = "123px";
    notify?.();
    expect(field.style.height).toBe("123px");
    width = 200;
    notify?.();
    expect(field.style.height).not.toBe("123px");
  });
});
