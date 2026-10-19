import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppRail from "../AppRail";

const renderRail = (overrides: Partial<React.ComponentProps<typeof AppRail>> = {}) => {
  const props: React.ComponentProps<typeof AppRail> = {
    homeActive: true,
    groupUnread: 0,
    onGoHome: jest.fn(),
    onOpenGroups: jest.fn(),
    onOpenSettings: jest.fn(),
    ...overrides,
  };
  render(<AppRail {...props} />);
  return props;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("switches between home, groups and settings", () => {
  const props = renderRail();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:home") }));
  expect(props.onGoHome).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:tabCommunities") }));
  expect(props.onOpenGroups).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:settings") }));
  expect(props.onOpenSettings).toHaveBeenCalled();
});

it("marks groups current outside home and shows the unread count", () => {
  renderRail({ homeActive: false, groupUnread: 120 });
  expect(screen.getByRole("button", { name: /Groups/ })).toHaveAttribute("aria-current", "page");
  expect(screen.getByRole("button", { name: i18n.t("sidebar:home") })).not.toHaveAttribute("aria-current");
  expect(screen.getByText("99+")).toBeInTheDocument();
});

it("highlights home while at home", () => {
  renderRail({ homeActive: true });
  expect(screen.getByRole("button", { name: i18n.t("sidebar:home") })).toHaveAttribute("aria-current", "page");
  expect(screen.getByRole("button", { name: i18n.t("sidebar:tabCommunities") })).not.toHaveAttribute("aria-current");
});
