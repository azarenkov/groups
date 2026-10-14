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

it("shows the unread count and marks the displayed notifications read once opened", async () => {
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  expect(await screen.findByText("1")).toBeInTheDocument();
  await openInbox();
  expect(await screen.findByText(/Alice invited you to “Night Owls”/)).toBeInTheDocument();
  await waitFor(() => expect(api.markRead).toHaveBeenCalledWith(["n1"]));
  expect(document.querySelector(".inbox-badge")).toBeNull();
});

it("keeps unread notifications beyond the loaded page unread until read all is pressed", async () => {
  api.unread.mockResolvedValue(3);
  api.list.mockResolvedValue({ items: [invitation, friendRequest], nextCursor: "next" });
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  fireEvent.click(await screen.findByRole("button", { name: i18n.t("sidebar:inboxUnreadLabel", { count: 3 }) }));
  await waitFor(() => expect(api.markRead).toHaveBeenCalledWith(["n1"]));
  expect(await screen.findByText("2")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:inboxReadAll") }));
  expect(api.markRead).toHaveBeenLastCalledWith();
  await waitFor(() => expect(screen.queryByText("2")).toBeNull());
});

it("accepts an invitation from the inbox and opens the community", async () => {
  const onOpenCommunity = jest.fn();
  api.acceptInvitation.mockResolvedValue({ communityId: "c1" });
  render(<InboxMenu onOpenCommunity={onOpenCommunity} onOpenFriendRequests={jest.fn()} />);
  await openInbox();
  await screen.findByText(/Night Owls/);
  const row = screen.getByText(/Night Owls/).closest(".inbox-item") as HTMLElement;
  fireEvent.click(row.querySelector(".inbox-action.primary") as HTMLElement);
  await waitFor(() => expect(onOpenCommunity).toHaveBeenCalledWith("c1"));
  expect(api.acceptInvitation).toHaveBeenCalledWith("i1");
});

it("puts a declined invitation back when the server refuses", async () => {
  api.declineInvitation.mockRejectedValue(new Error("down"));
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  await openInbox();
  await screen.findByText(/Night Owls/);
  const row = screen.getByText(/Night Owls/).closest(".inbox-item") as HTMLElement;
  const decline = Array.from(row.querySelectorAll(".inbox-action")).find((b) => !b.classList.contains("primary")) as HTMLElement;
  fireEvent.click(decline);
  await waitFor(() => expect(api.declineInvitation).toHaveBeenCalledWith("i1"));
  expect(await screen.findByText(i18n.t("sidebar:inboxActionFailed"))).toBeInTheDocument();
  expect(screen.getByText(/Night Owls/)).toBeInTheDocument();
});

it("refetches the inbox after rolling back so newer notifications are not lost", async () => {
  api.declineInvitation.mockRejectedValue(new Error("down"));
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  await openInbox();
  await screen.findByText(/Night Owls/);
  const newer: InboxItem = { ...friendRequest, id: "n3", actor: { id: "u3", name: "Carol" }, read: false };
  api.list.mockResolvedValue({ items: [newer, invitation, friendRequest], nextCursor: null });
  const row = screen.getByText(/Night Owls/).closest(".inbox-item") as HTMLElement;
  const decline = Array.from(row.querySelectorAll(".inbox-action")).find((b) => !b.classList.contains("primary")) as HTMLElement;
  fireEvent.click(decline);
  expect(await screen.findByText(/Carol/)).toBeInTheDocument();
});

it("offers friend request actions only while the request is still incoming", async () => {
  const accept = jest.fn().mockResolvedValue(undefined);
  useFriendshipStore.setState({ accept });
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  await openInbox();
  const row = (await screen.findByText(/Bob wants to be your friend/)).closest(".inbox-item") as HTMLElement;
  fireEvent.click(row.querySelector(".inbox-action.primary") as HTMLElement);
  await waitFor(() => expect(accept).toHaveBeenCalledWith("u2"));
  act(() => useFriendshipStore.setState({ statusById: { u2: "accepted" } }));
  expect(row.querySelector(".inbox-action")).toBeNull();
});

it("refetches when the server reports a change", async () => {
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  await screen.findByText("1");
  api.unread.mockResolvedValue(3);
  act(() => refreshInbox());
  expect(await screen.findByText("3")).toBeInTheDocument();
});

it("opens friend request management from the inbox", async () => {
  const onOpenFriendRequests = jest.fn();
  useFriendshipStore.setState({ statusById: { u2: "pending_incoming", u3: "pending_outgoing", u4: "accepted" } });
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={onOpenFriendRequests} />);
  await openInbox();
  const entry = await screen.findByRole("button", { name: `${i18n.t("sidebar:pendingRequests")} 2` });
  fireEvent.click(entry);
  expect(onOpenFriendRequests).toHaveBeenCalled();
  expect(screen.queryByRole("dialog", { name: i18n.t("sidebar:inboxTitle") })).toBeNull();
});

it("drops the unread highlight whichever way the menu closes", async () => {
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  const trigger = await openInbox();
  await waitFor(() => expect(api.markRead).toHaveBeenCalledWith(["n1"]));
  const row = () => screen.getByText(/Alice invited you to “Night Owls”/).closest(".inbox-item");
  expect(row()).toHaveClass("unread");
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog", { name: i18n.t("sidebar:inboxTitle") })).toBeNull();
  fireEvent.click(trigger);
  expect(await screen.findByText(/Alice invited you to “Night Owls”/)).toBeInTheDocument();
  expect(row()).not.toHaveClass("unread");
});

it("does not retry marking notifications read after the server refuses while the menu stays open", async () => {
  api.markRead.mockRejectedValue(new Error("down"));
  render(<InboxMenu onOpenCommunity={jest.fn()} onOpenFriendRequests={jest.fn()} />);
  await openInbox();
  await waitFor(() => expect(api.markRead).toHaveBeenCalledWith(["n1"]));
  await waitFor(() => expect(api.list).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(document.querySelector(".inbox-badge")).toHaveTextContent("1"));
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 50));
  });
  expect(api.markRead).toHaveBeenCalledTimes(1);
});
