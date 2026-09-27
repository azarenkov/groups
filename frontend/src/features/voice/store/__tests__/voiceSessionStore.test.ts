import { useVoiceSessionStore } from "../voiceSessionStore";

describe("voiceSessionStore", () => {
  beforeEach(() => useVoiceSessionStore.getState().clearAll());

  it("keeps the first start time of a session", () => {
    const store = useVoiceSessionStore.getState();
    store.begin("c1", "2026-09-24T10:00:00Z");
    store.begin("c1", "2026-09-24T10:05:00Z");
    expect(useVoiceSessionStore.getState().startedAt.c1).toBe("2026-09-24T10:00:00Z");
  });

  it("ends a session", () => {
    const store = useVoiceSessionStore.getState();
    store.begin("c1", "2026-09-24T10:00:00Z");
    store.end("c1");
    expect(useVoiceSessionStore.getState().startedAt.c1).toBeUndefined();
  });

  it("replaces the sessions of the synced channels only", () => {
    const store = useVoiceSessionStore.getState();
    store.begin("stale", "2026-09-24T09:00:00Z");
    store.begin("other", "2026-09-24T09:30:00Z");
    store.replaceForChannels(["stale", "live"], [
      { channelId: "live", startedAt: "2026-09-24T10:00:00Z" },
    ]);
    expect(useVoiceSessionStore.getState().startedAt).toEqual({
      other: "2026-09-24T09:30:00Z",
      live: "2026-09-24T10:00:00Z",
    });
  });

  it("keeps live updates that arrived after the snapshot was requested", () => {
    const store = useVoiceSessionStore.getState();
    const since = useVoiceSessionStore.getState().seq;
    store.begin("joined", "2026-09-24T12:00:00Z");
    store.begin("left", "2026-09-24T11:00:00Z");
    store.end("left");
    useVoiceSessionStore.getState().replaceForChannels(
      ["joined", "left", "quiet"],
      [
        { channelId: "left", startedAt: "2026-09-24T10:00:00Z" },
        { channelId: "quiet", startedAt: "2026-09-24T09:00:00Z" },
      ],
      since,
    );
    expect(useVoiceSessionStore.getState().startedAt).toEqual({
      joined: "2026-09-24T12:00:00Z",
      quiet: "2026-09-24T09:00:00Z",
    });
  });
});
