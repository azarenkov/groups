import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import DetailsPanel from "../DetailsPanel";
import ChannelHeader from "../ChannelHeader";
import { useDetailsPanelStore } from "../../store/detailsPanelStore";
import { useNotificationPrefsStore } from "../../../settings/store/notificationPrefsStore";

jest.mock("../SharedPanel", () => ({ __esModule: true, default: () => <div>shared-content</div> }));
jest.mock("../../../communities/components/MembersPanel", () => ({ __esModule: true, default: () => <div>members-content</div> }));
jest.mock("../../../social/hooks/useUserProfileDetails", () => ({
  useUserProfileDetails: () => ({
    profile: null,
    failed: false,
    connections: [],
    blockedByMe: false,
    blockedMe: false,
    friendshipStatus: "accepted",
    bio: "Люблю котов",
  }),
}));
jest.mock("../../../../shared/lib/avatarCache", () => ({ useAvatarUrl: () => null }));

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  window.localStorage.clear();
  act(() => useDetailsPanelStore.setState({ open: true, dmTab: "profile", channelTab: "members" }));
});

it("shows the friend's profile and notification choice for a direct chat", () => {
  const setDmLevel = jest.fn().mockResolvedValue(undefined);
  act(() => useNotificationPrefsStore.setState({ setDmLevel } as never));
  render(<DetailsPanel target={{ kind: "dm", friendId: "u1", name: "Alice" }} onShowMessage={jest.fn()} />);
  expect(screen.getByText("Alice")).toBeInTheDocument();
  expect(screen.getByText("Люблю котов")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("radio", { name: new RegExp(i18n.t("chat:notifications.nothing")) }));
  expect(setDmLevel).toHaveBeenCalledWith("u1", "none");
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("chat:details.shared") }));
  expect(screen.getByText("shared-content")).toBeInTheDocument();
  expect(useDetailsPanelStore.getState().dmTab).toBe("shared");
});

it("shows channel info, members and files for a channel", () => {
  render(
    <DetailsPanel
      target={{ kind: "channel", communityId: "c1", channelId: "ch1", name: "general" }}
      onShowMessage={jest.fn()}
    />,
  );
  expect(screen.getByText("members-content")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: i18n.t("chat:details.channel") }));
  expect(screen.getByText("general")).toBeInTheDocument();
  expect(screen.getAllByRole("radio")).toHaveLength(3);
  fireEvent.click(screen.getByRole("button", { name: i18n.t("chat:details.close") }));
  expect(useDetailsPanelStore.getState().open).toBe(false);
});

it("opens and closes from the chat header and remembers the choice", () => {
  act(() => useDetailsPanelStore.getState().setOpen(false));
  render(<ChannelHeader channel={{ id: "ch1", name: "general" } as never} />);
  expect(screen.queryByTitle(i18n.t("chat:notifications.bell"))).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: i18n.t("chat:details.open") }));
  expect(useDetailsPanelStore.getState().open).toBe(true);
  expect(window.localStorage.getItem("rift.detailsPanel")).toBe("open");
});

it("hides notification choices for a chat with someone who is not a friend", () => {
  act(() => useDetailsPanelStore.getState().showDm("profile"));
  render(<DetailsPanel target={{ kind: "dm", friendId: "u9", name: "Eve", isFriend: false }} onShowMessage={jest.fn()} />);
  expect(screen.queryAllByRole("radio")).toHaveLength(0);
});
