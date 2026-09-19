import { render, screen } from "@testing-library/react";

import ProBadge from "../ProBadge";

describe("ProBadge", () => {
  it("shows the member number only for the numbered style", () => {
    const { rerender } = render(<ProBadge tier="pro" badgeStyle="numbered" memberNumber={7} />);
    expect(screen.getByText("07")).toBeInTheDocument();
    rerender(<ProBadge tier="pro" badgeStyle="numbered" memberNumber={1234} />);
    expect(screen.getByText("1234")).toBeInTheDocument();
    rerender(<ProBadge tier="pro" badgeStyle="tint" memberNumber={7} />);
    expect(screen.queryByText("07")).toBeNull();
  });

  it("stays a plain badge until the number is known", () => {
    render(<ProBadge tier="premium" badgeStyle="numbered" memberNumber={null} />);
    expect(screen.getByText("PREMIUM")).toBeInTheDocument();
    expect(document.querySelector(".pro-badge-count")).toBeNull();
  });
});
