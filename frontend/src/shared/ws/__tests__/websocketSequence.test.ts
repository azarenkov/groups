import { ws, type WsEnvelope } from "../websocket";

afterEach(() => ws.disconnect());

it("hands a repeated socket event to listeners only once", () => {
  const seen: unknown[] = [];
  const off = ws.on((env) => seen.push(env.data));
  const env = { type: "dm_message.sent", data: { id: "m1" }, partition: "messaging:p", seq: 4 } as WsEnvelope;
  ws.receive(env);
  ws.receive(env);
  expect(seen).toEqual([{ id: "m1" }]);
  off();
});

it("passes local events without a sequence straight to listeners", () => {
  const seen: string[] = [];
  const off = ws.on((env) => seen.push(env.type));
  ws.receive({ type: "presence.update", data: {} } as WsEnvelope);
  ws.receive({ type: "presence.update", data: {} } as WsEnvelope);
  expect(seen).toEqual(["presence.update", "presence.update"]);
  off();
});
