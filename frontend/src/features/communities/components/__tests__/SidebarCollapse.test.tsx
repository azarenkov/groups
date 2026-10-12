import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import Sidebar from "../Sidebar";
import type { SidebarNav } from "../Sidebar";
import { useSidebarLayoutStore } from "../../store/sidebarLayoutStore";
import { useCallStore } from "../../../voice/store/callStore";
import type { DirectConversation } from "../../../dm/types";
import { useHiddenDmStore } from "../../../dm/store/hiddenDmStore";

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
  onSelectChannel: jest.fn(),
  onSelectDm: jest.fn(),
  onInviteCommunity: jest.fn(),
  onOpenCommunitySettings: jest.fn(),
  onAddChannel: jest.fn(),
  onDeleteChannel: jest.fn(),
  onAddFriend: jest.fn(),
  onOpenPending: jest.fn(),
  onCreateCommunity: jest.fn(),
  onJoinCommunity: jest.fn(),
});

const Harness: React.FC<{ h: ReturnType<typeof handlers>; pendingCount: number }> = ({ h, pendingCount }) => {
  const [tab, setTab] = React.useState<"dms" | "communities">("dms");
  const [openCommunityId, setOpenCommunityId] = React.useState<string | null>(null);
  const nav: SidebarNav = { tab, setTab, openCommunityId, setOpenCommunityId };
  return (
    <Sidebar
      {...h}
      nav={nav}
      communities={[{ id: "c1", name: "Gamers", ownerId: "me" } as never]}
      channelsByCommunity={{
        c1: [
          { id: "ch1", name: "general", kind: "text", communityId: "c1" } as never,
          { id: "ch2", name: "lobby", kind: "voice", communityId: "c1" } as never,
        ],
      }}
      expandedCommunityIds={new Set()}
      selectedChannelId={null}
      joinedChannelId={null}
      selectedDmId={null}
      directs={[dm("u1", "Alice", true), dm("u2", "Bob")]}
      pendingCount={pendingCount}
      currentUserId="me"
      unreadByDm={{ u1: 4 }}
    />
  );
};

const renderSidebar = (h = handlers(), pendingCount = 0) => {
  render(<Harness h={h} pendingCount={pendingCount} />);
  return h;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  useHiddenDmStore.setState({ hidden: [], loaded: true });
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
  expect(screen.getByRole("tab", { name: i18n.t("sidebar:showDirects") })).toHaveAttribute("aria-selected", "true");
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  expect(screen.getByRole("dialog", { name: "Gamers" })).toBeInTheDocument();
  expect(h.onSelectChannel).not.toHaveBeenCalled();
  expect(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") })).toHaveAttribute("aria-selected", "true");
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

it("opens a community's channels next to the rail and closes after picking one", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  const h = renderSidebar();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  const flyout = screen.getByRole("dialog", { name: "Gamers" });
  expect(flyout).toHaveTextContent("general");
  expect(flyout).toHaveTextContent("lobby");
  fireEvent.click(screen.getByText("general"));
  expect(h.onSelectChannel).toHaveBeenCalledWith(expect.objectContaining({ id: "c1" }), expect.objectContaining({ id: "ch1" }));
  expect(screen.queryByRole("dialog", { name: "Gamers" })).toBeNull();
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
});

it("closes the channel panel with Escape or a second click", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  renderSidebar();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  expect(screen.getByRole("dialog", { name: "Gamers" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("listitem", { name: "Gamers" }));
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("offers creating and joining a community from the rail", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  const h = renderSidebar();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:joinCommunity") }));
  expect(h.onJoinCommunity).toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:createCommunity") }));
  expect(h.onCreateCommunity).toHaveBeenCalled();
});

it("shows friend actions only with direct messages", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  renderSidebar();
  expect(screen.getByRole("button", { name: i18n.t("sidebar:addFriend") })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  expect(screen.queryByRole("button", { name: i18n.t("sidebar:addFriend") })).toBeNull();
  expect(screen.queryByRole("button", { name: i18n.t("sidebar:pendingRequests") })).toBeNull();
});

it("keeps the rail actions outside the scrolling list", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  renderSidebar();
  expect(screen.getByRole("button", { name: i18n.t("sidebar:addFriend") }).closest(".sb-scroll")).toBeNull();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  expect(screen.getByRole("button", { name: i18n.t("sidebar:joinCommunity") }).closest(".sb-scroll")).toBeNull();
});

it("finishes a drag that collapses the panel when the pointer is released", () => {
  renderSidebar();
  act(() => {
    screen
      .getByRole("separator", { name: i18n.t("sidebar:resizeSidebar") })
      .dispatchEvent(new MouseEvent("pointerdown", { button: 0, bubbles: true }));
  });
  expect(document.body).toHaveClass("sb-resizing");
  act(() => {
    window.dispatchEvent(new MouseEvent("pointermove", { clientX: 10 }));
  });
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
  act(() => {
    window.dispatchEvent(new MouseEvent("pointerup"));
  });
  expect(document.body).not.toHaveClass("sb-resizing");
});

it("marks a chat with an ongoing call", () => {
  act(() => useCallStore.setState({ joinableByPeer: { u2: "call-1" } }));
  renderSidebar();
  expect(screen.getByText(i18n.t("call:ongoingCall"))).toBeInTheDocument();
  act(() => useCallStore.setState({ joinableByPeer: {} }));
});

it("lights the chats dot only for unread chats that are known to be visible", () => {
  act(() => useSidebarLayoutStore.setState({ collapsed: true }));
  act(() => useHiddenDmStore.setState({ hidden: [], loaded: false, hydrated: false }));
  const { container } = render(<Harness h={handlers()} pendingCount={0} />);
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("sidebar:showCommunities") }));
  expect(container.querySelector(".sb-rail-dot")).toBeNull();
  act(() => useHiddenDmStore.setState({ hidden: ["u1"], loaded: true }));
  expect(container.querySelector(".sb-rail-dot")).toBeNull();
  act(() => useHiddenDmStore.setState({ hidden: [] }));
  expect(container.querySelector(".sb-rail-dot")).not.toBeNull();
});
