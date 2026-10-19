import React from "react";
import { act, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import UserProfileModal from "../UserProfileModal";
import { useProfileModalStore } from "../../store/profileModalStore";
import { homeAPI } from "../../../home/api/home";
import { useAuthStore } from "../../../auth/store/authStore";
import { useUserProfileStore } from "../../store/userProfileStore";

jest.mock("../../../../shared/lib/avatarCache", () => ({ useAvatarUrl: () => null }));
jest.mock("../../../home/api/home", () => ({ homeAPI: { profile: jest.fn() } }));

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

beforeEach(() => {
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
});

it("shows a profile opened before right away while refreshing it", async () => {
  await i18n.changeLanguage("en");
  profile.mockResolvedValueOnce(publicProfile("Bobby")).mockImplementation(() => new Promise(() => undefined));
  render(<UserProfileModal onOpenDm={jest.fn()} onCall={jest.fn()} onRemoveFriend={jest.fn()} onBlockUser={jest.fn()} onEditProfile={jest.fn()} />);
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

it("drops a profile answer that arrives after signing out", async () => {
  let finish: (v: unknown) => void = () => undefined;
  profile.mockReset();
  profile.mockImplementation(() => new Promise((resolve) => (finish = resolve)));
  render(<UserProfileModal onOpenDm={jest.fn()} onCall={jest.fn()} onRemoveFriend={jest.fn()} onBlockUser={jest.fn()} onEditProfile={jest.fn()} />);
  await act(async () => {
    useProfileModalStore.getState().open("u9");
  });
  act(() => useAuthStore.setState({ user: null } as never));
  await act(async () => {
    finish({ ...publicProfile("Leak"), id: "u9", bio: "old account bio", profileUpdatedAt: "2026-10-01T00:00:00Z" });
    await Promise.resolve();
  });
  expect(useUserProfileStore.getState().bios.u9).toBeUndefined();
  act(() => useProfileModalStore.getState().close());
});
