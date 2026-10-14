import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import InboxMenu from "../InboxMenu";
import { inboxAPI, type InboxItem } from "../../api/inbox";
import { queryClient } from "../../../../shared/query/queryClient";
import { useAuthStore } from "../../../auth/store/authStore";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { refreshInbox } from "../../lib/inbox";

jest.mock("../../api/inbox", () => ({
  inboxAPI: {
    list: jest.fn(),
    unread: jest.fn(),
    markRead: jest.fn(),
    acceptInvitation: jest.fn(),
    declineInvitation: jest.fn(),
  },
}));

const api = inboxAPI as jest.Mocked<typeof inboxAPI>;

const invitation: InboxItem = {
  id: "n1",
  kind: "community_invitation",
  actor: { id: "u1", name: "Alice" },
  community: { id: "c1", name: "Night Owls" },
  invitationId: "i1",
  createdAt: "2026-10-14T10:00:00Z",
  read: false,
};

const friendRequest: InboxItem = {
  id: "n2",
  kind: "friend_request_received",
  actor: { id: "u2", name: "Bob" },
  createdAt: "2026-10-14T09:00:00Z",
  read: true,
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  queryClient.clear();
  jest.resetAllMocks();
  useAuthStore.setState({ user: { id: "me", name: "me" } as never });
  useFriendshipStore.setState({ loaded: true, statusById: { u2: "pending_incoming" } });
  api.unread.mockResolvedValue(1);
  api.list.mockResolvedValue({ items: [invitation, friendRequest], nextCursor: null });
  api.markRead.mockResolvedValue();
});

const openInbox = async () => {
  const trigger = await screen.findByRole("button", { name: i18n.t("sidebar:inboxUnreadLabel", { count: 1 }) });
  fireEvent.click(trigger);
  return trigger;
};

it("shows the unread count and marks everything read once opened", async () => {
  render(<InboxMenu onOpenCommunity={jest.fn()} />);
  expect(await screen.findByText("1")).toBeInTheDocument();
  await openInbox();
  expect(await screen.findByText(/Alice invited you to “Night Owls”/)).toBeInTheDocument();
  await waitFor(() => expect(api.markRead).toHaveBeenCalledWith());
  expect(screen.queryByText("1")).toBeNull();
});

it("accepts an invitation from the inbox and opens the community", async () => {
  const onOpenCommunity = jest.fn();
  api.acceptInvitation.mockResolvedValue({ communityId: "c1" });
  render(<InboxMenu onOpenCommunity={onOpenCommunity} />);
  await openInbox();
  await screen.findByText(/Night Owls/);
  const row = screen.getByText(/Night Owls/).closest(".inbox-item") as HTMLElement;
  fireEvent.click(row.querySelector(".inbox-action.primary") as HTMLElement);
  await waitFor(() => expect(onOpenCommunity).toHaveBeenCalledWith("c1"));
  expect(api.acceptInvitation).toHaveBeenCalledWith("i1");
});

it("puts a declined invitation back when the server refuses", async () => {
  api.declineInvitation.mockRejectedValue(new Error("down"));
  render(<InboxMenu onOpenCommunity={jest.fn()} />);
  await openInbox();
  await screen.findByText(/Night Owls/);
  const row = screen.getByText(/Night Owls/).closest(".inbox-item") as HTMLElement;
  const decline = Array.from(row.querySelectorAll(".inbox-action")).find((b) => !b.classList.contains("primary")) as HTMLElement;
  fireEvent.click(decline);
  await waitFor(() => expect(api.declineInvitation).toHaveBeenCalledWith("i1"));
  expect(await screen.findByText(i18n.t("sidebar:inboxActionFailed"))).toBeInTheDocument();
  expect(screen.getByText(/Night Owls/)).toBeInTheDocument();
});

it("offers friend request actions only while the request is still incoming", async () => {
  const accept = jest.fn().mockResolvedValue(undefined);
  useFriendshipStore.setState({ accept });
  render(<InboxMenu onOpenCommunity={jest.fn()} />);
  await openInbox();
  const row = (await screen.findByText(/Bob wants to be your friend/)).closest(".inbox-item") as HTMLElement;
  fireEvent.click(row.querySelector(".inbox-action.primary") as HTMLElement);
  await waitFor(() => expect(accept).toHaveBeenCalledWith("u2"));
  act(() => useFriendshipStore.setState({ statusById: { u2: "accepted" } }));
  expect(row.querySelector(".inbox-action")).toBeNull();
});

it("refetches when the server reports a change", async () => {
  render(<InboxMenu onOpenCommunity={jest.fn()} />);
  await screen.findByText("1");
  api.unread.mockResolvedValue(3);
  act(() => refreshInbox());
  expect(await screen.findByText("3")).toBeInTheDocument();
});
