import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import { AccountSection } from "../AccountSection";
import { useAuthStore } from "../../../../auth/store/authStore";

jest.mock("../../../../../shared/lib/avatarCache", () => ({
  useAvatarUrl: () => null,
  bustAvatar: () => undefined,
}));

const ID = "0b6f3a1e-6a51-4c7e-9a0e-5f1b2c3d4e5f";

beforeEach(() => {
  useAuthStore.setState({ user: { id: ID, name: "me", email: "me@rift.app" } } as never);
});

it("shows the profile link and copies it", async () => {
  await i18n.changeLanguage("en");
  const writeText = jest.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  render(<AccountSection onAccountGone={jest.fn()} />);
  const link = `${window.location.origin}/u/${ID}`;
  expect(screen.getByText(link)).toBeInTheDocument();
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("settings:account.copyProfileLink") }));
  });
  expect(writeText).toHaveBeenCalledWith(link);
  expect(screen.getByRole("button", { name: i18n.t("settings:account.profileLinkCopied") })).toBeInTheDocument();
});

it("says so when the link cannot be copied", async () => {
  await i18n.changeLanguage("en");
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: undefined });
  const exec = jest.fn().mockReturnValue(false);
  Object.defineProperty(document, "execCommand", { configurable: true, value: exec });
  render(<AccountSection onAccountGone={jest.fn()} />);
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("settings:account.copyProfileLink") }));
  });
  expect(screen.getByText(i18n.t("settings:account.profileLinkError"))).toBeInTheDocument();
});
