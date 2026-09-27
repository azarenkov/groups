import { fireEvent, render, screen } from "@testing-library/react";

import "../../../../i18n";
import { Message } from "../Message";
import { useProfileModalStore } from "../../../social/store/profileModalStore";
import type { ChannelMessage } from "../../../../shared/types/channel";

const noop = () => {};

const props = (msg: ChannelMessage, displayName: string) => ({
  msg,
  displayName,
  continued: false,
  isMine: false,
  dmStyle: false,
  receipt: null,
  editing: false,
  highlighted: false,
  editText: "",
  editRemovedIds: [],
  editFiles: [],
  editSaving: false,
  inviteIsMember: false,
  onEditTextChange: noop,
  onStartEdit: noop,
  onCancelEdit: noop,
  onSaveEdit: noop,
  onToggleRemoveAttachment: noop,
  onAddEditFiles: noop,
  onRemoveEditFile: noop,
  onDelete: noop,
  onAcceptInvite: noop,
  onDeclineInvite: noop,
  onOpenInviteCommunity: noop,
});

const base: ChannelMessage = {
  id: "m1",
  channelId: "c1",
  authorId: "u-nikk",
  content: "привет",
  createdAt: "2026-09-24T10:00:00Z",
  editedAt: null,
};

describe("names open profiles", () => {
  beforeEach(() => useProfileModalStore.setState({ userId: null }));

  it("opens the author's profile from their name", () => {
    render(<Message {...props(base, "Nikk")} />);
    const links = screen.getAllByRole("link", { name: "Nikk" });
    expect(links.length).toBeGreaterThan(0);
    links.forEach((link) => {
      useProfileModalStore.setState({ userId: null });
      fireEvent.click(link);
      expect(useProfileModalStore.getState().userId).toBe("u-nikk");
    });
  });

  it("opens the profile of the member a system message is about", () => {
    const joined: ChannelMessage = {
      ...base,
      id: "m2",
      authorId: "system",
      kind: "system",
      system: { type: "member_joined", userId: "u-chef" },
    };
    render(<Message {...props(joined, "CHEF")} />);
    const link = screen.getByRole("link", { name: "CHEF" });
    expect(link.closest(".msg-system-text")?.textContent).toMatch(/CHEF/);
    fireEvent.keyDown(link, { key: "Enter" });
    expect(useProfileModalStore.getState().userId).toBe("u-chef");
  });
});
