import React from "react";
import { act, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import UserProfileModal from "../UserProfileModal";
import { useProfileModalStore } from "../../store/profileModalStore";
import { homeAPI } from "../../../home/api/home";

jest.mock("../../../../shared/lib/avatarCache", () => ({ useAvatarUrl: () => null }));
jest.mock("../../../home/api/home", () => ({ homeAPI: { profile: jest.fn() } }));
jest.mock("../../api/user", () => ({ userAPI: { userConnections: () => Promise.resolve([]) } }));
jest.mock("../../store/activityStore", () => {
  const actual = jest.requireActual("../../store/activityStore");
  return { ...actual, loadActivitiesFor: () => Promise.resolve() };
});

const profile = homeAPI.profile as jest.Mock;

const publicProfile = (name: string) => ({
  id: "u2",
  name,
  bio: "",
  profileUpdatedAt: null,
  active: true,
  timezone: "",
  memberSince: null,
  acceptsCalls: true,
  acceptsMessages: true,
  acceptsFriendRequests: true,
  blocked: false,
  blockedBy: false,
  mutualCommunities: [],
  voiceRoom: null,
});

it("shows a profile opened before right away while refreshing it", async () => {
  await i18n.changeLanguage("en");
  profile.mockResolvedValueOnce(publicProfile("Bobby")).mockImplementation(() => new Promise(() => undefined));
  render(<UserProfileModal onOpenDm={jest.fn()} onCall={jest.fn()} onRemoveFriend={jest.fn()} onBlockUser={jest.fn()} />);
  await act(async () => {
    useProfileModalStore.getState().open("u2");
  });
  await screen.findByText("Bobby");
  act(() => useProfileModalStore.getState().close());
  expect(screen.queryByText("Bobby")).toBeNull();
  act(() => useProfileModalStore.getState().open("u2"));
  expect(screen.getByText("Bobby")).toBeTruthy();
  expect(profile).toHaveBeenCalledTimes(2);
  act(() => useProfileModalStore.getState().close());
});
