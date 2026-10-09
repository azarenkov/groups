import React from "react";
import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import { SignInUp } from "../steps/SignInUp";
import { authAPI } from "../../api/auth";

jest.mock("../../api/auth", () => ({
  authAPI: { register: jest.fn(), login: jest.fn(), captchaChallenge: jest.fn() },
}));

jest.mock("../../lib/useCaptcha", () => ({
  isCaptchaError: () => false,
  useCaptcha: () => ({ take: () => Promise.resolve("captcha"), warmUp: () => undefined }),
}));

const register = authAPI.register as jest.Mock;
const login = authAPI.login as jest.Mock;

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

it("needs at least eight characters for a new password", async () => {
  register.mockRejectedValue(new Error("stop"));
  renderSignUp();
  fill("short-7", "short-7");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("auth:createAccount") }));
  });
  expect(screen.getByText(i18n.t("auth:passwordMin"))).toBeInTheDocument();
  expect(register).not.toHaveBeenCalled();
});

it("clears the mismatch once the first password is fixed", async () => {
  register.mockRejectedValue(new Error("stop"));
  renderSignUp();
  fill("secret-1", "secret-2");
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: i18n.t("auth:createAccount") }));
  });
  expect(screen.getByText(i18n.t("auth:passwordsDontMatch"))).toBeInTheDocument();
  await act(async () => {
    fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:passwordPlaceholderSignup")), { target: { value: "secret-2" } });
  });
  expect(screen.queryByText(i18n.t("auth:passwordsDontMatch"))).toBeNull();
});

it("does not complain about the confirmation before it is filled in", async () => {
  renderSignUp();
  await act(async () => {
    fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:passwordPlaceholderSignup")), { target: { value: "secret-1" } });
  });
  expect(screen.queryByText(i18n.t("auth:passwordsDontMatch"))).toBeNull();
});

it("keeps the four-character minimum for signing in with an older password", async () => {
  login.mockRejectedValue(new Error("stop"));
  render(
    <SignInUp
      mode="signin"
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
  fireEvent.change(screen.getByPlaceholderText(i18n.t("auth:emailPlaceholder")), { target: { value: "alice@rift.app" } });
  const password = screen.getByPlaceholderText(i18n.t("auth:passwordPlaceholder"));
  fireEvent.change(password, { target: { value: "abc" } });
  const submit = () => fireEvent.submit(password.closest("form") as HTMLFormElement);
  await act(async () => {
    submit();
  });
  expect(screen.getByText(i18n.t("auth:passwordMinSignIn"))).toBeInTheDocument();
  expect(login).not.toHaveBeenCalled();
  fireEvent.change(password, { target: { value: "abcd" } });
  await act(async () => {
    submit();
  });
  expect(login).toHaveBeenCalledWith("alice@rift.app", "abcd", null);
});
