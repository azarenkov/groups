import { friendshipAPI } from "../../../social/api/friendship";
import { useProfileModalStore } from "../../../social/store/profileModalStore";
import {
  clearMentionTargets,
  openMentionedProfile,
  rememberMentionTarget,
  renameMentionTarget,
} from "../mentionTargets";

jest.mock("../../../social/api/friendship", () => ({
  friendshipAPI: { searchUsers: jest.fn() },
}));

const mockedApi = friendshipAPI as jest.Mocked<typeof friendshipAPI>;

describe("mention targets", () => {
  beforeEach(() => {
    clearMentionTargets();
    useProfileModalStore.getState().close();
    mockedApi.searchUsers.mockReset();
  });

  it("opens a remembered author without asking the server", async () => {
    rememberMentionTarget("Alexey", "u1");
    await openMentionedProfile("alexey");
    expect(mockedApi.searchUsers).not.toHaveBeenCalled();
    expect(useProfileModalStore.getState().userId).toBe("u1");
  });

  it("falls back to an exact search match", async () => {
    mockedApi.searchUsers.mockResolvedValueOnce([
      { id: "u9", name: "Alex" },
      { id: "u2", name: "Alexey" },
    ]);
    await openMentionedProfile("Alexey");
    expect(useProfileModalStore.getState().userId).toBe("u2");
  });

  it("stays closed when nobody matches exactly", async () => {
    mockedApi.searchUsers.mockResolvedValueOnce([{ id: "u9", name: "Alexeyy" }]);
    await openMentionedProfile("Alexey");
    expect(useProfileModalStore.getState().userId).toBeNull();
  });
});

describe("mention targets after renames", () => {
  beforeEach(() => {
    clearMentionTargets();
    useProfileModalStore.getState().close();
    mockedApi.searchUsers.mockReset();
  });

  it("forgets the old name once a user renames", async () => {
    rememberMentionTarget("Alexey", "u1");
    renameMentionTarget("u1", "Lex");
    mockedApi.searchUsers.mockResolvedValueOnce([{ id: "u2", name: "Alexey" }]);
    await openMentionedProfile("Alexey");
    expect(useProfileModalStore.getState().userId).toBe("u2");
    useProfileModalStore.getState().close();
    await openMentionedProfile("Lex");
    expect(mockedApi.searchUsers).toHaveBeenCalledTimes(1);
    expect(useProfileModalStore.getState().userId).toBe("u1");
  });
});
