import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation, useNavigate } from "react-router-dom";

import AuthPage from "../AuthPage";
import { authAPI } from "../../api/auth";
import { ApiError } from "../../../../shared/lib/apiError";
import { useAuthStore } from "../../store/authStore";

jest.mock("../../api/auth", () => ({
  authAPI: {
    forgotPassword: jest.fn(),
    resetPassword: jest.fn(),
    captchaChallenge: jest.fn(),
  },
}));

const forgotPassword = authAPI.forgotPassword as jest.Mock;
const resetPassword = authAPI.resetPassword as jest.Mock;
const TOKEN = `${"a".repeat(32)}.${"b".repeat(64)}`;

let path = "";
let go: (to: string) => void = () => undefined;
const Where: React.FC = () => {
  path = useLocation().pathname + useLocation().search;
  go = useNavigate();
  return null;
};

const renderAt = (url: string, mode: "signin" | "reset-password") =>
  render(
    <MemoryRouter initialEntries={[url]} future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <Where />
      <Routes>
        <Route path="/login" element={<AuthPage defaultMode="signin" />} />
        <Route path="/reset-password" element={<AuthPage defaultMode={mode} />} />
      </Routes>
    </MemoryRouter>,
  );

beforeEach(() => {
  (authAPI.captchaChallenge as jest.Mock).mockResolvedValue(null);
  forgotPassword.mockResolvedValue(undefined);
  resetPassword.mockResolvedValue(undefined);
  useAuthStore.setState({ tokens: null, user: null, isAuthenticated: false });
});

afterEach(() => jest.useRealTimers());

it("sends the reset email and lets the user resend it after a minute", async () => {
  jest.useFakeTimers();
  renderAt("/login", "signin");
  fireEvent.click(screen.getByRole("button", { name: "Forgot password?" }));
  fireEvent.change(screen.getByPlaceholderText("you@rift.app"), { target: { value: " T@example.com " } });
  fireEvent.click(screen.getByRole("button", { name: "Send reset link" }));
  expect(await screen.findByText("Check your inbox")).toBeInTheDocument();
  expect(forgotPassword).toHaveBeenCalledWith("T@example.com", null);
  expect(screen.queryByRole("button", { name: "Resend the email" })).toBeNull();
  await act(async () => {
    jest.advanceTimersByTime(59_000);
  });
  expect(screen.queryByRole("button", { name: "Resend the email" })).toBeNull();
  await act(async () => {
    jest.advanceTimersByTime(1_000);
  });
  fireEvent.click(screen.getByRole("button", { name: "Resend the email" }));
  await waitFor(() => expect(forgotPassword).toHaveBeenCalledTimes(2));
});

it("sets a new password from the emailed link and signs out the old session", async () => {
  useAuthStore.setState({ tokens: { accessToken: "a", refreshToken: "r", tokenType: "Bearer", expiresIn: 1 }, isAuthenticated: true });
  renderAt(`/reset-password?token=${TOKEN}`, "reset-password");
  expect(path).toBe(`/reset-password?token=${TOKEN}`);
  const [first] = document.querySelectorAll<HTMLInputElement>("input[autocomplete=new-password]");
  fireEvent.change(first, { target: { value: "secret1" } });
  fireEvent.change(screen.getByPlaceholderText("Repeat your password"), { target: { value: "secret1" } });
  fireEvent.click(screen.getByRole("button", { name: "Update password" }));
  expect(await screen.findByText("Password updated")).toBeInTheDocument();
  expect(resetPassword).toHaveBeenCalledWith(TOKEN, "secret1");
  expect(useAuthStore.getState().tokens).toBeNull();
  expect(path).toBe("/reset-password");
  fireEvent.click(screen.getByRole("button", { name: "Continue to sign in" }));
  expect(path).toBe("/login");
});

it("offers a new link when the emailed one has expired", async () => {
  resetPassword.mockRejectedValueOnce(new ApiError("expired", 400, "RESET_TOKEN_EXPIRED"));
  renderAt(`/reset-password?token=${TOKEN}`, "reset-password");
  const [first] = document.querySelectorAll<HTMLInputElement>("input[autocomplete=new-password]");
  fireEvent.change(first, { target: { value: "secret1" } });
  fireEvent.change(screen.getByPlaceholderText("Repeat your password"), { target: { value: "secret1" } });
  fireEvent.click(screen.getByRole("button", { name: "Update password" }));
  expect(await screen.findByText("This link has expired. Request a new one.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Request a new link" }));
  expect(screen.getByText("Forgot your password?")).toBeInTheDocument();
});

it("explains a link without a token instead of showing the form", () => {
  renderAt("/reset-password", "reset-password");
  expect(screen.getByText("This link doesn't work")).toBeInTheDocument();
});

it("keeps the emailed link usable after a reload", () => {
  const first = renderAt(`/reset-password?token=${TOKEN}`, "reset-password");
  expect(path).toBe(`/reset-password?token=${TOKEN}`);
  first.unmount();
  renderAt(path, "reset-password");
  expect(screen.getByRole("button", { name: "Update password" })).toBeInTheDocument();
});

it("shows the form again when a fresh link arrives after a dead one", async () => {
  resetPassword.mockRejectedValueOnce(new ApiError("used", 400, "RESET_TOKEN_INVALID"));
  renderAt(`/reset-password?token=${TOKEN}`, "reset-password");
  const [first] = document.querySelectorAll<HTMLInputElement>("input[autocomplete=new-password]");
  fireEvent.change(first, { target: { value: "secret1" } });
  fireEvent.change(screen.getByPlaceholderText("Repeat your password"), { target: { value: "secret1" } });
  fireEvent.click(screen.getByRole("button", { name: "Update password" }));
  expect(await screen.findByText("This link doesn't work")).toBeInTheDocument();
  const fresh = `${"c".repeat(32)}.${"d".repeat(64)}`;
  act(() => go(`/reset-password?token=${fresh}`));
  expect(await screen.findByRole("button", { name: "Update password" })).toBeInTheDocument();
});
