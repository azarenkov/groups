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

it("keeps events a filter rejects away from listeners", () => {
  const seen: string[] = [];
  const off = ws.on((env) => seen.push(env.type));
  const offFilter = ws.addFilter((env) => env.type !== "dm_message.sent");
  ws.deliver({ type: "dm_message.sent", data: {} } as WsEnvelope);
  ws.deliver({ type: "dm.read", data: {} } as WsEnvelope);
  offFilter();
  ws.deliver({ type: "dm_message.sent", data: {} } as WsEnvelope);
  expect(seen).toEqual(["dm.read", "dm_message.sent"]);
  off();
});

it("drops a repeated event by its id even without a sequence", () => {
  const seen: string[] = [];
  const off = ws.on((env) => seen.push(env.type));
  const env = { type: "user.online", data: {}, eventId: "e-1" } as WsEnvelope;
  ws.receive(env);
  ws.receive(env);
  ws.receive({ ...env, type: "user.offline" } as WsEnvelope);
  ws.receive({ ...env, eventId: "e-2" } as WsEnvelope);
  expect(seen).toEqual(["user.online", "user.offline", "user.online"]);
  off();
});

it("still drops a repeated event after a reconnect", () => {
  const seen: string[] = [];
  const off = ws.on((env) => seen.push(env.type));
  const env = { type: "dm.read", data: {}, eventId: "e-3", partition: "messaging:r", seq: 4 } as WsEnvelope;
  ws.receive(env);
  ws.disconnect();
  ws.receive(env);
  expect(seen).toEqual(["dm.read"]);
  off();
});

it("delivers a replay of an event that was dropped while waiting in a gap", () => {
  const seen: string[] = [];
  const off = ws.on((env) => seen.push(`${env.type}#${env.seq}`));
  ws.receive({ type: "dm.read", data: {}, eventId: "g-1", partition: "messaging:g", seq: 1 } as WsEnvelope);
  ws.receive({ type: "dm.read", data: {}, eventId: "g-3", partition: "messaging:g", seq: 3 } as WsEnvelope);
  ws.disconnect();
  ws.receive({ type: "dm.read", data: {}, eventId: "g-3", partition: "messaging:g", seq: 3 } as WsEnvelope);
  expect(seen).toEqual(["dm.read#1", "dm.read#3"]);
  off();
});
