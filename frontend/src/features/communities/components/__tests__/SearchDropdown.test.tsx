import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import AppTopBar from "../AppTopBar";
import SearchDropdown from "../SearchDropdown";

jest.mock("../../../search/components/GlobalSearchResults", () => {
  const actual = jest.requireActual("../../../search/components/GlobalSearchResults");
  return { __esModule: true, ...actual, default: () => null };
});

const dm = { id: "u1", name: "Alice", initials: "A", color: "#333", online: true, unread: 0, isFriend: true, friendship: "accepted" };

const Harness: React.FC<{ picks: { dm: jest.Mock; community: jest.Mock; channel: jest.Mock } }> = ({ picks }) => {
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
          onDone={done}
        />
      )}
    />
  );
};

const setup = () => {
  const picks = { dm: jest.fn(), community: jest.fn(), channel: jest.fn() };
  render(<Harness picks={picks} />);
  return { picks, input: screen.getByRole("textbox", { name: i18n.t("sidebar:searchPlaceholder") }) };
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("lists people, communities and channels under the search box", () => {
  const { input } = setup();
  fireEvent.change(input, { target: { value: "al" } });
  expect(screen.getByLabelText(i18n.t("sidebar:searchPeople"))).toHaveTextContent("Alice");
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
  expect(screen.queryByLabelText(i18n.t("sidebar:searchPeople"))).toBeNull();
});
