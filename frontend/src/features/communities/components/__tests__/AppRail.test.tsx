import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppRail from "../AppRail";

const renderRail = (overrides: Partial<React.ComponentProps<typeof AppRail>> = {}) => {
  const props: React.ComponentProps<typeof AppRail> = {
    tab: "dms",
    homeActive: false,
    dmUnread: 0,
    groupUnread: 0,
    onGoHome: jest.fn(),
    onSetTab: jest.fn(),
    onOpenSettings: jest.fn(),
    ...overrides,
  };
  render(<AppRail {...props} />);
  return props;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("switches between home, chat, groups and settings", () => {
  const props = renderRail();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:home") }));
  expect(props.onGoHome).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:tabCommunities") }));
  expect(props.onSetTab).toHaveBeenCalledWith("communities");
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:tabDirects") }));
  expect(props.onSetTab).toHaveBeenCalledWith("dms");
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:settings") }));
  expect(props.onOpenSettings).toHaveBeenCalled();
});

it("marks the current section and shows unread counts", () => {
  renderRail({ tab: "communities", groupUnread: 120, dmUnread: 3 });
  expect(screen.getByRole("button", { name: /Groups/ })).toHaveAttribute("aria-current", "page");
  expect(screen.getByText("99+")).toBeInTheDocument();
  expect(screen.getByText("3")).toBeInTheDocument();
});

it("highlights home instead of a section while at home", () => {
  renderRail({ homeActive: true });
  expect(screen.getByRole("button", { name: i18n.t("sidebar:home") })).toHaveAttribute("aria-current", "page");
  expect(screen.getByRole("button", { name: i18n.t("sidebar:tabDirects") })).not.toHaveAttribute("aria-current");
});
