import { socialHandlers } from "../socialHandlers";
import { useAuthStore } from "../../../auth/store/authStore";
import { usePresenceStore } from "../../store/presenceStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { userAPI } from "../../api/user";

jest.mock("../../api/user", () => {
  const actual = jest.requireActual("../../api/user");
  return {
    ...actual,
    userAPI: {
      ...actual.userAPI,
      presenceSnapshot: jest.fn(),
    },
  };
});

const snapshot = userAPI.presenceSnapshot as jest.Mock;

beforeEach(() => {
  snapshot.mockReset();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
  usePresenceStore.getState().clearAll();
  useFriendshipStore.setState({ fetchFriends: jest.fn(() => Promise.resolve()) } as never);
});

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

it("shows a new friend as online right after they accept", async () => {
  snapshot.mockResolvedValue({ online: ["friend"] });
  socialHandlers["friendship.requestAccepted"]!({ accepterId: "friend", senderId: "me" }, {} as never);
  await settle();
  await settle();
  expect(snapshot).toHaveBeenCalledWith(["friend"]);
  expect(usePresenceStore.getState().online.friend).toBe(true);
});

it("checks the sender's status on the side that accepted", async () => {
  snapshot.mockResolvedValue({ online: ["sender"] });
  socialHandlers["friendship.requestAccepted"]!({ accepterId: "me", senderId: "sender" }, {} as never);
  await settle();
  await settle();
  expect(snapshot).toHaveBeenCalledWith(["sender"]);
  expect(usePresenceStore.getState().online.sender).toBe(true);
});

it("does not let a slow status check undo a newer offline event", async () => {
  let resolve: (v: { online: string[] }) => void = () => undefined;
  snapshot.mockImplementation(() => new Promise((r) => (resolve = r)));
  socialHandlers["friendship.requestAccepted"]!({ accepterId: "friend", senderId: "me" }, {} as never);
  await settle();
  socialHandlers["user.online"]!({ userId: "friend" }, {} as never);
  socialHandlers["user.offline"]!({ userId: "friend" }, {} as never);
  resolve({ online: ["friend"] });
  await settle();
  await settle();
  expect(usePresenceStore.getState().online.friend).toBeUndefined();
});

it("does not restore a status checked before presence was reset", async () => {
  let resolve: (v: { online: string[] }) => void = () => undefined;
  snapshot.mockImplementation(() => new Promise((r) => (resolve = r)));
  socialHandlers["friendship.requestAccepted"]!({ accepterId: "friend", senderId: "me" }, {} as never);
  await settle();
  usePresenceStore.getState().clearAll();
  resolve({ online: ["friend"] });
  await settle();
  await settle();
  expect(usePresenceStore.getState().online.friend).toBeUndefined();
});
