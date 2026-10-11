import { clearGeneration, clearWatermarkGate, clearWatermarks, loadAcrossClears, sendAcrossClears, wasClearedAway } from "../clearWatermarks";
import { dmAPI } from "../../api/dm";
import { useAuthStore } from "../../../auth/store/authStore";
import { ws } from "../../../../shared/ws/websocket";

jest.mock("../../api/dm", () => ({ dmAPI: { getMessage: jest.fn() } }));

const getMessage = dmAPI.getMessage as jest.Mock;

const cleared = (at: string) => ({
  type: "dm_conversation.cleared" as const,
  data: { participants: ["me", "peer"], scope: "everyone", clearedAt: at },
});
const sent = (id: string, at: string) => ({
  type: "dm_message.sent" as const,
  data: { id, senderId: "peer", recipientId: "me", content: "hi", occurredAt: at },
});

beforeEach(() => {
  clearWatermarks();
  getMessage.mockReset();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
});

it("lets messages through when nothing was cleared", () => {
  expect(clearWatermarkGate(sent("m1", "2026-10-11T09:00:00Z"))).toBeNull();
});

it("lets messages newer than the clear through without asking the server", () => {
  clearWatermarkGate(cleared("2026-10-11T10:00:00Z"));
  expect(clearWatermarkGate(sent("m2", "2026-10-11T10:00:01Z"))).toBeNull();
  expect(getMessage).not.toHaveBeenCalled();
});

it("drops a stale message the server no longer has", async () => {
  clearWatermarkGate(cleared("2026-10-11T10:00:00Z"));
  getMessage.mockRejectedValue({ status: 404 });
  await expect(clearWatermarkGate(sent("m3", "2026-10-11T09:00:00Z"))).resolves.toBe(false);
  expect(getMessage).toHaveBeenCalledWith("peer", "m3");
});

it("keeps a message that survived the clear", async () => {
  clearWatermarkGate(cleared("2026-10-11T10:00:00Z"));
  getMessage.mockResolvedValue({ id: "m4" });
  await expect(clearWatermarkGate(sent("m4", "2026-10-11T09:59:59Z"))).resolves.toBe(true);
});

it("shows the message when the check fails for another reason", async () => {
  clearWatermarkGate(cleared("2026-10-11T10:00:00Z"));
  getMessage.mockRejectedValue({ status: 503 });
  await expect(clearWatermarkGate(sent("m5", "2026-10-11T09:00:00Z"))).resolves.toBe(true);
});

it("holds a stale event back from listeners until the server answers", async () => {
  const off = ws.addGate(clearWatermarkGate);
  const seen: string[] = [];
  const unlisten = ws.on((env) => seen.push(env.type));
  try {
    ws.deliver(cleared("2026-10-11T11:00:00Z"));
    getMessage.mockRejectedValue({ status: 404 });
    ws.deliver(sent("m6", "2026-10-11T10:30:00Z"));
    await Promise.resolve();
    await Promise.resolve();
    expect(seen).toEqual(["dm_conversation.cleared"]);
  } finally {
    unlisten();
    off();
  }
});

it("keeps later events behind a pending check so their order holds", async () => {
  const off = ws.addGate(clearWatermarkGate);
  const seen: string[] = [];
  const unlisten = ws.on((env) => seen.push(`${env.type}:${(env.data as { id?: string }).id ?? ""}`));
  let answer!: (v: unknown) => void;
  getMessage.mockReturnValue(new Promise((r) => (answer = r)));
  try {
    ws.deliver(cleared("2026-10-11T12:00:00Z"));
    ws.deliver(sent("m7", "2026-10-11T11:59:00Z"));
    ws.deliver({ type: "dm_message.edited", data: { id: "m7", newContent: "edited" } });
    ws.deliver(sent("m8", "2026-10-11T12:00:05Z"));
    expect(seen).toEqual(["dm_conversation.cleared:"]);
    answer({ id: "m7" });
    for (let i = 0; i < 5; i += 1) await Promise.resolve();
    expect(seen).toEqual([
      "dm_conversation.cleared:",
      "dm_message.sent:m7",
      "dm_message.edited:m7",
      "dm_message.sent:m8",
    ]);
  } finally {
    unlisten();
    off();
  }
});

it("reloads history that was read before a clear landed", async () => {
  const load = jest
    .fn()
    .mockImplementationOnce(async () => {
      clearWatermarkGate(cleared("2026-10-11T13:00:00Z"));
      return ["stale"];
    })
    .mockResolvedValueOnce(["fresh"]);
  await expect(loadAcrossClears("peer", load)).resolves.toEqual(["fresh"]);
  expect(load).toHaveBeenCalledTimes(2);
  expect(clearGeneration("peer")).toBe(1);
});

it("marks a sent message that a concurrent clear removed", async () => {
  getMessage.mockRejectedValue({ status: 404 });
  const msg = await sendAcrossClears("peer", async () => {
    clearWatermarkGate(cleared("2026-10-11T14:00:00Z"));
    return { id: "s1" } as never;
  });
  expect(msg.id).toBe("s1");
  expect(wasClearedAway("s1")).toBe(true);
});

it("keeps a sent message that survived a concurrent clear", async () => {
  getMessage.mockResolvedValue({ id: "s2" });
  await sendAcrossClears("peer", async () => {
    clearWatermarkGate(cleared("2026-10-11T15:00:00Z"));
    return { id: "s2" } as never;
  });
  expect(wasClearedAway("s2")).toBe(false);
});

it("does not ask the server when no clear happened during a send", async () => {
  await sendAcrossClears("peer", async () => ({ id: "s3" }) as never);
  expect(getMessage).not.toHaveBeenCalled();
  expect(wasClearedAway("s3")).toBe(false);
});
