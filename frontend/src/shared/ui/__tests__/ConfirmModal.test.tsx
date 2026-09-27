import { render, screen } from "@testing-library/react";

import ConfirmModal from "../ConfirmModal";

const Page = ({ open }: { open: boolean }) => (
  <>
    <button type="button">row</button>
    {open && (
      <ConfirmModal title="Delete" confirmLabel="Delete" onCancel={jest.fn()} onConfirm={jest.fn()} />
    )}
  </>
);

describe("ConfirmModal", () => {
  it("focuses Cancel and gives focus back to the opener when it closes", () => {
    const { rerender } = render(<Page open={false} />);
    const row = screen.getByRole("button", { name: "row" });
    row.focus();
    rerender(<Page open />);
    expect(screen.getByRole("button", { name: "cancel" })).toHaveFocus();
    rerender(<Page open={false} />);
    expect(row).toHaveFocus();
  });
});
