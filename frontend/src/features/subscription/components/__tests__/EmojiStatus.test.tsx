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

it("lets a subscriber pick a sticker as a status and remove it", async () => {
  api.setEmojiStatus.mockResolvedValue({ emojiStatus: { stickerId: "s2", until: null }, updatedAt: new Date().toISOString() });
  api.clearEmojiStatus.mockResolvedValue({ emojiStatus: null, updatedAt: new Date().toISOString() });
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  render(<ProfileStyleSettings />);
  const sticker = await screen.findByRole("button", { name: "😼" });
  await act(async () => {
    fireEvent.click(sticker);
  });
  expect(api.setEmojiStatus).toHaveBeenCalledWith("s2");
  expect(useEntitlementsStore.getState().emojiStatus?.stickerId).toBe("s2");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("subscription:emojiStatus.remove") }));
  });
  expect(api.clearEmojiStatus).toHaveBeenCalled();
  expect(useEntitlementsStore.getState().emojiStatus).toBeNull();
});

it("does not let a free user set a status", () => {
  render(<ProfileStyleSettings />);
  expect(screen.queryByRole("button", { name: "😺" })).toBeNull();
  expect(stickersAPI.mine).not.toHaveBeenCalled();
});

it("shows a status instead of someone's badge and brings the badge back when it runs out", async () => {
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
    const { container } = render(<UserBadge userId="u2" />);
    expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
    expect(container.querySelector(".pro-badge")).toBeNull();
    act(() => {
      jest.advanceTimersByTime(61_000);
    });
    expect(screen.queryByTestId("emoji-status")).toBeNull();
    expect(container.querySelector(".pro-badge")).not.toBeNull();
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

it("brings the badge back when a badge style is picked over a sticker", async () => {
  api.setEmojiStatus.mockResolvedValue({ emojiStatus: { stickerId: "s1", until: null }, updatedAt: new Date().toISOString() });
  api.clearEmojiStatus.mockResolvedValue({ emojiStatus: null, updatedAt: new Date().toISOString() });
  api.setBadgeStyle.mockImplementation((style) => Promise.resolve(style));
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  render(<ProfileStyleSettings />);
  const sticker = await screen.findByRole("button", { name: "😺" });
  await act(async () => {
    fireEvent.click(sticker);
  });
  const badges = within(screen.getByRole("radiogroup", { name: i18n.t("subscription:badge.title") }));
  for (const option of badges.getAllByRole("radio")) expect(option).toHaveAttribute("aria-checked", "false");
  await act(async () => {
    fireEvent.click(badges.getByRole("radio", { name: new RegExp(i18n.t("subscription:badge.styles.ink")) }));
  });
  expect(api.setBadgeStyle).toHaveBeenCalledWith("ink");
  expect(api.clearEmojiStatus).toHaveBeenCalled();
  expect(useEntitlementsStore.getState().emojiStatus).toBeNull();
});

it("shows one pack at a time so large libraries stay light", async () => {
  const dogs = {
    ...pack,
    id: "p2",
    name: "Dogs",
    slug: "dogs",
    stickers: [{ id: "d1", packId: "p2", emoji: "🐶", animated: false, width: 512, height: 512 }],
  };
  stickersAPI.mine.mockResolvedValue([pack, dogs]);
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  render(<ProfileStyleSettings />);
  expect(await screen.findByRole("button", { name: "😺" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "🐶" })).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: "Dogs" }));
  expect(screen.getByRole("button", { name: "🐶" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "😺" })).toBeNull();
});

it("offers stickers from the user's own packs even when they are not installed", async () => {
  const mine = {
    ...pack,
    id: "p3",
    name: "Mine",
    slug: "mine",
    ownerId: "me",
    installed: false,
    stickers: [{ id: "m1", packId: "p3", emoji: "🦊", animated: false, width: 512, height: 512 }],
  };
  const removed = { ...mine, id: "p4", name: "Removed", ownerId: "someone", stickers: [{ ...mine.stickers[0], id: "r1", packId: "p4", emoji: "🐸" }] };
  stickersAPI.mine.mockResolvedValue([mine, removed]);
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  render(<ProfileStyleSettings />);
  expect(await screen.findByRole("button", { name: "🦊" })).toBeInTheDocument();
  expect(screen.queryByRole("tab", { name: "Removed" })).toBeNull();
  expect(screen.queryByRole("button", { name: "🐸" })).toBeNull();
});

it("ignores a status from a profile event older than what is already known", () => {
  act(() => useBadgeStore.getState().apply("u3", { tier: "pro", badgeStyle: "tint", memberNumber: null }));
  render(<UserBadge userId="u3" />);
  const newer = new Date().toISOString();
  const older = new Date(Date.now() - 60_000).toISOString();
  act(() => {
    socialHandlers["user.profileChanged"]({ userId: "u3", bio: "", badgeStyle: "tint", emojiStatus: null, occurredAt: newer });
    socialHandlers["user.profileChanged"]({
      userId: "u3",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: older,
    });
  });
  expect(screen.queryByTestId("emoji-status")).toBeNull();
});

it("applies a delayed status event even after a newer event without status fields", () => {
  act(() => useBadgeStore.getState().apply("u4", { tier: "pro", badgeStyle: "tint", memberNumber: null }));
  render(<UserBadge userId="u4" />);
  const newer = new Date().toISOString();
  const older = new Date(Date.now() - 60_000).toISOString();
  act(() => {
    socialHandlers["user.profileChanged"]({ userId: "u4", bio: "", badgeStyle: "tint", occurredAt: newer });
    socialHandlers["user.profileChanged"]({
      userId: "u4",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: older,
    });
  });
  expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
});

it("does not let a delayed older event undo a status the user just picked", async () => {
  const before = new Date(Date.now() - 60_000).toISOString();
  api.setEmojiStatus.mockResolvedValue({ emojiStatus: { stickerId: "s2", until: null }, updatedAt: new Date().toISOString() });
  act(() => useEntitlementsStore.getState().applyTier("pro"));
  await act(async () => {
    await useEntitlementsStore.getState().setEmojiStatus("s2");
  });
  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "me",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: before,
    });
  });
  expect(useEntitlementsStore.getState().emojiStatus).toEqual({ stickerId: "s2", until: null });
});

it("keeps a status from an event that lands while a badge lookup is in flight", async () => {
  let answer: (value: unknown) => void = () => undefined;
  api.lookupBadges.mockImplementation(() => new Promise((resolve) => (answer = resolve)));
  render(<UserBadge userId="u5" />);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 60));
  });
  expect(api.lookupBadges).toHaveBeenCalledWith(["u5"]);
  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "u5",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: new Date().toISOString(),
    });
  });
  await act(async () => {
    answer([{ userId: "u5", tier: "pro", badgeStyle: "tint", memberNumber: null, emojiStatus: null }]);
  });
  expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
});

it("does not let a delayed older event replace the status loaded from the server", async () => {
  api.getMine.mockResolvedValue({
    tier: "pro",
    badgeStyle: "tint",
    memberNumber: null,
    validUntil: null,
    trial: false,
    trialAvailable: false,
    emojiStatus: { stickerId: "s2", until: null },
    profileUpdatedAt: new Date().toISOString(),
  });
  await act(async () => {
    await useEntitlementsStore.getState().fetch();
  });
  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "me",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: new Date(Date.now() - 60_000).toISOString(),
    });
  });
  expect(useEntitlementsStore.getState().emojiStatus).toEqual({ stickerId: "s2", until: null });
});

it("keeps a status event received before a badge lookup when the projection is still behind", async () => {
  const eventAt = new Date().toISOString();
  act(() => useBadgeStore.getState().apply("u6", { tier: "pro", badgeStyle: "tint", memberNumber: null }));
  act(() => {
    socialHandlers["user.profileChanged"]({
      userId: "u6",
      bio: "",
      badgeStyle: "tint",
      emojiStatus: { stickerId: "s1", until: null },
      occurredAt: eventAt,
    });
  });
  api.lookupBadges.mockResolvedValue([
    {
      userId: "u6",
      tier: "pro",
      badgeStyle: "tint",
      memberNumber: null,
      emojiStatus: null,
      emojiStatusUpdatedAt: new Date(Date.now() - 60_000).toISOString(),
    },
  ]);
  act(() => useBadgeStore.getState().forget("u6"));
  render(<UserBadge userId="u6" />);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 60));
  });
  expect(api.lookupBadges).toHaveBeenCalledWith(["u6"]);
  expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
});

it("takes a newer status from a badge lookup", async () => {
  api.lookupBadges.mockResolvedValue([
    {
      userId: "u7",
      tier: "pro",
      badgeStyle: "tint",
      memberNumber: null,
      emojiStatus: { stickerId: "s2", until: null },
      emojiStatusUpdatedAt: new Date().toISOString(),
    },
  ]);
  render(<UserBadge userId="u7" />);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 60));
  });
  expect(screen.getByTestId("emoji-status")).toBeInTheDocument();
});
