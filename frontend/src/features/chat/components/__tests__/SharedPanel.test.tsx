import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import SharedPanel from "../SharedPanel";
import { sharedAPI } from "../../api/shared";

jest.mock("../../api/shared", () => {
  const actual = jest.requireActual("../../api/shared");
  return { ...actual, sharedAPI: { list: jest.fn() } };
});

jest.mock("../AttachmentItem", () => ({
  __esModule: true,
  default: ({ attachment }: { attachment: { fileName: string } }) => <span>{attachment.fileName}</span>,
}));

const list = sharedAPI.list as jest.Mock;

const file = (id: string, fileName: string) => ({
  messageId: `m-${id}`,
  authorId: "u2",
  authorName: "Bob",
  createdAt: "2026-10-01T10:00:00Z",
  attachment: { id, fileName, contentType: "application/pdf", sizeBytes: 10, createdAt: "2026-10-01T10:00:00Z" },
});

describe("SharedPanel", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  beforeEach(() => {
    list.mockReset();
  });

  const renderPanel = (onShowMessage = jest.fn()) =>
    render(
      <SharedPanel
        scope={{ kind: "dm", friendId: "u2" }}
        currentUserId="u1"
        refreshKey={0}
        onClose={() => undefined}
        onShowMessage={onShowMessage}
      />,
    );

  it("loads media first and switches tabs", async () => {
    list.mockResolvedValue({ items: [], nextCursor: null });
    renderPanel();
    expect(await screen.findByText("Images and videos from this chat will appear here")).toBeInTheDocument();
    expect(list).toHaveBeenLastCalledWith({ kind: "dm", friendId: "u2" }, "media", expect.objectContaining({ before: null }));
    list.mockResolvedValueOnce({ items: [file("a1", "report.pdf")], nextCursor: null });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("report.pdf")).toBeInTheDocument();
    expect(screen.getByText("Bob")).toBeInTheDocument();
    expect(list).toHaveBeenLastCalledWith({ kind: "dm", friendId: "u2" }, "files", expect.anything());
  });

  it("lists links and jumps to their message", async () => {
    const onShowMessage = jest.fn();
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    list.mockResolvedValueOnce({
      items: [{ messageId: "m9", authorId: "u1", authorName: "Me", createdAt: "2026-10-01T10:00:00Z", url: "https://example.com/a" }],
      nextCursor: null,
    });
    renderPanel(onShowMessage);
    fireEvent.click(screen.getByRole("tab", { name: "Links" }));
    const link = await screen.findByRole("link");
    expect(link).toHaveAttribute("href", "https://example.com/a");
    expect(screen.getByText("example.com")).toBeInTheDocument();
    expect(screen.getByText("You")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Show in chat" }));
    expect(onShowMessage).toHaveBeenCalledWith("m9");
  });

  it("searches after typing and retries after a failure", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    renderPanel();
    await waitFor(() => expect(list).toHaveBeenCalledTimes(1));
    list.mockRejectedValueOnce(new Error("offline"));
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "Report" } });
    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't load");
    expect(list).toHaveBeenLastCalledWith(expect.anything(), "media", expect.objectContaining({ q: "Report" }));
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText("Nothing found")).toBeInTheDocument();
  });
});
