import type { Friendship } from "../../../../shared/types/friendship";
import { acceptedFriendIds, useFriendshipStore } from "../friendshipStore";

const friend = (id: string, status: Friendship["status"] = "accepted"): Friendship => ({
  friendId: id,
  friendName: `name-${id}`,
  status,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
});

describe("friendship store", () => {
  beforeEach(() => {
    useFriendshipStore.getState().clearAll();
  });

  it("renames a loaded row in place", () => {
    useFriendshipStore.setState({ friends: [friend("a"), friend("b")] });
    useFriendshipStore.getState().applyName("a", "renamed");
    expect(useFriendshipStore.getState().friends.map((f) => f.friendName)).toEqual([
      "renamed",
      "name-b",
    ]);
  });

  it("forgets a friendship in both the rows and the status map", () => {
    useFriendshipStore.setState({
      friends: [friend("a"), friend("b")],
      statusById: { a: "accepted", b: "accepted" },
    });
    useFriendshipStore.getState().forgetFriend("a");
    const state = useFriendshipStore.getState();
    expect(state.friends.map((f) => f.friendId)).toEqual(["b"]);
    expect(state.statusById).toEqual({ b: "accepted" });
  });

  it("lists accepted ids from the complete status map", () => {
    useFriendshipStore.setState({
      statusById: { a: "accepted", b: "pending_incoming", c: "accepted" },
    });
    expect(acceptedFriendIds().sort()).toEqual(["a", "c"]);
  });

  it("clears everything on account change", () => {
    useFriendshipStore.setState({
      friends: [friend("a")],
      statusById: { a: "accepted" },
      nextCursor: "cursor",
      loaded: true,
    });
    useFriendshipStore.getState().clearAll();
    const state = useFriendshipStore.getState();
    expect(state.friends).toEqual([]);
    expect(state.statusById).toEqual({});
    expect(state.nextCursor).toBeNull();
    expect(state.loaded).toBe(false);
  });
});
