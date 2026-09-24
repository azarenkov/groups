import { act, fireEvent, render, screen } from "@testing-library/react";

import ContextMenuHost from "../ContextMenuHost";
import { openContextMenu, useContextMenuStore } from "../contextMenuStore";

const openWith = (onCopy: jest.Mock, onDelete: jest.Mock) =>
  act(() => {
    openContextMenu({ clientX: 10, clientY: 10 }, [
      { kind: "item", id: "copy", label: "Copy", onSelect: onCopy },
      { kind: "separator", id: "s" },
      { kind: "item", id: "delete", label: "Delete", danger: true, onSelect: onDelete },
    ]);
  });

describe("ContextMenuHost", () => {
  beforeEach(() => useContextMenuStore.setState({ menu: null }));

  it("focuses the first item and moves with the arrow keys", () => {
    render(<ContextMenuHost />);
    openWith(jest.fn(), jest.fn());
    const copy = screen.getByRole("menuitem", { name: "Copy" });
    expect(copy).toHaveFocus();
    fireEvent.keyDown(copy, { key: "ArrowDown" });
    expect(screen.getByRole("menuitem", { name: "Delete" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });
    expect(copy).toHaveFocus();
  });

  it("runs the chosen action and closes", () => {
    const onDelete = jest.fn();
    render(<ContextMenuHost />);
    openWith(jest.fn(), onDelete);
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(onDelete).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("closes on Escape and on a click outside", () => {
    render(<ContextMenuHost />);
    openWith(jest.fn(), jest.fn());
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    openWith(jest.fn(), jest.fn());
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
  });
});
