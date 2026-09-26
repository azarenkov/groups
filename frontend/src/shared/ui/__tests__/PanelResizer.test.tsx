import { fireEvent, render, screen } from "@testing-library/react";

import PanelResizer from "../PanelResizer";

const setup = () =>
  render(
    <PanelResizer
      cssVar="--w-test"
      otherSelector=".nothing"
      side="start"
      min={240}
      max={420}
      fallback={320}
      storageKey="test.panel"
      label="Resize"
    />,
  );

const width = () =>
  document.documentElement.style.getPropertyValue("--w-test");

describe("PanelResizer", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.style.removeProperty("--w-test");
    Object.defineProperty(window, "innerWidth", { value: 1600, configurable: true });
  });

  it("widens with the arrow keys and remembers the width", () => {
    setup();
    const handle = screen.getByRole("separator", { name: "Resize" });
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(width()).toBe("336px");
    expect(window.localStorage.getItem("test.panel")).toBe("336");
    fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });
    expect(width()).toBe("288px");
  });

  it("stays within its limits", () => {
    setup();
    const handle = screen.getByRole("separator", { name: "Resize" });
    for (let i = 0; i < 20; i += 1) fireEvent.keyDown(handle, { key: "ArrowRight", shiftKey: true });
    expect(width()).toBe("420px");
  });

  it("restores the saved width and resets on double click", () => {
    window.localStorage.setItem("test.panel", "300");
    setup();
    expect(width()).toBe("300px");
    fireEvent.doubleClick(screen.getByRole("separator", { name: "Resize" }));
    expect(width()).toBe("");
    expect(window.localStorage.getItem("test.panel")).toBeNull();
  });

  it("re-clamps the saved width when the window narrows and restores it when it widens", () => {
    window.localStorage.setItem("test.panel", "420");
    setup();
    expect(width()).toBe("420px");
    Object.defineProperty(window, "innerWidth", { value: 700, configurable: true });
    fireEvent(window, new Event("resize"));
    expect(parseFloat(width())).toBeLessThan(420);
    Object.defineProperty(window, "innerWidth", { value: 1600, configurable: true });
    fireEvent(window, new Event("resize"));
    expect(width()).toBe("420px");
  });

  it("shrinks the default width when another panel appears in a narrow window", () => {
    Object.defineProperty(window, "innerWidth", { value: 880, configurable: true });
    const { rerender } = render(
      <PanelResizer
        cssVar="--w-test"
        otherSelector=".other-panel"
        side="start"
        min={240}
        max={420}
        fallback={320}
        storageKey="test.panel"
        label="Resize"
      />,
    );
    expect(width()).toBe("");
    const other = document.createElement("div");
    other.className = "other-panel";
    other.getBoundingClientRect = () => ({ width: 240 }) as DOMRect;
    document.body.appendChild(other);
    rerender(
      <>
        <PanelResizer
          cssVar="--w-test"
          otherSelector=".other-panel"
          side="start"
          min={240}
          max={420}
          fallback={320}
          storageKey="test.panel"
          label="Resize"
        />
        <PanelResizer
          cssVar="--w-other"
          otherSelector=".nothing"
          side="end"
          min={220}
          max={360}
          fallback={240}
          storageKey="test.other"
          label="Resize other"
        />
      </>,
    );
    expect(width()).toBe("240px");
    other.remove();
    document.documentElement.style.removeProperty("--w-other");
  });
});
