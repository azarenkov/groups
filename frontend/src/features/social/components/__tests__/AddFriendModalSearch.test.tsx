import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AddFriendModal from "../AddFriendModal";
import { friendshipAPI } from "../../api/friendship";

jest.mock("../../api/friendship", () => ({ friendshipAPI: { searchUsers: jest.fn() } }));

const searchUsers = friendshipAPI.searchUsers as jest.Mock;

it("never shows results of a search the user already typed past", async () => {
  await i18n.changeLanguage("en");
  jest.useFakeTimers();
  const pending: Record<string, (users: { id: string; name: string }[]) => void> = {};
  searchUsers.mockImplementation((q: string) => new Promise((resolve) => (pending[q] = resolve)));
  render(<AddFriendModal isOpen onClose={jest.fn()} />);
  const input = screen.getByPlaceholderText(i18n.t("modals:addFriend.placeholder"));
  fireEvent.change(input, { target: { value: "al" } });
  await act(async () => {
    jest.advanceTimersByTime(200);
  });
  fireEvent.change(input, { target: { value: "bo" } });
  await act(async () => {
    pending.al?.([{ id: "u1", name: "alice" }]);
    for (let i = 0; i < 5; i += 1) {
      await Promise.resolve();
      jest.advanceTimersByTime(1);
    }
  });
  expect(screen.queryByText("alice")).toBeNull();
  await act(async () => {
    jest.advanceTimersByTime(200);
  });
  await act(async () => {
    pending.bo?.([{ id: "u2", name: "bob" }]);
    await Promise.resolve();
  });
  expect(await screen.findByText("bob")).toBeTruthy();
  jest.useRealTimers();
});
