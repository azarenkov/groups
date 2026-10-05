import {
  SIDEBAR_SNAPSHOT_KEY,
  SNAPSHOT_SAVE_DELAY_MS,
  hydrateSidebar,
  readSidebarSnapshot,
  startSidebarSnapshots,
} from "../sidebarSnapshot";
import { useCommunityStore } from "../../store/communityStore";
import { useConversationPinsStore } from "../../store/conversationPinsStore";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { useDmPreviewStore } from "../../../dm/store/dmPreviewStore";
import { useChannelStore } from "../../store/channelStore";

const community = { id: "c1", name: "Rift", ownerId: "u1" } as never;
const friend = { friendId: "u2", friendName: "Bob", status: "accepted" } as never;
const preview = {
  messageId: "m1",
  senderId: "u2",
  content: "hi",
  callStatus: null,
  inviteCommunityId: null,
  hasAttachments: false,
  createdAt: "2026-10-01T10:00:00Z",
};

beforeEach(() => {
  localStorage.clear();
  useCommunityStore.setState({ communities: [], isLoading: false, pagination: null });
  useFriendshipStore.setState({ friends: [], statusById: {}, loaded: false, isLoading: false });
  useDmPreviewStore.getState().clear();
  useConversationPinsStore.getState().clearAll();
});

it("saves the sidebar once friends and chats have loaded and restores it on the next start", () => {
  jest.useFakeTimers();
  useCommunityStore.setState({ communities: [community] });
  useFriendshipStore.setState({ friends: [friend], statusById: { u2: "accepted", u3: "pending_incoming" }, loaded: true });
  useDmPreviewStore.getState().setAll([{ ...preview, peerId: "u2" }], true);
  const stop = startSidebarSnapshots("me");
  jest.advanceTimersByTime(SNAPSHOT_SAVE_DELAY_MS);
  expect(localStorage.getItem(SIDEBAR_SNAPSHOT_KEY)).toBeNull();
  useDmPreviewStore.setState({ loaded: true });
  jest.advanceTimersByTime(SNAPSHOT_SAVE_DELAY_MS);
  stop();
  jest.useRealTimers();
  const saved = readSidebarSnapshot()!;
  expect(saved.userId).toBe("me");
  expect(saved.statusById).toEqual({ u2: "accepted" });

  useCommunityStore.setState({ communities: [] });
  useFriendshipStore.setState({ friends: [], statusById: {}, loaded: false });
  useDmPreviewStore.getState().clear();
  expect(hydrateSidebar("me")).toBe(true);
  expect(useCommunityStore.getState().communities).toEqual([community]);
  expect(useFriendshipStore.getState()).toMatchObject({ friends: [friend], loaded: false });
  expect(useDmPreviewStore.getState()).toMatchObject({ loaded: false });
  expect(useDmPreviewStore.getState().byFriend.u2.content).toBe("hi");
});

it("ignores and removes a snapshot of another account", () => {
  localStorage.setItem(
    SIDEBAR_SNAPSHOT_KEY,
    JSON.stringify({ v: 1, userId: "someone", communities: [community], friends: [], statusById: {}, previews: [], pins: { dms: [], communities: [] } }),
  );
  expect(hydrateSidebar("me")).toBe(false);
  expect(useCommunityStore.getState().communities).toEqual([]);
  expect(localStorage.getItem(SIDEBAR_SNAPSHOT_KEY)).toBeNull();
});

it("does not overwrite data that already came from the server", () => {
  localStorage.setItem(
    SIDEBAR_SNAPSHOT_KEY,
    JSON.stringify({ v: 1, userId: "me", communities: [community], friends: [friend], statusById: {}, previews: [], pins: { dms: ["old"], communities: [] } }),
  );
  useFriendshipStore.setState({ friends: [], loaded: true });
  useConversationPinsStore.getState().applyRemote({ dms: ["fresh"], communities: [] });
  hydrateSidebar("me");
  expect(useFriendshipStore.getState().friends).toEqual([]);
  expect(useConversationPinsStore.getState().dms).toEqual(["fresh"]);
});

it("treats a broken snapshot as missing", () => {
  localStorage.setItem(SIDEBAR_SNAPSHOT_KEY, "{not json");
  expect(hydrateSidebar("me")).toBe(false);
});

it("restores channels of fully loaded communities without marking them loaded", async () => {
  const channel = { id: "ch1", communityId: "c1", name: "general", kind: "text" } as never;
  localStorage.setItem(
    SIDEBAR_SNAPSHOT_KEY,
    JSON.stringify({
      v: 1,
      userId: "me",
      communities: [community],
      channels: { c1: [channel] },
      friends: [],
      statusById: {},
      previews: [],
      pins: { dms: [], communities: [] },
    }),
  );
  hydrateSidebar("me");
  expect(useChannelStore.getState().channelsByCommunity.c1).toEqual([channel]);
  const { channelsFullyLoaded } = jest.requireActual("../../store/channelStore");
  expect(channelsFullyLoaded("c1")).toBe(false);
});
