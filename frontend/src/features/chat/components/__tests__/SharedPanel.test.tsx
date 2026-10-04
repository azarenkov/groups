import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import SharedPanel from "../SharedPanel";
import { useBlockStore } from "../../../social/store/blockStore";
import { sharedAPI } from "../../api/shared";
import { compareInstants, itemKey, mergeFirstPage, messageItems, reconcileEdit, replaceMessageItems } from "../../hooks/useSharedContent";

jest.mock("../../api/shared", () => {
  const actual = jest.requireActual("../../api/shared");
  return { ...actual, sharedAPI: { list: jest.fn(), message: jest.fn(), markListened: jest.fn() } };
});

const wsHandlers: {
  current: { onSent?: (m: unknown) => void; onEdited?: (m: unknown) => void; onDeleted?: (m: unknown) => void };
} = { current: {} };

jest.mock("../../hooks/useWebSocketMessages", () => ({
  useWsChannelMessages: () => undefined,
  useWsDmMessages: (_peer: string | null, _self: string | undefined, handlers: typeof wsHandlers.current) => {
    wsHandlers.current = handlers;
  },
}));

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

  it("keeps loaded pages when a new file arrives and drops deleted messages", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    renderPanel();
    list.mockResolvedValueOnce({ items: [file("a1", "old.pdf")], nextCursor: "c1" });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("old.pdf")).toBeInTheDocument();
    list.mockResolvedValueOnce({ items: [{ ...file("a2", "new.pdf"), createdAt: "2026-10-02T10:00:00Z" }, file("a1", "old.pdf")], nextCursor: "c2" });
    act(() => wsHandlers.current.onSent?.({ content: "", attachments: [{}] }));
    expect(await screen.findByText("new.pdf")).toBeInTheDocument();
    expect(screen.getAllByText("old.pdf")).toHaveLength(1);
    act(() => wsHandlers.current.onDeleted?.({ id: "m-a1" }));
    expect(screen.queryByText("old.pdf")).toBeNull();
    expect(screen.getByText("new.pdf")).toBeInTheDocument();
  });
});

describe("compareInstants", () => {
  it("orders timestamps with different fractional precision", () => {
    expect(compareInstants("2026-10-01T10:00:00.1Z", "2026-10-01T10:00:00.100100Z")).toBe(-1);
    expect(compareInstants("2026-10-01T10:00:00.100100Z", "2026-10-01T10:00:00.1Z")).toBe(1);
    expect(compareInstants("2026-10-01T10:00:00Z", "2026-10-01T10:00:00.000Z")).toBe(0);
    expect(compareInstants("2026-10-01T10:00:01Z", "2026-10-01T10:00:00.999999Z")).toBe(1);
  });

  it("keeps an older same-second item when merging", () => {
    const older = { messageId: "b", authorId: "u", authorName: "", createdAt: "2026-10-01T10:00:00.100Z", url: "https://old" };
    const edge = { messageId: "a", authorId: "u", authorName: "", createdAt: "2026-10-01T10:00:00.100100Z", url: "https://edge" };
    expect(mergeFirstPage([edge, older], [edge], false).map((i) => i.url)).toEqual(["https://edge", "https://old"]);
  });
});

describe("mergeFirstPage", () => {
  const item = (messageId: string, createdAt: string) => ({ messageId, authorId: "u", authorName: "", createdAt, url: `https://x/${messageId}` });

  it("prepends fresh items and keeps older loaded ones", () => {
    const current = [item("b", "2026-10-02"), item("a", "2026-10-01")];
    const page = [item("c", "2026-10-03"), item("b", "2026-10-02")];
    expect(mergeFirstPage(current, page, false).map((i) => i.messageId)).toEqual(["c", "b", "a"]);
  });

  it("takes the page as is when it is the whole list", () => {
    const current = [item("b", "2026-10-02"), item("a", "2026-10-01")];
    expect(mergeFirstPage(current, [item("b", "2026-10-02")], true).map((i) => i.messageId)).toEqual(["b"]);
  });
});

describe("shared panel edge cases", () => {
  const link = (messageId: string, url: string, createdAt = "2026-10-01") => ({
    messageId,
    authorId: "u",
    authorName: "",
    createdAt,
    url,
  });

  it("keeps siblings of the boundary message when merging", () => {
    const current = [link("m2", "https://b"), link("m2", "https://a"), link("m1", "https://z", "2026-09-01")];
    const page = [link("m3", "https://new", "2026-10-05"), link("m2", "https://b")];
    expect(mergeFirstPage(current, page, false).map((i) => i.url)).toEqual([
      "https://new",
      "https://b",
      "https://a",
      "https://z",
    ]);
  });

  it("keeps the forwarded author when rebuilding edited links", () => {
    const items = [{ ...link("m2", "https://old"), forwardedAuthorId: "blocked" }];
    const edited = reconcileEdit(items, { messageId: "m2", content: "https://new" }, "links", "");
    expect(edited[0].forwardedAuthorId).toBe("blocked");
  });

  it("reconciles an edited message in place", () => {
    const items = [link("m3", "https://c"), link("m2", "https://old2"), link("m2", "https://old1"), link("m1", "https://a")];
    const edited = reconcileEdit(items, { messageId: "m2", content: "now https://one and https://two." }, "links", "");
    expect(edited.map((i) => i.url)).toEqual(["https://c", "https://two", "https://one", "https://a"]);
    const files = [file("x1", "a.pdf"), { ...file("x2", "b.pdf"), messageId: "m-x1" }];
    expect(reconcileEdit(files, { messageId: "m-x1", attachmentIds: ["x2"] }, "files", "").map((i) => i.attachment?.id)).toEqual(["x2"]);
  });

  it("retries a failed refresh instead of loading older pages", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({ items: [file("a1", "old.pdf")], nextCursor: "c1" });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("old.pdf")).toBeInTheDocument();
    list.mockRejectedValueOnce(new Error("offline"));
    act(() => wsHandlers.current.onSent?.({ content: "", attachments: [{}] }));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    list.mockResolvedValueOnce({ items: [{ ...file("a2", "new.pdf"), createdAt: "2026-10-02T10:00:00Z" }, file("a1", "old.pdf")], nextCursor: "c1" });
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText("new.pdf")).toBeInTheDocument();
    expect(list).toHaveBeenLastCalledWith(expect.anything(), "files", expect.objectContaining({ before: null }));
  });

  it("places items gained by an edit among the loaded ones", () => {
    const items = [link("m3", "https://c", "2026-10-03"), link("m1", "https://a", "2026-10-01")];
    const fresh = [link("m2", "https://b", "2026-10-02")];
    expect(replaceMessageItems(items, "m2", fresh, false).map((i) => i.url)).toEqual(["https://c", "https://b", "https://a"]);
    const older = [link("m2", "https://b", "2026-09-01")];
    expect(replaceMessageItems(items, "m2", older, false).map((i) => i.url)).toEqual(["https://c", "https://a"]);
    expect(replaceMessageItems(items, "m2", older, true).map((i) => i.url)).toEqual(["https://c", "https://a", "https://b"]);
  });

  it("builds a message's items for the open tab", () => {
    const message = {
      id: "m1",
      authorId: "u",
      authorName: "Bob",
      createdAt: "2026-10-01",
      content: "https://x.io https://x.io",
      attachments: [
        { id: "a", fileName: "pic.png", contentType: "image/png", sizeBytes: 1, createdAt: "" },
        { id: "b", fileName: "doc.pdf", contentType: "application/pdf", sizeBytes: 1, createdAt: "" },
      ],
    };
    expect(messageItems(message, "media", "").map((i) => i.attachment?.id)).toEqual(["a"]);
    expect(messageItems(message, "files", "doc").map((i) => i.attachment?.id)).toEqual(["b"]);
    const links = messageItems(message, "links", "");
    expect(links.map((i) => i.position)).toEqual([2, 1]);
    expect(new Set(links.map(itemKey)).size).toBe(2);
  });

  it("keeps repeated links apart across page boundaries", () => {
    const first = { ...link("m1", "https://same", "2026-10-01"), position: 2 };
    const second = { ...link("m1", "https://same", "2026-10-01"), position: 1 };
    expect(mergeFirstPage([first, second], [first], false)).toHaveLength(2);
  });

  it("fetches an edited message that gained an attachment", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({
      items: [{ ...file("a3", "new.pdf"), createdAt: "2026-10-03T00:00:00Z" }, { ...file("a1", "old.pdf"), createdAt: "2026-10-01T00:00:00Z" }],
      nextCursor: "older",
    });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("old.pdf")).toBeInTheDocument();
    (sharedAPI.message as jest.Mock).mockResolvedValueOnce({
      id: "m2",
      authorId: "u2",
      authorName: "Bob",
      createdAt: "2026-10-02T00:00:00Z",
      content: "",
      attachments: [{ id: "a2", fileName: "added.pdf", contentType: "application/pdf", sizeBytes: 5, createdAt: "" }],
    });
    list.mockResolvedValueOnce({ items: [{ ...file("a3", "new.pdf"), createdAt: "2026-10-03T00:00:00Z" }], nextCursor: "first" });
    act(() => wsHandlers.current.onEdited?.({ id: "m2", newContent: "", attachments: [{ id: "a2" }] }));
    expect(await screen.findByText("added.pdf")).toBeInTheDocument();
  });

  it("fetches an edited message that gained links and shows them", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({
      items: [link("m3", "https://c.example", "2026-10-03T00:00:00Z"), link("m1", "https://a.example", "2026-10-01T00:00:00Z")],
      nextCursor: "older",
    });
    fireEvent.click(screen.getByRole("tab", { name: "Links" }));
    expect(await screen.findByText("https://a.example")).toBeInTheDocument();
    (sharedAPI.message as jest.Mock).mockResolvedValueOnce({
      id: "m2",
      authorId: "u2",
      authorName: "Bob",
      createdAt: "2026-10-02T00:00:00Z",
      content: "see https://b.example",
    });
    list.mockResolvedValueOnce({ items: [link("m3", "https://c.example", "2026-10-03T00:00:00Z")], nextCursor: "first" });
    act(() => wsHandlers.current.onEdited?.({ id: "m2", newContent: "see https://b.example" }));
    expect(await screen.findByText("https://b.example")).toBeInTheDocument();
    await waitFor(() =>
      expect(Array.from(document.querySelectorAll(".shared-link-url")).map((n) => n.textContent)).toEqual([
        "https://c.example",
        "https://b.example",
        "https://a.example",
      ]),
    );
  });

  it("ignores a link lookup that finishes after switching tabs", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({ items: [link("m3", "https://c.example", "2026-10-03T00:00:00Z")], nextCursor: "older" });
    fireEvent.click(screen.getByRole("tab", { name: "Links" }));
    expect(await screen.findByText("https://c.example")).toBeInTheDocument();
    let resolveMessage: (m: unknown) => void = () => undefined;
    (sharedAPI.message as jest.Mock).mockReturnValueOnce(new Promise((resolve) => (resolveMessage = resolve)));
    list.mockResolvedValueOnce({ items: [link("m3", "https://c.example", "2026-10-03T00:00:00Z")], nextCursor: "older" });
    act(() => wsHandlers.current.onEdited?.({ id: "m2", newContent: "see https://b.example" }));
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("Files from this chat will appear here")).toBeInTheDocument();
    await act(async () => {
      resolveMessage({ id: "m2", authorId: "u2", authorName: "Bob", createdAt: "2026-10-02T00:00:00Z", content: "see https://b.example" });
      await Promise.resolve();
    });
    expect(screen.getByText("Files from this chat will appear here")).toBeInTheDocument();
    expect(screen.queryByText("https://b.example")).toBeNull();
  });

  it("does not bring back a message deleted while a refresh was running", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({ items: [file("a1", "old.pdf")], nextCursor: null });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("old.pdf")).toBeInTheDocument();
    let finish: (page: unknown) => void = () => undefined;
    list.mockReturnValueOnce(new Promise((resolve) => (finish = resolve)));
    act(() => wsHandlers.current.onSent?.({ content: "", attachments: [{}] }));
    act(() => wsHandlers.current.onDeleted?.({ id: "m-a2" }));
    await act(async () => {
      finish({ items: [{ ...file("a2", "gone.pdf"), createdAt: "2026-10-02T10:00:00Z" }, file("a1", "old.pdf")], nextCursor: null });
      await Promise.resolve();
    });
    expect(screen.queryByText("gone.pdf")).toBeNull();
    expect(screen.getByText("old.pdf")).toBeInTheDocument();
  });

  it("applies only the latest lookup when a message is edited twice", async () => {
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
    );
    list.mockResolvedValueOnce({
      items: [link("m9", "https://top.example", "2026-10-05T00:00:00Z"), link("m1", "https://a.example", "2026-10-01T00:00:00Z")],
      nextCursor: "older",
    });
    list.mockResolvedValue({ items: [link("m9", "https://top.example", "2026-10-05T00:00:00Z")], nextCursor: "first" });
    fireEvent.click(screen.getByRole("tab", { name: "Links" }));
    expect(await screen.findByText("https://a.example")).toBeInTheDocument();
    const message = (content: string) => ({ id: "m1", authorId: "u2", authorName: "Bob", createdAt: "2026-10-01T00:00:00Z", content });
    let first: (m: unknown) => void = () => undefined;
    (sharedAPI.message as jest.Mock)
      .mockReturnValueOnce(new Promise((resolve) => (first = resolve)))
      .mockResolvedValueOnce(message("now https://second.example"));
    act(() => wsHandlers.current.onEdited?.({ id: "m1", newContent: "https://first.example" }));
    act(() => wsHandlers.current.onEdited?.({ id: "m1", newContent: "now https://second.example" }));
    expect(await screen.findByText("https://second.example")).toBeInTheDocument();
    await act(async () => {
      first(message("https://first.example"));
      await Promise.resolve();
    });
    expect(screen.getByText("https://second.example")).toBeInTheDocument();
    expect(screen.queryByText("https://first.example")).toBeNull();
    list.mockReset();
  });

  it("hides content from blocked channel members until asked", async () => {
    useBlockStore.setState({ blocked: { u2: true }, loaded: false } as never);
    list.mockResolvedValueOnce({ items: [], nextCursor: null });
    render(
      <SharedPanel
        scope={{ kind: "channel", communityId: "c1", channelId: "ch1" }}
        currentUserId="u1"
        onClose={() => undefined}
        onShowMessage={() => undefined}
      />,
    );
    list.mockResolvedValueOnce({
      items: [file("a1", "abuse.pdf"), { ...file("a0", "forwarded.pdf"), authorId: "u3", forwardedAuthorId: "u2" }],
      nextCursor: null,
    });
    fireEvent.click(screen.getByRole("tab", { name: "Files" }));
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(screen.queryByText("abuse.pdf")).toBeNull();
    expect(screen.queryByText("Message from a blocked user")).toBeNull();
    act(() => useBlockStore.setState({ loaded: true } as never));
    expect(await screen.findAllByText("Message from a blocked user")).toHaveLength(2);
    expect(screen.queryByText("abuse.pdf")).toBeNull();
    expect(screen.queryByText("forwarded.pdf")).toBeNull();
    fireEvent.click(screen.getAllByRole("button", { name: "Show" })[0]);
    expect(screen.getByText("abuse.pdf")).toBeInTheDocument();
    useBlockStore.setState({ blocked: {}, loaded: false } as never);
  });

  it("retries a failed lookup of an edited message", async () => {
    jest.useFakeTimers();
    try {
      list.mockResolvedValueOnce({ items: [], nextCursor: null });
      render(
        <SharedPanel scope={{ kind: "dm", friendId: "u2" }} currentUserId="u1" onClose={() => undefined} onShowMessage={() => undefined} />,
      );
      list.mockResolvedValueOnce({
        items: [link("m9", "https://top.example", "2026-10-05T00:00:00Z"), link("m1", "https://a.example", "2026-10-01T00:00:00Z")],
        nextCursor: "older",
      });
      fireEvent.click(screen.getByRole("tab", { name: "Links" }));
      await act(async () => {
        jest.advanceTimersByTime(300);
        await Promise.resolve();
      });
      expect(await screen.findByText("https://a.example")).toBeInTheDocument();
      list.mockResolvedValue({ items: [link("m9", "https://top.example", "2026-10-05T00:00:00Z")], nextCursor: "first" });
      (sharedAPI.message as jest.Mock)
        .mockRejectedValueOnce(new Error("offline"))
        .mockResolvedValueOnce({ id: "m1", authorId: "u2", authorName: "Bob", createdAt: "2026-10-01T00:00:00Z", content: "https://fixed.example" });
      act(() => wsHandlers.current.onEdited?.({ id: "m1", newContent: "https://fixed.example" }));
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
        jest.advanceTimersByTime(1000);
        await Promise.resolve();
        await Promise.resolve();
      });
      expect(sharedAPI.message).toHaveBeenCalledTimes(2);
      expect(screen.getByText("https://fixed.example")).toBeInTheDocument();
    } finally {
      jest.useRealTimers();
      list.mockReset();
    }
  });

  it("does not page through a channel while blocks are loading", async () => {
    const original = (window as unknown as { IntersectionObserver?: unknown }).IntersectionObserver;
    (window as unknown as { IntersectionObserver: unknown }).IntersectionObserver = class {
      constructor(private callback: (entries: { isIntersecting: boolean }[]) => void) {}
      observe() {
        this.callback([{ isIntersecting: true }]);
      }
      disconnect() {}
    };
    useBlockStore.setState({ blocked: {}, loaded: false } as never);
    try {
      list.mockResolvedValue({ items: [file("a1", "one.pdf")], nextCursor: "more" });
      render(
        <SharedPanel
          scope={{ kind: "channel", communityId: "c1", channelId: "ch1" }}
          currentUserId="u1"
          onClose={() => undefined}
          onShowMessage={() => undefined}
        />,
      );
      await waitFor(() => expect(list).toHaveBeenCalledTimes(1));
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 50));
      });
      expect(list).toHaveBeenCalledTimes(1);
    } finally {
      (window as unknown as { IntersectionObserver?: unknown }).IntersectionObserver = original;
      list.mockReset();
    }
  });
});

