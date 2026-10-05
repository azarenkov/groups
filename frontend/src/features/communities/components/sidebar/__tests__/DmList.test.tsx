import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import DmList from "../DmList";
import { useNotificationPrefsStore } from "../../../../settings/store/notificationPrefsStore";
import { useNotificationSettingsStore } from "../../../../settings/store/notificationSettingsStore";
import type { DirectConversation } from "../../../../dm/types";
import { useConversationPinsStore } from "../../../store/conversationPinsStore";

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
      useFriendshipStore.setState({ friends: [], nextCursor: null });
      useDmPreviewStore.setState({ byFriend: {}, nextCursor: null });
      useConversationPinsStore
        .getState()
        .applyRemote({ dms: ["u1", "gone1", "gone2", "gone3", "gone4", "gone5", "gone6", "gone7", "gone8", "gone9"] });
    });
    renderList();
    const bob = screen.getByText("Bob").closest(".sb-dm") as HTMLElement;
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
