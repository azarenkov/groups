import React from "react";
import { act, render, screen } from "@testing-library/react";

import { useDialogFocus } from "../useDialogFocus";

const Dialog = ({ open }: { open: boolean }) => {
  const ref = React.useRef<HTMLDivElement>(null);
  useDialogFocus(ref, open);
  if (!open) return null;
  return (
    <div ref={ref} role="dialog">
      <button type="button">first</button>
    </div>
  );
};

const Page = ({ open }: { open: boolean }) => (
  <>
    <button type="button">opener</button>
    <Dialog open={open} />
  </>
);

describe("useDialogFocus", () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it("focuses the dialog on open and gives focus back on close", () => {
    const { rerender } = render(<Page open={false} />);
    const opener = screen.getByRole("button", { name: "opener" });
    opener.focus();
    rerender(<Page open />);
    act(() => {
      jest.runOnlyPendingTimers();
    });
    expect(screen.getByRole("button", { name: "first" })).toHaveFocus();
    rerender(<Page open={false} />);
    expect(opener).toHaveFocus();
  });
});
