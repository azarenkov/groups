import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import UserProfileModal from "../UserProfileModal";
import { useProfileModalStore } from "../../store/profileModalStore";

jest.mock("../../../../shared/lib/avatarCache", () => ({
  useAvatarUrl: (kind: string, id: string | null) => (kind === "user" && id ? `blob:${id}` : null),
}));

jest.mock("../../../home/api/home", () => ({
  homeAPI: { profile: () => new Promise(() => undefined) },
}));

jest.mock("../../api/user", () => ({
  userAPI: { userConnections: () => Promise.resolve([]) },
}));

jest.mock("../../store/activityStore", () => {
  const actual = jest.requireActual("../../store/activityStore");
  return { ...actual, loadActivitiesFor: () => Promise.resolve() };
});

describe("UserProfileModal avatar viewer", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  const renderModal = () =>
    render(
      <UserProfileModal
        onOpenDm={jest.fn()}
        onCall={jest.fn()}
        onRemoveFriend={jest.fn()}
        onBlockUser={jest.fn()}
      />,
    );

  it("closes only the avatar viewer on Escape, then the profile", () => {
    act(() => useProfileModalStore.getState().open("u2"));
    renderModal();
    fireEvent.click(screen.getByLabelText(i18n.t("profile:openAvatar")));
    expect(document.querySelector(".profile-avatar-full")).not.toBeNull();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(document.querySelector(".profile-avatar-full")).toBeNull();
    expect(useProfileModalStore.getState().userId).toBe("u2");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(useProfileModalStore.getState().userId).toBeNull();
  });

  it("does not reopen the viewer for the next profile", () => {
    act(() => useProfileModalStore.getState().open("u2"));
    renderModal();
    fireEvent.click(screen.getByLabelText(i18n.t("profile:openAvatar")));
    act(() => useProfileModalStore.getState().open("u3"));
    expect(document.querySelector(".profile-avatar-full")).toBeNull();
  });
});
