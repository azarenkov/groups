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

  it("shows just the number for the number-only style", () => {
    render(<ProBadge tier="pro" badgeStyle="number_only" memberNumber={42} />);
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.queryByText("PRO")).toBeNull();
    expect(screen.getByTitle("Rift Pro #42")).toBeInTheDocument();
  });

  it("falls back to a regular badge when the number is not known yet", () => {
    render(<ProBadge tier="pro" badgeStyle="number_only" memberNumber={null} />);
    expect(screen.getByText("PRO")).toBeInTheDocument();
    expect(document.querySelector(".pro-badge-tint")).not.toBeNull();
  });

  it("shows gold only on a Premium badge", () => {
    const { rerender } = render(<ProBadge tier="premium" badgeStyle="gold" />);
    expect(document.querySelector(".pro-badge-gold")).toHaveTextContent("PREMIUM");
    rerender(<ProBadge tier="pro" badgeStyle="gold" />);
    expect(document.querySelector(".pro-badge-gold")).toBeNull();
    expect(document.querySelector(".pro-badge-tint")).toHaveTextContent("PRO");
  });
});
