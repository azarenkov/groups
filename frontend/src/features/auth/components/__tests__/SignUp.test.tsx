import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import { SignInUp } from "../steps/SignInUp";
import { authAPI } from "../../api/auth";

jest.mock("../../api/auth", () => ({
  authAPI: { register: jest.fn(), captchaChallenge: jest.fn() },
}));

jest.mock("../../lib/useCaptcha", () => ({
  isCaptchaError: () => false,
  useCaptcha: () => ({ take: () => Promise.resolve("captcha"), warmUp: () => undefined }),
}));

const register = authAPI.register as jest.Mock;

const renderSignUp = () =>
  render(
    <SignInUp
      mode="signup"
      email=""
      onEmailChange={jest.fn()}
      onSwitchMode={jest.fn()}
      onSignedIn={jest.fn()}
      go2fa={jest.fn()}
      goVerify={jest.fn()}
      goForgot={jest.fn()}
      onAvatarPicked={jest.fn()}
    />,
  );

const fill = (password: string, confirm: string) => {
  fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:displayNamePlaceholder")), { target: { value: "alice" } });
  fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:emailPlaceholder")), { target: { value: "alice@rift.app" } });
  fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:passwordPlaceholderSignup")), { target: { value: password } });
  fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:repeatPassword")), { target: { value: confirm } });
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("asks to repeat the password and stops when it differs", async () => {
  register.mockRejectedValue(new Error("stop"));
  renderSignUp();
  fill("secret-1", "secret-2");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("auth:createAccount") }));
  });
  expect(screen.getByText(i18n.t("auth:passwordsDontMatch"))).toBeInTheDocument();
  expect(register).not.toHaveBeenCalled();
});

it("signs up once both passwords match", async () => {
  register.mockRejectedValue(new Error("stop"));
  renderSignUp();
  fill("secret-1", "secret-1");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("auth:createAccount") }));
  });
  expect(register).toHaveBeenCalledWith("alice", "alice@rift.app", "secret-1", expect.anything(), "captcha");
});
