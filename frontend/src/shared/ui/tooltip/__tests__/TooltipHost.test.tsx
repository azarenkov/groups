import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import TooltipHost, { TOOLTIP_DELAY_MS, positionTip } from "../TooltipHost";

const view = { width: 1000, height: 800 };
const rect = (left: number, top: number, width: number, height: number) =>
  ({ left, top, width, height, right: left + width, bottom: top + height }) as DOMRect;

beforeEach(() => jest.useFakeTimers());
afterEach(() => jest.useRealTimers());

const setup = () =>
  render(
    <>
      <TooltipHost />
      <button type="button" title="Add friend">
        +
      </button>
      <button type="button" title="Inbox" />
    </>,
  );

it("shows a styled tooltip quickly instead of the native title", () => {
  setup();
  const button = screen.getByText("+");
  fireEvent.pointerOver(button);
  expect(button.hasAttribute("title")).toBe(false);
  expect(screen.queryByRole("tooltip")).toBeNull();
  act(() => jest.advanceTimersByTime(TOOLTIP_DELAY_MS));
  expect(screen.getByRole("tooltip").textContent).toBe("Add friend");
});

it("restores the title when the pointer leaves", () => {
  setup();
  const button = screen.getByText("+");
  fireEvent.pointerOver(button);
  act(() => jest.advanceTimersByTime(TOOLTIP_DELAY_MS));
  fireEvent.pointerOut(button, { relatedTarget: document.body });
  expect(screen.queryByRole("tooltip")).toBeNull();
  expect(button.getAttribute("title")).toBe("Add friend");
});

it("switches instantly between neighbouring buttons", () => {
  setup();
  const add = screen.getByText("+");
  const inbox = screen.getByTitle("Inbox");
  fireEvent.pointerOver(add);
  act(() => jest.advanceTimersByTime(TOOLTIP_DELAY_MS));
  fireEvent.pointerOut(add, { relatedTarget: inbox });
  fireEvent.pointerOver(inbox);
  expect(screen.getByRole("tooltip").textContent).toBe("Inbox");
});

it("keeps an icon-only button named while its title is borrowed", () => {
  setup();
  const inbox = screen.getByTitle("Inbox");
  fireEvent.pointerOver(inbox);
  expect(inbox.getAttribute("aria-label")).toBe("Inbox");
  fireEvent.pointerOut(inbox, { relatedTarget: document.body });
  expect(inbox.hasAttribute("aria-label")).toBe(false);
  expect(inbox.getAttribute("title")).toBe("Inbox");
});

it("hides on click", () => {
  setup();
  const button = screen.getByText("+");
  fireEvent.pointerOver(button);
  act(() => jest.advanceTimersByTime(TOOLTIP_DELAY_MS));
  fireEvent.pointerDown(button);
  expect(screen.queryByRole("tooltip")).toBeNull();
});

it("places the tooltip under the element and flips it when there is no room", () => {
  expect(positionTip(rect(100, 100, 40, 40), { width: 60, height: 28 }, "bottom", view)).toEqual({
    top: 148,
    left: 90,
    placement: "bottom",
  });
  expect(positionTip(rect(100, 760, 40, 30), { width: 60, height: 28 }, "bottom", view).placement).toBe("top");
});

it("places rail tooltips to the right and keeps them on screen", () => {
  expect(positionTip(rect(20, 100, 48, 48), { width: 120, height: 28 }, "right", view)).toEqual({
    top: 110,
    left: 76,
    placement: "right",
  });
  expect(positionTip(rect(0, 100, 10, 10), { width: 200, height: 28 }, "bottom", view).left).toBe(8);
});
