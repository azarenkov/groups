import React from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import i18n from "../../../../i18n";
import ProfileStyleSettings from "../ProfileStyleSettings";
import UserBadge from "../UserBadge";
import { entitlementsAPI } from "../../api/entitlements";
import { useEntitlementsStore } from "../../store/entitlementsStore";
import { useBadgeStore } from "../../store/badgeStore";
import { useAuthStore } from "../../../auth/store/authStore";
import { useStickerStore } from "../../../stickers/store/stickerStore";
import { socialHandlers } from "../../../social/ws/socialHandlers";

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: {
    getMine: jest.fn(),
    setBadgeStyle: jest.fn(),
    setEmojiStatus: jest.fn(),
    clearEmojiStatus: jest.fn(),
    lookupBadges: jest.fn(),
  },
}));

jest.mock("../../../stickers/api/stickers", () => ({
  stickersAPI: { mine: jest.fn() },
}));

jest.mock("../../../stickers/lib/stickerImages", () => ({
  useStickerUrl: (id: string) => `blob:${id}`,
}));

jest.mock("../../../social/api/user", () => ({
  userAPI: { downloadBanner: jest.fn(), downloadAvatar: jest.fn() },
}));

const { userAPI } = jest.requireMock("../../../social/api/user") as {
  userAPI: { downloadBanner: jest.Mock; downloadAvatar: jest.Mock };
};

const api = entitlementsAPI as jest.Mocked<typeof entitlementsAPI>;
const { stickersAPI } = jest.requireMock("../../../stickers/api/stickers") as { stickersAPI: { mine: jest.Mock } };

const pack = {
  id: "p1",
  ownerId: "me",
  name: "Cats",
  slug: "cats",
  installCount: 1,
  stickerCount: 2,
  installed: true,
  coverStickerId: null,
  stickers: [
    { id: "s1", packId: "p1", emoji: "😺", animated: false, width: 512, height: 512 },
    { id: "s2", packId: "p1", emoji: "😼", animated: true, width: 512, height: 512 },
  ],
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  act(() => {
    useEntitlementsStore.getState().clear();
    useBadgeStore.getState().clear();
  });
  useAuthStore.setState({ user: { id: "me", name: "me" } } as never);
  useStickerStore.setState({ packs: [], loaded: false, failed: false, userId: null } as never);
  stickersAPI.mine.mockResolvedValue([pack]);
  api.lookupBadges.mockResolvedValue([]);
  userAPI.downloadBanner.mockRejectedValue(new Error("none"));
  userAPI.downloadAvatar.mockRejectedValue(new Error("none"));
});

it("lets a subscriber pick a sticker as a status for a chosen time and remove it", async () => {
  api.setEmojiStatus.mockResolvedValue({ stickerId: "s2", until: new Date(Date.now() + 8 * 3600_000).toISOString() });
  api.clearEmojiStatus.mockResolvedValue(undefined);
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  render(<ProfileStyleSettings />);
  const durations = within(screen.getByRole("radiogroup", { name: i18n.t("subscription:emojiStatus.duration") }));
  fireEvent.click(durations.getByRole("radio", { name: i18n.t("subscription:emojiStatus.durations.8h") }));
  const sticker = await screen.findByRole("button", { name: "😼" });
  await act(async () => {
    fireEvent.click(sticker);
  });
  expect(api.setEmojiStatus).toHaveBeenCalledWith("s2", "8h");
  expect(useEntitlementsStore.getState().emojiStatus?.stickerId).toBe("s2");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("subscription:emojiStatus.remove") }));
  });
  expect(api.clearEmojiStatus).toHaveBeenCalled();
  expect(useEntitlementsStore.getState().emojiStatus).toBeNull();
});

it("does not let a free user set a status", () => {
  render(<ProfileStyleSettings />);
  for (const option of within(
    screen.getByRole("radiogroup", { name: i18n.t("subscription:emojiStatus.duration") }),
  ).getAllByRole("radio")) {
    expect(option).toBeDisabled();
  }
  expect(stickersAPI.mine).not.toHaveBeenCalled();
});

it("shows a status next to someone's badge and hides it when it runs out", async () => {
  jest.useFakeTimers();
  try {
    act(() =>
      useBadgeStore.getState().apply("u2", {
        tier: "pro",
        badgeStyle: "tint",
        memberNumber: null,
        emojiStatus: { stickerId: "s1", until: new Date(Date.now() + 60_000).toISOString() },
      }),
    );
    render(<UserBadge userId="u2" />);
    expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
    act(() => {
      jest.advanceTimersByTime(61_000);
    });
    expect(screen.queryByTestId("emoji-status")).toBeNull();
  } finally {
    jest.useRealTimers();
  }
});

it("updates a status live from profile events", async () => {
  act(() => useBadgeStore.getState().apply("u2", { tier: "pro", badgeStyle: "tint", memberNumber: null }));
  render(<UserBadge userId="u2" />);
  expect(screen.queryByTestId("emoji-status")).toBeNull();
  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "u2",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: new Date().toISOString(),
    });
  });
  expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
  act(() => {
    socialHandlers["user.profileChanged"]({ userId: "u2", bio: "", badgeStyle: "tint", emojiStatus: null, occurredAt: new Date().toISOString() });
  });
  await waitFor(() => expect(screen.queryByTestId("emoji-status")).toBeNull());

  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "me",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s2", until: null },
      occurredAt: new Date().toISOString(),
    });
  });
  expect(useEntitlementsStore.getState().emojiStatus).toEqual({ stickerId: "s2", until: null });
});
