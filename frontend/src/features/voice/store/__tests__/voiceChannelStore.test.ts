import {
  leaveVoiceChannel,
  useVoiceChannelStore,
} from "../voiceChannelStore";

describe("leaving a voice channel before a call", () => {
  beforeEach(() => {
    useVoiceChannelStore.getState().setSession(null, null);
  });

  it("does nothing when no channel is joined", async () => {
    await expect(leaveVoiceChannel()).resolves.toBe(false);
  });

  it("runs the registered leave and forgets the channel", async () => {
    const order: string[] = [];
    useVoiceChannelStore.getState().setSession("general", async () => {
      order.push("left");
    });

    await expect(leaveVoiceChannel()).resolves.toBe(true);
    expect(order).toEqual(["left"]);
    expect(useVoiceChannelStore.getState().name).toBeNull();
  });

  it("waits for the leave to finish before resolving", async () => {
    const order: string[] = [];
    useVoiceChannelStore.getState().setSession("general", async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));
      order.push("disconnected");
    });

    await leaveVoiceChannel();
    order.push("joined the call");

    expect(order).toEqual(["disconnected", "joined the call"]);
  });

  it("does not block the call when leaving fails", async () => {
    useVoiceChannelStore.getState().setSession("general", async () => {
      throw new Error("room already gone");
    });

    await expect(leaveVoiceChannel()).resolves.toBe(true);
    expect(useVoiceChannelStore.getState().name).toBeNull();
  });

  it("keeps no stale leave when the channel is cleared", () => {
    useVoiceChannelStore.getState().setSession("general", async () => {});
    useVoiceChannelStore.getState().setSession(null, async () => {});

    expect(useVoiceChannelStore.getState().leaveFn).toBeNull();
  });
});
