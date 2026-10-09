import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import Sidebar from "../Sidebar";
import { useSidebarLayoutStore } from "../../store/sidebarLayoutStore";
import type { DirectConversation } from "../../../dm/types";

const dm = (id: string, name: string, online = false): DirectConversation => ({
  id,
  name,
  initials: name[0],
  color: "#333",
  online,
  unread: 0,
  isFriend: true,
  friendship: "accepted",
});

const handlers = () => ({
  onToggleCommunity: jest.fn(),
  onSelectCommunity: jest.fn(),
  onSelectChannel: jest.fn(),
  onSelectDm: jest.fn(),
  onInviteCommunity: jest.fn(),
  onOpenCommunitySettings: jest.fn(),
  onAddChannel: jest.fn(),
  onDeleteChannel: jest.fn(),
  onOpenSettings: jest.fn(),
  onAddFriend: jest.fn(),
  onOpenPending: jest.fn(),
  onGoHome: jest.fn(),
  onCreateCommunity: jest.fn(),
  onJoinCommunity: jest.fn(),
  onSearchQueryChange: jest.fn(),
});

const renderSidebar = (h = handlers(), pendingCount = 0) => {
  render(
    <Sidebar
      {...h}
      communities={[{ id: "c1", name: "Gamers", ownerId: "me" } as never]}
      channelsByCommunity={{}}
      expandedCommunityIds={new Set()}
      selectedChannelId={null}
      joinedChannelId={null}
      selectedDmId={null}
      directs={[dm("u1", "Alice", true), dm("u2", "Bob")]}
      homeActive={false}
      pendingCount={pendingCount}
      currentUserId="me"
      currentUserName="me"
      currentUserStatus="online"
      searchQuery=""
      unreadByDm={{ u1: 4 }}
    />,
  );
  return h;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  window.localStorage.clear();
  act(() => useSidebarLayoutStore.setState({ width: 320, collapsed: false }));
});

it("collapses to a rail of pictures and expands back", () => {
  renderSidebar();
  expect(screen.getByText("Alice")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:collapseSidebar") }));
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
  expect(screen.queryByText("Alice")).toBeNull();
  expect(screen.getByRole("listitem", { name: i18n.t("sidebar:unreadIn", { name: "Alice", count: 4 }) })).toBeInTheDocument();
  expect(screen.getByRole("listitem", { name: "Bob" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:expandSidebar") }));
  expect(screen.getByText("Alice")).toBeInTheDocument();
});

it("keeps friend actions and the tab switch on the rail", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  const h = renderSidebar(handlers(), 3);
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:addFriend") }));
  expect(h.onAddFriend).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: `${i18n.t("sidebar:pendingRequests")}: 3` }));
  expect(h.onOpenPending).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("listitem", { name: "Bob" }));
  expect(h.onSelectDm).toHaveBeenCalledWith(expect.objectContaining({ id: "u2" }));
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:showCommunities") }));
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  expect(h.onSelectCommunity).toHaveBeenCalledWith("c1");
  expect(screen.getByRole("button", { name: i18n.t("sidebar:showDirects") })).toBeInTheDocument();
});

it("toggles from the resize handle with a double click and the keyboard", () => {
  renderSidebar();
  const handle = screen.getByRole("separator", { name: i18n.t("sidebar:resizeSidebar") });
  fireEvent.doubleClick(handle);
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
  fireEvent.keyDown(screen.getByRole("separator"), { key: "ArrowRight" });
  expect(useSidebarLayoutStore.getState().collapsed).toBe(false);
  fireEvent.keyDown(screen.getByRole("separator"), { key: "Enter" });
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
});
