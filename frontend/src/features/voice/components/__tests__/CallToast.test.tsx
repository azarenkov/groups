import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import { CallToast } from "../CallReturnBar";
import { useCallStore } from "../../store/callStore";

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

beforeEach(() => {
  useCallStore.setState({ callError: null, callNotice: null });
});

it("opens the chat with the caller from a missed call notice", () => {
  const onOpenDm = jest.fn();
  useCallStore.setState({ callNotice: { kind: "missed", userId: "peer", name: "Peer" } });
  render(<CallToast onOpenDm={onOpenDm} />);
  fireEvent.click(screen.getByRole("status"));
  expect(onOpenDm).toHaveBeenCalledWith("peer");
  expect(useCallStore.getState().callNotice).toBeNull();
});

it("shows who declined the call", () => {
  useCallStore.setState({ callNotice: { kind: "declined", userId: "peer", name: "Peer" } });
  render(<CallToast onOpenDm={jest.fn()} />);
  expect(screen.getByRole("status")).toHaveTextContent("Peer declined the call");
});
