import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppTopBar from "../AppTopBar";
import SearchDropdown from "../SearchDropdown";
import { peopleAPI } from "../../../search/api/people";
import { communityAPI } from "../../api/community";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { useSearchFocusStore } from "../../../search/store/searchFocusStore";
import { queryClient } from "../../../../shared/query/queryClient";

import { searchHistoryAPI } from "../../../search/api/searchHistory";
import { useAuthStore } from "../../../auth/store/authStore";
import { useBlockStore } from "../../../social/store/blockStore";
import { clearStoredHistory, resetSearchHistorySession, storeHistory } from "../../../search/lib/searchHistory";

jest.mock("../../../search/api/people", () => ({ peopleAPI: { search: jest.fn() } }));
jest.mock("../../../search/api/searchHistory", () => {
  const actual = jest.requireActual("../../../search/api/searchHistory");
  return {
    ...actual,
    searchHistoryAPI: { list: jest.fn(), record: jest.fn(), remove: jest.fn(), clear: jest.fn() },
  };
});

jest.mock("../../../search/components/GlobalSearchResults", () => {
  const actual = jest.requireActual("../../../search/components/GlobalSearchResults");
  return { __esModule: true, ...actual, default: () => null };
});

const dm = { id: "u1", name: "Alice", initials: "A", color: "#333", online: true, unread: 0, isFriend: true, friendship: "accepted" };

type Picks = { dm: jest.Mock; community: jest.Mock; channel: jest.Mock; message: jest.Mock; join: jest.Mock };

const Harness: React.FC<{ picks: Picks }> = ({ picks }) => {
  const [query, setQuery] = React.useState("");
  return (
    <AppTopBar
      navState={{ back: false, forward: false }}
      goHistory={jest.fn()}
      homeActive={false}
      onGoHome={jest.fn()}
      query={query}
      onQueryChange={setQuery}
      currentUserName="me"
      currentUserStatus="online"
      onOpenSettings={jest.fn()}
      renderResults={(done) => (
        <SearchDropdown
          query={query}
          directs={[dm as never]}
          communities={[{ id: "c1", name: "Alpha Squad", ownerId: "me" } as never]}
          channelsByCommunity={{ c1: [{ id: "ch1", name: "alerts", kind: "text", communityId: "c1" } as never] }}
          onPickDm={picks.dm}
          onPickCommunity={picks.community}
          onPickChannel={picks.channel}
          onOpenResult={jest.fn()}
          onMessagePerson={picks.message}
          onJoinInvite={picks.join}
          onSearch={setQuery}
          isFriend={() => true}
          onDone={done}
        />
      )}
    />
  );
};

const setup = () => {
  const picks = {
    dm: jest.fn(),
    community: jest.fn(),
    channel: jest.fn(),
    message: jest.fn(),
    join: jest.fn().mockResolvedValue(undefined),
  };
  render(<Harness picks={picks} />);
  return { picks, input: screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholder") }) };
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  queryClient.clear();
  (peopleAPI.search as jest.Mock).mockReset().mockResolvedValue([]);
  jest.spyOn(communityAPI, "previewInvite").mockReset();
  jest.spyOn(communityAPI, "downloadLogo").mockRejectedValue(new Error("no logo"));
  useFriendshipStore.setState({ loaded: false, statusById: {} });
  useAuthStore.setState({ user: null });
  clearStoredHistory();
  const history = searchHistoryAPI as jest.Mocked<typeof searchHistoryAPI>;
  history.list.mockReset().mockResolvedValue([]);
  history.record.mockReset().mockResolvedValue({ id: "saved", usedAt: "2026-10-13T12:00:00Z" });
  history.remove.mockReset().mockResolvedValue();
  history.clear.mockReset().mockResolvedValue();
});

const signIn = () => useAuthStore.setState({ user: { id: "me", name: "me" } as never });

it("lists chats, communities and channels under the search box", () => {
  const { input } = setup();
  fireEvent.change(input, { target: { value: "al" } });
  expect(screen.getByLabelText(i18n.t("sidebar:searchChats"))).toHaveTextContent("Alice");
  expect(screen.getByLabelText(i18n.t("sidebar:searchCommunities"))).toHaveTextContent("Alpha Squad");
  expect(screen.getByLabelText(i18n.t("sidebar:searchChannels"))).toHaveTextContent("alerts");
});

it("opens a result by click and clears the search", () => {
  const { picks, input } = setup();
  fireEvent.change(input, { target: { value: "alpha" } });
  fireEvent.click(screen.getByText("Squad", { exact: false }));
  expect(picks.community).toHaveBeenCalledWith(expect.objectContaining({ id: "c1" }));
  expect(input).toHaveValue("");
  expect(screen.queryByLabelText(i18n.t("sidebar:searchCommunities"))).toBeNull();
});

it("walks the results with arrows and opens with Enter", () => {
  const { picks, input } = setup();
  fireEvent.change(input, { target: { value: "al" } });
  fireEvent.keyDown(input, { key: "ArrowDown" });
  expect(document.activeElement).toHaveTextContent("Alice");
  fireEvent.keyDown(document.activeElement as Element, { key: "ArrowDown" });
  expect(document.activeElement).toHaveTextContent("Alpha Squad");
  fireEvent.keyDown(document.activeElement as Element, { key: "ArrowUp" });
  fireEvent.keyDown(document.activeElement as Element, { key: "ArrowUp" });
  expect(document.activeElement).toBe(input);
  fireEvent.keyDown(input, { key: "Enter" });
  expect(picks.dm).toHaveBeenCalledWith(expect.objectContaining({ id: "u1" }));
});

it("closes on Escape", () => {
  const { input } = setup();
  fireEvent.change(input, { target: { value: "al" } });
  fireEvent.keyDown(input, { key: "Escape" });
  expect(input).toHaveValue("");
  expect(screen.queryByLabelText(i18n.t("sidebar:searchChats"))).toBeNull();
});

it("finds people on the server and acts on each relation", async () => {
  const sendRequest = jest.fn().mockResolvedValue(undefined);
  const accept = jest.fn().mockResolvedValue(undefined);
  const remove = jest.fn().mockResolvedValue(undefined);
  useFriendshipStore.setState({ sendRequest, accept, remove });
  (peopleAPI.search as jest.Mock).mockResolvedValue([
    { id: "u1", name: "Alice", relation: "friend", acceptsFriendRequests: true },
    { id: "p1", name: "Bobby", relation: "none", acceptsFriendRequests: true },
    { id: "p2", name: "Bonnie", relation: "incoming", acceptsFriendRequests: true },
    { id: "p3", name: "Boris", relation: "outgoing", acceptsFriendRequests: true },
    { id: "p4", name: "Bolt", relation: "none", acceptsFriendRequests: false },
  ]);
  const { picks, input } = setup();
  fireEvent.change(input, { target: { value: "Bo" } });
  const section = await screen.findByLabelText(i18n.t("sidebar:searchPeople"));
  await screen.findByText("bby", { exact: false });
  expect(peopleAPI.search).toHaveBeenCalledWith("bo", 8);
  expect(section).toHaveTextContent(i18n.t("sidebar:requestsClosed"));
  const row = (name: string) => screen.getByText(name, { exact: false }).closest(".search-dd-person") as HTMLElement;
  fireEvent.click(row("bby").querySelector(".search-dd-action") as HTMLElement);
  expect(sendRequest).toHaveBeenCalledWith("Bobby");
  fireEvent.click(row("nnie").querySelector(".search-dd-action") as HTMLElement);
  expect(accept).toHaveBeenCalledWith("p2");
  fireEvent.click(row("ris").querySelector(".search-dd-action") as HTMLElement);
  expect(remove).toHaveBeenCalledWith("p3");
  expect(row("lt").querySelector(".search-dd-action")).toBeNull();
  expect(picks.message).not.toHaveBeenCalled();
  await act(async () => {});
});

it("hides people already listed as chats and offers a message to friends", async () => {
  (peopleAPI.search as jest.Mock).mockResolvedValue([
    { id: "u1", name: "Alice", relation: "friend", acceptsFriendRequests: true },
    { id: "f2", name: "Alina", relation: "friend", acceptsFriendRequests: true },
  ]);
  const { picks, input } = setup();
  fireEvent.change(input, { target: { value: "ali" } });
  const section = await screen.findByLabelText(i18n.t("sidebar:searchPeople"));
  await screen.findByText("na", { exact: false, selector: ".search-dd-name, .search-dd-name *" });
  expect(section).not.toHaveTextContent("Alice");
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:peopleMessage") }));
  expect(picks.message).toHaveBeenCalledWith("f2", "Alina", true);
});

it("prefers the live friendship state over the search snapshot", async () => {
  useFriendshipStore.setState({ loaded: true, statusById: { p1: "pending_outgoing" } });
  (peopleAPI.search as jest.Mock).mockResolvedValue([
    { id: "p1", name: "Bobby", relation: "none", acceptsFriendRequests: true },
  ]);
  const { input } = setup();
  fireEvent.change(input, { target: { value: "bob" } });
  expect(await screen.findByText(i18n.t("sidebar:relationOutgoing"))).toBeInTheDocument();
  expect(screen.getByRole("button", { name: i18n.t("sidebar:peopleCancel") })).toBeInTheDocument();
});

it("previews a pasted invite link and joins from the search", async () => {
  const token = "AbCdEfGhIjKlMnOpQrStUv";
  (communityAPI.previewInvite as jest.Mock).mockResolvedValue({
    status: "active",
    community: { id: "c9", name: "Night Owls", description: null, accentColor: null, memberCount: 3 },
  });
  const { picks, input } = setup();
  fireEvent.change(input, { target: { value: `https://rift.app/join/${token}` } });
  expect(await screen.findByText("Night Owls")).toBeInTheDocument();
  expect(communityAPI.previewInvite).toHaveBeenCalledWith(token);
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:inviteJoin") }));
  expect(picks.join).toHaveBeenCalledWith(token);
  await act(async () => {});
  expect(input).toHaveValue("");
});

it("explains a dead invite instead of offering to join", async () => {
  (communityAPI.previewInvite as jest.Mock).mockResolvedValue({ status: "revoked", community: null });
  const { input } = setup();
  fireEvent.change(input, { target: { value: "https://rift.app/join/ZyXwVuTsRqPoNmLkJiHgFe" } });
  expect(await screen.findByText(i18n.t("sidebar:inviteUnavailable"))).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: i18n.t("sidebar:inviteJoin") })).toBeNull();
});

it("focuses the search with a hint when asked from elsewhere", () => {
  setup();
  act(() => useSearchFocusStore.getState().focus("people"));
  const input = screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholderPeople") });
  expect(document.activeElement).toBe(input);
  act(() => useSearchFocusStore.getState().focus("invite"));
  expect(screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholderInvite") })).toBe(document.activeElement);
});

it("drops the previous query's people as soon as the text changes", async () => {
  (peopleAPI.search as jest.Mock).mockImplementation(async (q: string) =>
    q === "bob" ? [{ id: "p1", name: "Bobby", relation: "none", acceptsFriendRequests: true }] : [],
  );
  const { input } = setup();
  fireEvent.change(input, { target: { value: "bob" } });
  await screen.findByText("by", { exact: false, selector: ".search-dd-name, .search-dd-name *" });
  fireEvent.change(input, { target: { value: "bon" } });
  expect(screen.queryByRole("button", { name: i18n.t("sidebar:peopleAdd") })).toBeNull();
  expect(screen.getByText(i18n.t("sidebar:peopleSearching"))).toBeInTheDocument();
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 400));
  });
});

it("returns to the plain search hint after a focused search is closed", () => {
  setup();
  act(() => useSearchFocusStore.getState().focus("people"));
  const input = screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholderPeople") });
  fireEvent.change(input, { target: { value: "bo" } });
  fireEvent.keyDown(input, { key: "Escape" });
  expect(screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholder") })).toBe(input);
});

it("shows recent searches in an empty search and reruns a recent query", async () => {
  signIn();
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h1", usedAt: "2026-10-13T10:00:00Z", type: "query", text: "deploy" },
    { id: "h2", usedAt: "2026-10-13T09:00:00Z", type: "community", communityId: "c1", name: "Alpha Squad" },
  ]);
  const { input } = setup();
  fireEvent.focus(input);
  const recent = await screen.findByLabelText(i18n.t("sidebar:recentTitle"));
  expect(recent).toHaveTextContent("deploy");
  fireEvent.click(screen.getByText("deploy"));
  expect(input).toHaveValue("deploy");
  await waitFor(() => expect(searchHistoryAPI.record).toHaveBeenCalledWith({ type: "query", text: "deploy" }));
});

it("opens a recent community and lifts it to the top", async () => {
  signIn();
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h2", usedAt: "2026-10-13T09:00:00Z", type: "community", communityId: "c1", name: "Alpha Squad" },
  ]);
  const { picks, input } = setup();
  fireEvent.focus(input);
  fireEvent.click(await screen.findByText("Alpha Squad"));
  expect(picks.community).toHaveBeenCalledWith(expect.objectContaining({ id: "c1" }));
  await waitFor(() => expect(searchHistoryAPI.record).toHaveBeenCalledWith(expect.objectContaining({ type: "community", communityId: "c1" })));
  await act(async () => {});
});

it("removes one recent entry or clears them all at once", async () => {
  signIn();
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h1", usedAt: "2026-10-13T10:00:00Z", type: "query", text: "deploy" },
    { id: "h3", usedAt: "2026-10-13T08:00:00Z", type: "query", text: "release" },
  ]);
  let answerRemove: () => void = () => {};
  (searchHistoryAPI.remove as jest.Mock).mockReturnValue(new Promise<void>((resolve) => (answerRemove = resolve)));
  let answerClear: () => void = () => {};
  (searchHistoryAPI.clear as jest.Mock).mockReturnValue(new Promise<void>((resolve) => (answerClear = resolve)));
  const { input } = setup();
  fireEvent.focus(input);
  await screen.findByText("deploy");
  fireEvent.click(screen.getAllByRole("button", { name: i18n.t("sidebar:recentRemove") })[0]);
  await waitFor(() => expect(screen.queryByText("deploy")).toBeNull());
  expect(searchHistoryAPI.remove).toHaveBeenCalledWith("h1");
  await act(async () => answerRemove());
  fireEvent.click(screen.getByRole("button", { name: i18n.t("sidebar:recentClear") }));
  await waitFor(() => expect(screen.queryByText("release")).toBeNull());
  expect(searchHistoryAPI.clear).toHaveBeenCalled();
  await act(async () => answerClear());
  expect(screen.getByText(i18n.t("sidebar:recentEmpty"))).toBeInTheDocument();
});

it("remembers what was opened from the results", async () => {
  signIn();
  const { input } = setup();
  fireEvent.change(input, { target: { value: "alpha" } });
  fireEvent.click(screen.getByText("Squad", { exact: false }));
  await waitFor(() => expect(searchHistoryAPI.record).toHaveBeenCalledWith({ type: "community", communityId: "c1", name: "Alpha Squad" }));
  await act(async () => {});
});

it("opens the first recent entry on Enter and never clears the history from the keyboard", async () => {
  signIn();
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h2", usedAt: "2026-10-13T09:00:00Z", type: "community", communityId: "c1", name: "Alpha Squad" },
  ]);
  const { picks, input } = setup();
  fireEvent.focus(input);
  await screen.findByText("Alpha Squad");
  fireEvent.keyDown(input, { key: "ArrowDown" });
  expect(document.activeElement).toHaveTextContent("Alpha Squad");
  fireEvent.keyDown(document.activeElement as Element, { key: "ArrowDown" });
  expect(document.activeElement).toHaveTextContent("Alpha Squad");
  fireEvent.keyDown(document.activeElement as Element, { key: "ArrowUp" });
  fireEvent.keyDown(input, { key: "Enter" });
  expect(picks.community).toHaveBeenCalledWith(expect.objectContaining({ id: "c1" }));
  expect(searchHistoryAPI.clear).not.toHaveBeenCalled();
  await act(async () => {});
});

it("masks recent channel messages from blocked authors", async () => {
  signIn();
  useBlockStore.setState({ loaded: true, blocked: { u9: true } });
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    {
      id: "h5",
      usedAt: "2026-10-13T09:00:00Z",
      type: "message",
      messageId: "m5",
      authorId: "u9",
      authorName: "Mallory",
      snippet: "secret plans",
      createdAt: "2026-10-12T09:00:00Z",
      place: { type: "channel", communityId: "c1", communityName: "Alpha Squad", channelId: "ch1", channelName: "alerts" },
    },
  ]);
  const { input } = setup();
  fireEvent.focus(input);
  expect(await screen.findByText(i18n.t("chat:blockedMessage"))).toBeInTheDocument();
  expect(screen.queryByText("secret plans")).toBeNull();
  expect(screen.queryByText("Mallory", { exact: false })).toBeNull();
  expect(screen.getByText("#alerts")).toBeInTheDocument();
});

it("hides recent DMs with people blocked since they were cached and refetches the history", async () => {
  signIn();
  useBlockStore.setState({ loaded: true, blocked: {} });
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h6", usedAt: "2026-10-13T09:00:00Z", type: "dm", userId: "u7", name: "Trudy" },
    {
      id: "h7",
      usedAt: "2026-10-13T08:00:00Z",
      type: "message",
      messageId: "m7",
      authorId: "u7",
      authorName: "Trudy",
      snippet: "private words",
      createdAt: "2026-10-12T09:00:00Z",
      place: { type: "dm", peerId: "u7", peerName: "Trudy" },
    },
  ]);
  const { input } = setup();
  fireEvent.focus(input);
  expect(await screen.findByText("private words")).toBeInTheDocument();
  (searchHistoryAPI.list as jest.Mock).mockClear();
  act(() => useBlockStore.setState({ blocked: { u7: true } }));
  expect(screen.queryByText("Trudy")).toBeNull();
  expect(screen.queryByText("private words")).toBeNull();
  await waitFor(() => expect(searchHistoryAPI.list).toHaveBeenCalled());
});

it("hides recent entries for people who blocked the viewer", async () => {
  signIn();
  useBlockStore.setState({ loaded: true, blocked: {}, blockedBy: {} });
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    { id: "h8", usedAt: "2026-10-13T09:00:00Z", type: "person", userId: "u8", name: "Oscar" },
  ]);
  const { input } = setup();
  fireEvent.focus(input);
  expect(await screen.findByText("Oscar")).toBeInTheDocument();
  (searchHistoryAPI.list as jest.Mock).mockClear();
  act(() => useBlockStore.setState({ blockedBy: { u8: true } }));
  expect(screen.queryByText("Oscar")).toBeNull();
  await waitFor(() => expect(searchHistoryAPI.list).toHaveBeenCalled());
});

it("holds back cached access-dependent recents until the server has vouched for them", async () => {
  resetSearchHistorySession();
  signIn();
  useBlockStore.setState({ loaded: true, blocked: {}, blockedBy: {} });
  storeHistory("me", [
    { id: "h9", usedAt: "2026-10-13T09:00:00Z", type: "dm", userId: "u9", name: "Eve" },
    { id: "h10", usedAt: "2026-10-13T08:00:00Z", type: "community", communityId: "c1", name: "Alpha Squad" },
    { id: "h11", usedAt: "2026-10-13T07:00:00Z", type: "query", text: "roadmap" },
  ]);
  let answer: (items: unknown[]) => void = () => {};
  (searchHistoryAPI.list as jest.Mock).mockReturnValue(new Promise((resolve) => (answer = resolve)));
  const { input } = setup();
  fireEvent.focus(input);
  expect(await screen.findByText("roadmap")).toBeInTheDocument();
  expect(screen.queryByText("Eve")).toBeNull();
  expect(screen.queryByText("Alpha Squad")).toBeNull();
  await act(async () =>
    answer([{ id: "h9", usedAt: "2026-10-13T09:00:00Z", type: "dm", userId: "u9", name: "Eve" }]),
  );
  expect(await screen.findByText("Eve")).toBeInTheDocument();
  expect(screen.queryByText("Alpha Squad")).toBeNull();
});

it("reveals verified recents as soon as an unchanged history comes back from the server", async () => {
  resetSearchHistorySession();
  signIn();
  const cached = [
    { id: "h12", usedAt: "2026-10-13T09:00:00Z", type: "community", communityId: "c1", name: "Alpha Squad" },
  ];
  storeHistory("me", cached as never);
  let answer: (items: unknown[]) => void = () => {};
  (searchHistoryAPI.list as jest.Mock).mockReturnValue(new Promise((resolve) => (answer = resolve)));
  const { input } = setup();
  fireEvent.focus(input);
  expect(await screen.findByText(i18n.t("sidebar:recentEmpty"))).toBeInTheDocument();
  await act(async () => answer(JSON.parse(JSON.stringify(cached))));
  expect(await screen.findByText("Alpha Squad")).toBeInTheDocument();
});

it("loads a recent channel's community channels before opening it", async () => {
  signIn();
  const { useChannelStore } = jest.requireActual("../../store/channelStore");
  const fetchChannels = jest.fn(async (communityId: string) => {
    useChannelStore.setState((state: { channelsByCommunity: Record<string, unknown[]> }) => ({
      channelsByCommunity: {
        ...state.channelsByCommunity,
        [communityId]: [{ id: "ch9", name: "lore", kind: "text", communityId }],
      },
    }));
    return true;
  });
  useChannelStore.setState({ fetchChannels, channelsByCommunity: {} });
  (searchHistoryAPI.list as jest.Mock).mockResolvedValue([
    {
      id: "h13",
      usedAt: "2026-10-13T09:00:00Z",
      type: "channel",
      communityId: "c1",
      communityName: "Alpha Squad",
      channelId: "ch9",
      name: "lore",
      voice: false,
    },
  ]);
  const { picks, input } = setup();
  fireEvent.focus(input);
  fireEvent.click(await screen.findByText("lore"));
  await waitFor(() =>
    expect(picks.channel).toHaveBeenCalledWith(expect.objectContaining({ id: "c1" }), expect.objectContaining({ id: "ch9" })),
  );
  expect(fetchChannels).toHaveBeenCalledWith("c1");
  expect(picks.community).not.toHaveBeenCalled();
});
