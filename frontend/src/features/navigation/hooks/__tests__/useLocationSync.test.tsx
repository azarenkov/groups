import React from "react";
import { act, render } from "@testing-library/react";
import { MemoryRouter, useLocation, useNavigate, type NavigateFunction } from "react-router-dom";

import { useLocationSync } from "../useLocationSync";
import { useBrowserHistory } from "../useBrowserHistory";
import { rememberLocation } from "../../lib/lastLocation";
import type { AppLocation } from "../../lib/appLocation";

const me = "11111111-1111-1111-1111-111111111111";
const peer = "22222222-2222-2222-2222-222222222222";
const other = "33333333-3333-3333-3333-333333333333";
const message = "55555555-5555-5555-5555-555555555555";

let seenPath = "";
let navigateTo: NavigateFunction = () => {};
let history: ReturnType<typeof useBrowserHistory> | null = null;

const Harness: React.FC<{ current: AppLocation | null; resolving?: boolean; open: (location: AppLocation) => void }> = ({
  current,
  resolving = false,
  open,
}) => {
  useLocationSync({ viewerId: me, current, resolving, open });
  history = useBrowserHistory();
  const location = useLocation();
  seenPath = `${location.pathname}${location.search}`;
  navigateTo = useNavigate();
  return null;
};

const renderAt = (path: string, props: React.ComponentProps<typeof Harness>) =>
  render(
    <MemoryRouter initialEntries={[path]} future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <Harness {...props} />
    </MemoryRouter>,
  );

beforeEach(() => localStorage.clear());

it("opens the last chat for this user from the bare app address and keeps query links", () => {
  rememberLocation(me, `/app/dm/${peer}`);
  const open = jest.fn();
  renderAt("/app?profile=x", { current: { kind: "home" }, open });
  expect(seenPath).toBe(`/app/dm/${peer}?profile=x`);
  expect(open).toHaveBeenCalledWith({ kind: "dm", peerId: peer });
});

it("falls back to home when nothing was opened before", () => {
  renderAt("/app", { current: { kind: "home" }, open: jest.fn() });
  expect(seenPath).toBe("/app/home");
});

it("opens what the address points to and jumps to a linked message", () => {
  const open = jest.fn();
  renderAt(`/app/dm/${peer}/${message}`, { current: { kind: "dm", peerId: peer }, open });
  expect(open).toHaveBeenCalledWith({ kind: "dm", peerId: peer, messageId: message });
});

it("sends malformed addresses home", () => {
  renderAt("/app/dm/nope", { current: { kind: "home" }, open: jest.fn() });
  expect(seenPath).toBe("/app/home");
});

it("records each chat the user opens in the address and history", () => {
  const open = jest.fn();
  const view = renderAt("/app/home", { current: { kind: "home" }, open });
  view.rerender(
    <MemoryRouter initialEntries={["/app/home"]} future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <Harness current={{ kind: "dm", peerId: peer }} open={open} />
    </MemoryRouter>,
  );
  expect(seenPath).toBe(`/app/dm/${peer}`);
  expect(open).not.toHaveBeenCalled();
});

it("leaves the address alone while a linked chat is still being opened", () => {
  const open = jest.fn();
  renderAt(`/app/dm/${other}`, { current: { kind: "dm", peerId: peer }, resolving: true, open });
  expect(seenPath).toBe(`/app/dm/${other}`);
});

it("follows browser back and forward through visited chats", () => {
  const open = jest.fn();
  renderAt("/app/home", { current: { kind: "home" }, open });
  expect(history?.navState).toEqual({ back: false, forward: false });
  act(() => navigateTo(`/app/dm/${peer}`));
  expect(open).toHaveBeenLastCalledWith({ kind: "dm", peerId: peer });
  expect(history?.navState.back).toBe(true);
  act(() => history?.goHistory(-1));
  expect(seenPath).toBe("/app/home");
  expect(history?.navState).toEqual({ back: false, forward: true });
});
