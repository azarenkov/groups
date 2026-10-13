import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppTopBar from "../AppTopBar";
import SearchDropdown from "../SearchDropdown";
import { peopleAPI } from "../../../search/api/people";
import { communityAPI } from "../../api/community";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { useSearchFocusStore } from "../../../search/store/searchFocusStore";
import { queryClient } from "../../../../shared/query/queryClient";

jest.mock("../../../search/api/people", () => ({ peopleAPI: { search: jest.fn() } }));

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
});

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
  expect(picks.message).toHaveBeenCalledWith("f2", "Alina");
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
