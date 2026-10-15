import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import SettingsView from "../SettingsView";

jest.mock("../sections/AccountSection", () => ({ AccountSection: () => <div>account</div> }));

const renderSettings = (onClose = jest.fn(), onSignOut = jest.fn()) => {
  render(<SettingsView open section="account" onSection={jest.fn()} onClose={onClose} onSignOut={onSignOut} />);
  return { onClose, onSignOut };
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

const openConfirm = () => fireEvent.click(screen.getByRole("button", { name: i18n.t("settings:logOut") }));

it("asks before logging out and logs out only after confirming", () => {
  const { onSignOut } = renderSettings();
  openConfirm();
  expect(screen.getByText(i18n.t("settings:logOutConfirmTitle"))).toBeInTheDocument();
  expect(onSignOut).not.toHaveBeenCalled();
  const buttons = screen.getAllByRole("button", { name: i18n.t("settings:logOut") });
  fireEvent.click(buttons[buttons.length - 1]);
  expect(onSignOut).toHaveBeenCalledTimes(1);
});

it("closes only the confirmation on Escape and keeps the session", () => {
  const { onClose, onSignOut } = renderSettings();
  openConfirm();
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByText(i18n.t("settings:logOutConfirmTitle"))).toBeNull();
  expect(onClose).not.toHaveBeenCalled();
  expect(onSignOut).not.toHaveBeenCalled();
});
