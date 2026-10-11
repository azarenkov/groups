import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import DmList from "../DmList";
import { useNotificationPrefsStore } from "../../../../settings/store/notificationPrefsStore";
import { useNotificationSettingsStore } from "../../../../settings/store/notificationSettingsStore";
import type { DirectConversation } from "../../../../dm/types";
import { useConversationPinsStore } from "../../../store/conversationPinsStore";
import { useHiddenDmStore } from "../../../../dm/store/hiddenDmStore";

const dm = (id: string, name: string): DirectConversation => ({
  id,
  name,
  initials: name[0],
  color: "#333",
  online: false,
  unread: 0,
  isFriend: true,
  friendship: "accepted",
});

beforeEach(() => {
  act(() => useHiddenDmStore.setState({ loaded: true }));
});

const renderList = () =>
  render(
    <DmList
      directs={[dm("u1", "Alice"), dm("u2", "Bob")]}
      isFiltering={false}
      selectedDmId={null}
      unreadByDm={{ u1: 3, u2: 2 }}
      pendingCount={0}
      onAddFriend={jest.fn()}
      onOpenPending={jest.fn()}
      onPick={jest.fn()}
    />,
  );

describe("DmList mute indicator", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("marks only conversations set to deliver nothing", () => {
    act(() => {
      useNotificationSettingsStore.setState({ mentions: "all" });
      useNotificationPrefsStore.setState({ dmLevels: { u2: "none" } });
    });
    renderList();
    const muted = screen.getAllByLabelText("Notifications muted");
    expect(muted).toHaveLength(1);
    expect(muted[0].closest(".sb-dm")?.textContent).toContain("Bob");
    expect(screen.getByText("2").className).toContain("muted");
    expect(screen.getByText("3").className).not.toContain("muted");
  });

  it("follows the global setting when a conversation has no override", () => {
    act(() => {
      useNotificationSettingsStore.setState({ mentions: "none" });
      useNotificationPrefsStore.setState({ dmLevels: { u1: "all" } });
    });
    renderList();
    const muted = screen.getAllByLabelText("Notifications muted");
    expect(muted).toHaveLength(1);
    expect(muted[0].closest(".sb-dm")?.textContent).toContain("Bob");
  });
});

describe("DmList pins", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("marks pinned chats", () => {
    act(() => {
      useNotificationSettingsStore.setState({ mentions: "all" });
      useNotificationPrefsStore.setState({ dmLevels: {} });
      useConversationPinsStore.getState().applyRemote({ dms: ["u1"] });
    });
    renderList();
    const marks = screen.getAllByLabelText("Pinned");
    expect(marks).toHaveLength(1);
    expect(marks[0].closest(".sb-dm")?.textContent).toContain("Alice");
  });
});

describe("DmList pin limit", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("does not count pins of chats that can no longer be shown", () => {
    const { useContextMenuStore } = jest.requireActual("../../../../../shared/ui/contextMenu/contextMenuStore");
    const { useFriendshipStore } = jest.requireActual("../../../../social/store/friendshipStore");
    const { useDmPreviewStore } = jest.requireActual("../../../../dm/store/dmPreviewStore");
    act(() => {
      useFriendshipStore.setState({ friends: [], nextCursor: null, loaded: true });
      useDmPreviewStore.setState({ byFriend: {}, nextCursor: null, loaded: false });
      useConversationPinsStore
        .getState()
        .applyRemote({ dms: ["u1", "gone1", "gone2", "gone3", "gone4", "gone5", "gone6", "gone7", "gone8", "gone9"] });
    });
    renderList();
    const bob = screen.getByText("Bob").closest(".sb-dm") as HTMLElement;
    fireEvent.contextMenu(bob);
    const early = useContextMenuStore.getState().menu.entries.find((e: { id: string }) => e.id === "pin");
    expect(early.disabled).toBe(true);
    act(() => {
      useDmPreviewStore.setState({ loaded: true });
    });
    fireEvent.contextMenu(bob);
    const pin = useContextMenuStore.getState().menu.entries.find((e: { id: string }) => e.id === "pin");
    expect(pin.disabled).toBe(false);
    act(() => {
      useDmPreviewStore.setState({ nextCursor: "more" });
    });
    fireEvent.contextMenu(bob);
    const limited = useContextMenuStore.getState().menu.entries.find((e: { id: string }) => e.id === "pin");
    expect(limited.disabled).toBe(true);
  });
});

describe("DmList hidden chats", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  beforeEach(() => {
    act(() => {
      useNotificationSettingsStore.setState({ mentions: "all" });
      useNotificationPrefsStore.setState({ dmLevels: {} });
      useConversationPinsStore.getState().applyRemote({ dms: [] });
    });
  });

  afterEach(() => {
    act(() => useHiddenDmStore.setState({ hidden: [] }));
  });

  it("moves hidden chats into a folder with their total unread", () => {
    act(() => useHiddenDmStore.setState({ hidden: ["u2", "gone"] }));
    render(
      <DmList
        directs={[dm("u1", "Alice"), dm("u2", "Bob")]}
        selectedDmId={null}
        unreadByDm={{ u1: 3, u2: 2, gone: 4 }}
        pendingCount={0}
        onAddFriend={jest.fn()}
        onOpenPending={jest.fn()}
        onPick={jest.fn()}
      />,
    );
    expect(screen.queryByText("Bob")).toBeNull();
    expect(screen.getByText("Alice")).toBeTruthy();
    const folder = screen.getByText("Hidden").closest("button")!;
    expect(folder.textContent).toContain("6");
    fireEvent.click(folder);
    expect(screen.getByText("Bob")).toBeTruthy();
    expect(screen.queryByText("Alice")).toBeNull();
    expect(screen.getByText("2").className).toContain("muted");
  });

  it("keeps loading more chats from the compact hidden view", () => {
    act(() => useHiddenDmStore.setState({ hidden: ["far"] }));
    const loadMore = jest.fn();
    render(
      <DmList
        compact
        directs={[dm("u1", "Alice")]}
        selectedDmId={null}
        unreadByDm={{}}
        pendingCount={0}
        hasMoreDirects
        onLoadMoreDirects={loadMore}
        onAddFriend={jest.fn()}
        onOpenPending={jest.fn()}
        onPick={jest.fn()}
      />,
    );
    fireEvent.click(screen.getByLabelText("Hidden"));
    fireEvent.click(screen.getByLabelText("Show more"));
    expect(loadMore).toHaveBeenCalled();
  });

  it("has no folder when nothing is hidden", () => {
    renderList();
    expect(screen.queryByText("Hidden")).toBeNull();
    expect(screen.getByText("Alice")).toBeTruthy();
    expect(screen.getByText("Bob")).toBeTruthy();
  });
});

describe("DmList before hidden chats are known", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("does not show any chat until the hidden list is loaded", () => {
    act(() => useHiddenDmStore.setState({ hidden: [], loaded: false, hydrated: false }));
    renderList();
    expect(screen.queryByText("Alice")).toBeNull();
    expect(screen.getByText("Loading…")).toBeTruthy();
    act(() => useHiddenDmStore.setState({ hidden: ["u2"], hydrated: true }));
    expect(screen.queryByText("Alice")).toBeNull();
    act(() => useHiddenDmStore.setState({ hidden: ["u2"], loaded: true }));
    expect(screen.getByText("Alice")).toBeTruthy();
    expect(screen.queryByText("Bob")).toBeNull();
  });
});
