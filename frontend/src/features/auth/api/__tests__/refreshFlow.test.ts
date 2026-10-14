import { authAPI } from "../auth";
import { useAuthStore } from "../../store/authStore";

const tokensFor = (who: string) => ({ accessToken: `access-${who}`, refreshToken: `refresh-${who}` }) as never;

it("discards a refresh that finishes after another account signed in", async () => {
  useAuthStore.setState({ user: { id: "a" } as never, tokens: tokensFor("a"), isAuthenticated: true });
  let answer: (tokens: unknown) => void = () => {};
  jest.spyOn(authAPI, "refreshWith").mockReturnValue(new Promise((resolve) => (answer = resolve)) as never);
  const logout = jest.spyOn(useAuthStore.getState(), "logout");

  const refreshing = authAPI.handleRefreshFlow();
  useAuthStore.setState({ user: { id: "b" } as never, tokens: tokensFor("b") });
  answer(tokensFor("a-new"));

  await expect(refreshing).rejects.toThrow("session changed");
  expect(useAuthStore.getState().tokens).toEqual(tokensFor("b"));
  expect(logout).not.toHaveBeenCalled();
});

it("installs refreshed tokens when the session stayed the same", async () => {
  useAuthStore.setState({ user: { id: "a" } as never, tokens: tokensFor("a"), isAuthenticated: true });
  jest.spyOn(authAPI, "refreshWith").mockResolvedValue(tokensFor("a-new"));

  await authAPI.handleRefreshFlow();

  expect(useAuthStore.getState().tokens).toEqual(expect.objectContaining({ accessToken: "access-a-new" }));
});
