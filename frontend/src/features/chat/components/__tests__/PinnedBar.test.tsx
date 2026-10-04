import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import PinnedBar from "../PinnedBar";
import type { PinnedMessage } from "../../api/pins";

const pin = (id: string, extra: Partial<PinnedMessage> = {}): PinnedMessage => ({
  messageId: id,
  authorId: "u2",
  authorName: "Bob",
  content: `text ${id}`,
  hasAttachments: false,
  createdAt: "2026-10-01T10:00:00Z",
  pinnedBy: "u2",
  pinnedAt: "2026-10-01T10:00:00Z",
  ...extra,
});

describe("PinnedBar", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("renders nothing without pins", () => {
    const { container } = render(
      <PinnedBar pins={[]} canUnpin onJump={jest.fn()} onUnpin={jest.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("jumps to the shown pin and then moves on to the next one", () => {
    const onJump = jest.fn();
    render(<PinnedBar pins={[pin("a"), pin("b")]} currentUserId="u1" canUnpin onJump={onJump} onUnpin={jest.fn()} />);
    expect(screen.getByText("Pinned message 1 of 2")).toBeTruthy();
    fireEvent.click(screen.getByText("text a"));
    expect(onJump).toHaveBeenLastCalledWith("a");
    expect(screen.getByText("Pinned message 2 of 2")).toBeTruthy();
    fireEvent.click(screen.getByText("text b"));
    expect(onJump).toHaveBeenLastCalledWith("b");
    expect(screen.getByText("Pinned message 1 of 2")).toBeTruthy();
  });

  it("describes stickers and attachments and lists every pin", () => {
    const onUnpin = jest.fn();
    render(
      <PinnedBar
        pins={[pin("a", { content: "", stickerEmoji: "🐱" }), pin("b", { content: "", hasAttachments: true, authorId: "u1" })]}
        currentUserId="u1"
        canUnpin
        onJump={jest.fn()}
        onUnpin={onUnpin}
      />,
    );
    expect(screen.getByText("Sticker 🐱")).toBeTruthy();
    fireEvent.click(screen.getByLabelText("All pinned messages"));
    expect(screen.getByText("📎 Attachment")).toBeTruthy();
    expect(screen.getByText("You")).toBeTruthy();
    fireEvent.click(screen.getAllByLabelText("Unpin")[1]);
    expect(onUnpin).toHaveBeenCalledWith("b");
  });

  it("hides unpinning for people who cannot unpin", () => {
    render(<PinnedBar pins={[pin("a")]} canUnpin={false} onJump={jest.fn()} onUnpin={jest.fn()} />);
    fireEvent.click(screen.getByLabelText("All pinned messages"));
    expect(screen.queryByLabelText("Unpin")).toBeNull();
  });
});
