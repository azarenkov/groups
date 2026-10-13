import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import GlobalSearchResults, { hitLink, highlightTerms } from "../GlobalSearchResults";
import { globalSearchAPI, type GlobalSearchHit } from "../../api/globalSearch";
import { normalizeGlobalQuery } from "../../hooks/useGlobalSearch";
import { useBlockStore } from "../../../social/store/blockStore";

jest.mock("../../api/globalSearch", () => {
  const actual = jest.requireActual("../../api/globalSearch");
  return { ...actual, globalSearchAPI: { search: jest.fn() } };
});

const search = globalSearchAPI.search as jest.Mock;

const channelHit = (id: string, extra: Partial<GlobalSearchHit> = {}): GlobalSearchHit => ({
  messageId: id,
  authorId: "u2",
  authorName: "Bob",
  createdAt: "2026-10-01T10:00:00Z",
  place: { type: "channel", communityId: "c1", communityName: "Rift", channelId: "ch1", channelName: "general" },
  snippet: `hello ${id}`,
  ...extra,
});

const dmHit = (id: string): GlobalSearchHit => ({
  messageId: id,
  authorId: "u1",
  authorName: "Me",
  createdAt: "2026-10-01T10:00:00Z",
  place: { type: "dm", peerId: "u3", peerName: "Carol" },
  snippet: `hello ${id}`,
});

const page = (items: GlobalSearchHit[], nextCursor: string | null = null) => Promise.resolve({ items, nextCursor });

describe("GlobalSearchResults", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  beforeEach(() => {
    jest.useFakeTimers();
    search.mockReset();
    useBlockStore.setState({ blocked: {}, loaded: true });
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  const flush = async () => {
    await act(async () => {
      jest.advanceTimersByTime(300);
    });
  };

  it("does not search for short queries", async () => {
    render(<GlobalSearchResults query=" h " currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    expect(search).not.toHaveBeenCalled();
    expect(screen.queryByText("Messages")).toBeNull();
  });

  it("debounces typing and searches every kind once", async () => {
    search.mockImplementation(() => page([]));
    const { rerender } = render(<GlobalSearchResults query="he" currentUserId="u1" onOpen={jest.fn()} />);
    rerender(<GlobalSearchResults query="hel" currentUserId="u1" onOpen={jest.fn()} />);
    rerender(<GlobalSearchResults query="hello" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    expect(search.mock.calls.map((c) => [c[0], c[1]])).toEqual([
      ["hello", "messages"],
      ["hello", "files"],
      ["hello", "links"],
    ]);
    await waitFor(() => expect(screen.getAllByText("Nothing found")).toHaveLength(3));
  });

  it("reports a visit when the link itself is opened", async () => {
    search.mockImplementation((_q: string, kind: string) =>
      page(kind === "links" ? [channelHit("l1", { url: "https://rift.dev/docs" } as Partial<GlobalSearchHit>)] : []),
    );
    const onOpen = jest.fn();
    const onVisitLink = jest.fn();
    render(<GlobalSearchResults query="docs" currentUserId="u1" onOpen={onOpen} onVisitLink={onVisitLink} />);
    await flush();
    const anchor = await screen.findByRole("link");
    fireEvent.click(anchor);
    expect(onVisitLink).toHaveBeenCalledWith(expect.objectContaining({ messageId: "l1" }));
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("opens the message and loads more pages", async () => {
    search.mockImplementation((_q: string, kind: string, opts: { before?: string }) => {
      if (kind !== "messages") return page([]);
      return opts.before ? page([dmHit("m3")]) : page([channelHit("m1"), dmHit("m2")], "cur");
    });
    const onOpen = jest.fn();
    render(<GlobalSearchResults query="hello" currentUserId="u1" onOpen={onOpen} />);
    await flush();
    await screen.findByText("#general · Rift");
    fireEvent.click(screen.getByText("#general · Rift"));
    expect(onOpen).toHaveBeenCalledWith({ kind: "channel", communityId: "c1", channelId: "ch1", messageId: "m1" }, expect.objectContaining({ messageId: expect.any(String) }));
    expect(screen.getAllByText("Carol")).toHaveLength(1);
    expect(screen.getByText("You:", { exact: false })).toBeTruthy();
    fireEvent.click(screen.getByText("Show more"));
    await waitFor(() => expect(screen.getAllByText("Carol")).toHaveLength(2));
    expect(search).toHaveBeenLastCalledWith("hello", "messages", { before: "cur", limit: 20 });
    expect(screen.queryByText("Show more")).toBeNull();
  });

  it("drops responses for an outdated query", async () => {
    let resolveOld: (value: { items: GlobalSearchHit[]; nextCursor: null }) => void = () => undefined;
    search.mockImplementation((q: string, kind: string) => {
      if (kind !== "messages") return page([]);
      if (q === "old") return new Promise((resolve) => (resolveOld = resolve));
      return page([channelHit("fresh")]);
    });
    const { rerender } = render(<GlobalSearchResults query="old" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    rerender(<GlobalSearchResults query="new" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    await screen.findByText("fresh", { exact: false });
    await act(async () => resolveOld({ items: [channelHit("stale")], nextCursor: null }));
    expect(screen.queryByText("stale", { exact: false })).toBeNull();
  });

  it("hides results of the previous query while the next one is debounced", async () => {
    let resolveOld: (value: { items: GlobalSearchHit[]; nextCursor: null }) => void = () => undefined;
    search.mockImplementation((q: string, kind: string) => {
      if (kind !== "messages") return page([]);
      if (q === "old") return new Promise((resolve) => (resolveOld = resolve));
      return page([]);
    });
    const { rerender } = render(<GlobalSearchResults query="old" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    rerender(<GlobalSearchResults query="older" currentUserId="u1" onOpen={jest.fn()} />);
    await act(async () => resolveOld({ items: [channelHit("stale")], nextCursor: null }));
    expect(screen.queryByText("stale", { exact: false })).toBeNull();
  });

  it("shows who posted files and keeps the DM peer name for opening", async () => {
    search.mockImplementation((_q: string, kind: string) =>
      kind === "files"
        ? page([
            {
              ...dmHit("f1"),
              authorId: "u3",
              authorName: "Carol",
              attachment: { id: "a1", fileName: "report.pdf", contentType: "application/pdf", sizeBytes: 10, createdAt: "2026-10-01T10:00:00Z" },
            },
          ])
        : page([]),
    );
    const onOpen = jest.fn();
    render(<GlobalSearchResults query="report" currentUserId="u1" onOpen={onOpen} />);
    await flush();
    await screen.findByText("Carol:", { exact: false });
    fireEvent.click(screen.getByText("Carol:", { exact: false }));
    expect(onOpen).toHaveBeenCalledWith({ kind: "dm", peerId: "u3", messageId: "f1", peerName: "Carol" }, expect.objectContaining({ messageId: expect.any(String) }));
  });

  it("collapses blocked authors in channels", async () => {
    useBlockStore.setState({ blocked: { u9: true }, loaded: true });
    search.mockImplementation((_q: string, kind: string) =>
      kind === "messages" ? page([channelHit("m1", { forwardedAuthorId: "u9", snippet: "secret words" })]) : page([]),
    );
    render(<GlobalSearchResults query="secret" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    await screen.findByText(i18n.t("chat:blockedMessage"));
    expect(screen.queryByText("words", { exact: false })).toBeNull();
  });

  it("offers a retry when a section fails", async () => {
    let fail = true;
    search.mockImplementation((_q: string, kind: string) =>
      kind === "files" && fail ? Promise.reject(new Error("boom")) : page([]),
    );
    render(<GlobalSearchResults query="report" currentUserId="u1" onOpen={jest.fn()} />);
    await flush();
    await screen.findByText("Search failed");
    fail = false;
    fireEvent.click(screen.getByText("Retry"));
    await waitFor(() => expect(screen.getAllByText("Nothing found")).toHaveLength(3));
    expect(screen.queryByText("Search failed")).toBeNull();
  });
});

describe("GlobalSearchResults cache", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("reuses results when the same text is searched again", async () => {
    jest.useFakeTimers();
    search.mockReset();
    search.mockImplementation(() => page([]));
    const { rerender } = render(<GlobalSearchResults query="hello" currentUserId="u1" onOpen={jest.fn()} />);
    await act(async () => {
      jest.advanceTimersByTime(300);
    });
    expect(search).toHaveBeenCalledTimes(3);
    rerender(<GlobalSearchResults query="hello world" currentUserId="u1" onOpen={jest.fn()} />);
    await act(async () => {
      jest.advanceTimersByTime(300);
    });
    expect(search).toHaveBeenCalledTimes(6);
    rerender(<GlobalSearchResults query="hello" currentUserId="u1" onOpen={jest.fn()} />);
    await act(async () => {
      jest.advanceTimersByTime(300);
    });
    expect(search).toHaveBeenCalledTimes(6);
    jest.useRealTimers();
  });
});

describe("GlobalSearchResults retry", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });

  it("retries a failed refresh instead of loading another page", async () => {
    const { queryClient } = jest.requireActual("../../../../shared/query/queryClient");
    jest.useFakeTimers();
    search.mockReset();
    search.mockImplementation((_q: string, kind: string) =>
      kind === "files" ? page([channelHit("f1")], "next") : page([]),
    );
    render(<GlobalSearchResults query="report" currentUserId="u1" onOpen={jest.fn()} />);
    await act(async () => {
      jest.advanceTimersByTime(300);
    });
    search.mockImplementation((_q: string, kind: string) =>
      kind === "files" ? Promise.reject(new Error("down")) : page([]),
    );
    await act(async () => {
      await queryClient.invalidateQueries();
    });
    await screen.findByText("Search failed");
    search.mockClear();
    search.mockImplementation(() => page([]));
    fireEvent.click(screen.getByText("Retry"));
    await act(async () => {
      await Promise.resolve();
    });
    expect(search.mock.calls.every((c: unknown[]) => !(c[2] as { before?: string }).before)).toBe(true);
    jest.useRealTimers();
  });
});

describe("global search helpers", () => {
  it("normalizes queries to the server limits", () => {
    expect(normalizeGlobalQuery("  hi ")).toBe("hi");
    expect(normalizeGlobalQuery("😀")).toBeNull();
    expect(normalizeGlobalQuery("x".repeat(101))).toBeNull();
  });

  it("links DM hits to the peer", () => {
    expect(hitLink(dmHit("m1"))).toEqual({ kind: "dm", peerId: "u3", messageId: "m1", peerName: "Carol" });
  });

  it("highlights every term without treating it as a pattern", () => {
    const { container } = render(<span>{highlightTerms("a+b and C.d", "c.d A+B")}</span>);
    expect(Array.from(container.querySelectorAll("mark")).map((m) => m.textContent)).toEqual(["a+b", "C.d"]);
  });
});
