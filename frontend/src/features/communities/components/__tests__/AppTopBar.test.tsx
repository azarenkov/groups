import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppTopBar from "../AppTopBar";

const renderBar = (overrides: Partial<React.ComponentProps<typeof AppTopBar>> = {}) => {
  const props: React.ComponentProps<typeof AppTopBar> = {
    navState: { back: true, forward: false },
    goHistory: jest.fn(),
    homeActive: false,
    onGoHome: jest.fn(),
    query: "",
    onQueryChange: jest.fn(),
    currentUserId: "me",
    currentUserName: "Azarenkov",
    currentUserStatus: "online",
    onOpenSettings: jest.fn(),
    onSetStatus: jest.fn(),
    ...overrides,
  };
  render(<AppTopBar {...props} />);
  return props;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("navigates back, forward and home from the top", () => {
  const p = renderBar();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("common:back") }));
  expect(p.goHistory).toHaveBeenCalledWith(-1);
  expect(screen.getByRole("button", { name: i18n.t("common:forward") })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:home") }));
  expect(p.onGoHome).toHaveBeenCalled();
});

it("searches from the centre and clears with Escape", () => {
  const p = renderBar({ query: "bob" });
  const input = screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholder") });
  fireEvent.change(input, { target: { value: "bobby" } });
  expect(p.onQueryChange).toHaveBeenCalledWith("bobby");
  fireEvent.keyDown(input, { key: "Escape" });
  expect(p.onQueryChange).toHaveBeenLastCalledWith("");
});

const openMenu = () =>
  fireEvent.click(screen.getByRole("button", { name: `Azarenkov, ${i18n.t("sidebar:accountMenu")}` }));

it("opens settings and sets the status from the account menu", () => {
  const p = renderBar();
  openMenu();
  fireEvent.click(screen.getByRole("menuitem", { name: i18n.t("sidebar:settings") }));
  expect(p.onOpenSettings).toHaveBeenCalled();
  expect(screen.queryByRole("menu")).toBeNull();
  openMenu();
  fireEvent.click(screen.getByRole("menuitemradio", { name: i18n.t("sidebar:statusOffline") }));
  expect(p.onSetStatus).toHaveBeenCalledWith("offline");
});

it("asks before logging out from the account menu", () => {
  const onSignOut = jest.fn();
  renderBar({ onSignOut });
  openMenu();
  fireEvent.click(screen.getByRole("menuitem", { name: i18n.t("sidebar:logOut") }));
  expect(onSignOut).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:logOut") }));
  expect(onSignOut).toHaveBeenCalled();
});
