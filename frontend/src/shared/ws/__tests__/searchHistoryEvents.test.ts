import { touchesSearchHistory } from "../websocketBootstrap";
import { useAuthStore } from "../../../features/auth/store/authStore";

const env = (type: string, data: Record<string, unknown> = {}) => ({ type, data }) as never;

it("refreshes history on events that change what its targets may show", () => {
  useAuthStore.setState({ user: { id: "me" } as never });
  for (const type of ["message.edited", "dm_message.edited", "message.deleted", "message.hidden", "dm_message.deleted", "dm_conversation.cleared", "channel.deleted", "community.deleted", "user.renamed"]) {
    expect(touchesSearchHistory(env(type))).toBe(true);
  }
  expect(touchesSearchHistory(env("community.memberLeft", { userId: "me", communityId: "c1" }))).toBe(true);
  expect(touchesSearchHistory(env("community.memberLeft", { userId: "other", communityId: "c1" }))).toBe(false);
  expect(touchesSearchHistory(env("message.sent"))).toBe(false);
});
