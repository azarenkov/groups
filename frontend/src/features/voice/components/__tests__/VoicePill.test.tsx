import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import VoicePill from "../VoicePill";

const renderPill = (overrides: Partial<React.ComponentProps<typeof VoicePill>> = {}) => {
  const props: React.ComponentProps<typeof VoicePill> = {
    title: "voice",
    meta: "1 connected",
    isMuted: false,
    onToggleMute: jest.fn(),
    isDeafened: false,
    onToggleDeafen: jest.fn(),
    onLeave: jest.fn(),
    cameraOn: false,
    onToggleCamera: jest.fn(),
    screenOn: true,
    onToggleScreen: jest.fn(),
    ...overrides,
  };
  render(<VoicePill {...props} />);
  return props;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("controls the camera and screen share from the call bar", () => {
  const p = renderPill();
  const camera = screen.getByTitle(i18n.t("voice:startCamera"));
  expect(camera).toHaveAttribute("aria-pressed", "false");
  fireEvent.click(camera);
  expect(p.onToggleCamera).toHaveBeenCalled();
  const share = screen.getByTitle(i18n.t("voice:stopScreenShare"));
  expect(share).toHaveAttribute("aria-pressed", "true");
  fireEvent.click(share);
  expect(p.onToggleScreen).toHaveBeenCalled();
  fireEvent.click(screen.getByTitle(i18n.t("voice:leave")));
  expect(p.onLeave).toHaveBeenCalled();
});

it("hides camera and screen buttons where they are not offered", () => {
  renderPill({ onToggleCamera: undefined, onToggleScreen: undefined });
  expect(screen.queryByTitle(i18n.t("voice:startCamera"))).toBeNull();
  expect(screen.queryByTitle(i18n.t("voice:startScreenShare"))).toBeNull();
});
