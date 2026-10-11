import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";

import i18n from "../../../../i18n";
import AuthPage from "../AuthPage";
import { useAuthStore } from "../../store/authStore";

const unverifiedToken = () => {
  const encode = (value: object) => btoa(JSON.stringify(value)).replace(/=+$/, "");
  return `${encode({ alg: "none" })}.${encode({ sub: "u1", verified: false, exp: 4102444800 })}.sig`;
};

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("leaving email verification drops the unverified session and opens sign in", () => {
  useAuthStore.setState({
    user: { id: "u1", name: "alice", email: "alice@rift.app" },
    tokens: { accessToken: unverifiedToken(), refreshToken: "r" },
    isAuthenticated: true,
  } as never);
  render(
    <MemoryRouter initialEntries={["/register"]}>
      <Routes>
        <Route path="/register" element={<AuthPage defaultMode="signup" />} />
        <Route path="/login" element={<AuthPage defaultMode="signin" />} />
      </Routes>
    </MemoryRouter>,
  );
  fireEvent.click(screen.getByRole("button", { name: new RegExp(i18n.t("auth:backToSignIn")) }));
  expect(useAuthStore.getState().tokens).toBeNull();
  expect(screen.queryByRole("button", { name: new RegExp(i18n.t("auth:backToSignIn")) })).toBeNull();
});
