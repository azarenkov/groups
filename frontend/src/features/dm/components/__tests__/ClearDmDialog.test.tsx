import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

import i18n from "../../../../i18n";
import ClearDmDialog, { useClearDmDialog } from "../ClearDmDialog";
import { dmAPI } from "../../api/dm";
import { ws, type WsEnvelope } from "../../../../shared/ws/websocket";
import { admitDmEvent, forgetDmClears } from "../../lib/dmCleared";
import { useAuthStore } from "../../../auth/store/authStore";

jest.mock("../../api/dm", () => ({ dmAPI: { clearConversation: jest.fn() } }));

const clear = dmAPI.clearConversation as jest.Mock;

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

let stamp = "";
let tick = 0;

beforeEach(() => {
  tick += 1;
  stamp = `2026-10-11T10:00:0${tick}Z`;
  clear.mockReset();
  clear.mockResolvedValue({ clearedAt: stamp });
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
});

const openFor = () => act(() => useClearDmDialog.getState().open({ id: "peer", name: "Bob" }));

it.each([
  ["Only for me", false, "me"],
  ["Delete for both", true, "everyone"],
])("%s clears the conversation and applies it locally", async (label, forEveryone, scope) => {
  const seen: unknown[] = [];
  const off = ws.on((env) => seen.push(env));
  render(<ClearDmDialog />);
  openFor();
  expect(screen.getByText("Delete chat history with Bob?")).toBeTruthy();
  fireEvent.click(screen.getByText(label));
  expect(screen.queryByText("Delete chat history with Bob?")).toBeNull();
  await waitFor(() => expect(seen).toHaveLength(1));
  off();
  expect(clear).toHaveBeenCalledWith("peer", forEveryone);
  expect(seen[0]).toEqual({
    type: "dm_conversation.cleared",
    data: { actorId: "me", participants: ["me", "peer"], scope, clearedAt: stamp },
  });
});

it("does nothing when cancelled", () => {
  render(<ClearDmDialog />);
  openFor();
  fireEvent.click(screen.getByText("Cancel"));
  expect(clear).not.toHaveBeenCalled();
  expect(useClearDmDialog.getState().target).toBeNull();
});

it("applies the same clear only once even when it arrives twice", async () => {
  const seen: unknown[] = [];
  const off = ws.on((env) => seen.push(env));
  const env = {
    type: "dm_conversation.cleared" as const,
    data: { actorId: "peer", participants: ["peer", "me"], scope: "everyone", clearedAt: "2026-10-11T11:00:00Z" },
    partition: "messaging:clear-twice",
    seq: 1,
  };
  ws.receive(env);
  ws.receive({ ...env, data: { ...env.data, participants: ["me", "peer"] } });
  ws.receive({ ...env, seq: 2, data: { ...env.data, clearedAt: "2026-10-11T11:00:01Z" } });
  off();
  expect(seen).toHaveLength(2);
});

it("applies a clear once when the server echoes a clear applied locally", () => {
  const offFilter = ws.addFilter((env) => admitDmEvent(env, "me"));
  const seen: unknown[] = [];
  const off = ws.on((env) => seen.push(env));
  const data = { actorId: "me", participants: ["me", "echo-peer"], scope: "me", clearedAt: "2026-10-11T12:00:00Z" };
  ws.deliver({ type: "dm_conversation.cleared", data });
  ws.receive({ type: "dm_conversation.cleared", data, partition: "messaging:clear-echo", seq: 7 } as WsEnvelope);
  off();
  offFilter();
  forgetDmClears();
  expect(seen).toHaveLength(1);
});

it("applies the clear locally even while the socket is open", async () => {
  const open = jest.spyOn(ws, "isOpen").mockReturnValue(true);
  const seen: unknown[] = [];
  const off = ws.on((env) => seen.push(env));
  render(<ClearDmDialog />);
  openFor();
  fireEvent.click(screen.getByText("Only for me"));
  await waitFor(() => expect(seen).toHaveLength(1));
  off();
  open.mockRestore();
});
