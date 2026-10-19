import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import UserProfileModal from "../UserProfileModal";
import { useProfileModalStore } from "../../store/profileModalStore";
import { useAuthStore } from "../../../auth/store/authStore";

jest.mock("../../../../shared/lib/avatarCache", () => ({
  useAvatarUrl: () => null,
}));

jest.mock("../../../home/api/home", () => ({
  homeAPI: { profile: () => new Promise(() => undefined) },
}));

const SELF = "0b6f3a1e-6a51-4c7e-9a0e-5f1b2c3d4e5f";
const OTHER = "1c7a4b2f-7b62-4d8f-8b1f-6a2c3d4e5f60";

describe("sharing a profile", () => {
  const writeText = jest.fn();
  const onEditProfile = jest.fn();
  const originalMatchMedia = window.matchMedia;

  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  beforeEach(() => {
    writeText.mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    useAuthStore.setState({ user: { id: SELF, name: "me" } as never });
  });

  afterEach(() => {
    act(() => useProfileModalStore.getState().close());
    window.matchMedia = originalMatchMedia;
    Object.defineProperty(navigator, "share", { configurable: true, value: undefined });
  });

  const renderModal = () =>
    render(
      <UserProfileModal
        onOpenDm={jest.fn()}
        onCall={jest.fn()}
        onRemoveFriend={jest.fn()}
        onBlockUser={jest.fn()}
        onEditProfile={onEditProfile}
      />,
    );

  it("copies a link to your own profile and opens the editor", async () => {
    act(() => useProfileModalStore.getState().open(SELF));
    renderModal();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: i18n.t("profile:shareProfile") }));
    });
    expect(writeText).toHaveBeenCalledWith(`${window.location.origin}/u/${SELF}`);
    expect(screen.getByText(i18n.t("profile:linkCopied"))).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: i18n.t("profile:editProfile") }));
    expect(onEditProfile).toHaveBeenCalledTimes(1);
    expect(useProfileModalStore.getState().userId).toBeNull();
  });

  it("copies someone else's profile link from the menu", async () => {
    act(() => useProfileModalStore.getState().open(OTHER));
    renderModal();
    expect(screen.queryByRole("button", { name: i18n.t("profile:editProfile") })).toBeNull();
    fireEvent.click(screen.getByLabelText(i18n.t("profile:more")));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: i18n.t("profile:shareProfile") }));
    });
    expect(writeText).toHaveBeenCalledWith(`${window.location.origin}/u/${OTHER}`);
    expect(screen.getByRole("status")).toHaveTextContent(i18n.t("profile:linkCopied"));
  });

  it("uses the system share sheet on narrow screens", async () => {
    const share = jest.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    window.matchMedia = ((query: string) => ({ matches: true, media: query, addEventListener: () => {}, removeEventListener: () => {} })) as never;
    act(() => useProfileModalStore.getState().open(SELF));
    renderModal();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: i18n.t("profile:shareProfile") }));
    });
    expect(share).toHaveBeenCalledWith({ url: `${window.location.origin}/u/${SELF}`, title: "@me" });
    expect(writeText).not.toHaveBeenCalled();
  });
});
