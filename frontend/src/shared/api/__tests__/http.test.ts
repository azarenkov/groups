import type { AxiosRequestConfig, AxiosResponse } from "axios";

import { http } from "../http";
import { useAuthStore } from "../../../features/auth/store/authStore";

jest.mock("../../../features/auth/api/auth", () => ({ authAPI: { handleRefreshFlow: jest.fn() } }));

const captureAdapter = (seen: { config?: AxiosRequestConfig }) =>
  (async (config: AxiosRequestConfig) => {
    seen.config = config;
    return {
      data: { data: {} },
      status: 200,
      statusText: "OK",
      headers: {},
      config,
    } as AxiosResponse;
  }) as never;

describe("shared http client", () => {
  it("sends FormData as multipart instead of serializing it to json", async () => {
    const seen: { config?: AxiosRequestConfig } = {};
    const form = new FormData();
    form.append("file", new Blob(["hello"]), "hello.txt");

    await http.post("/v1/attachments", form, { adapter: captureAdapter(seen) });

    expect(seen.config?.data).toBeInstanceOf(FormData);
    expect(String(seen.config?.headers?.["Content-Type"])).toContain(
      "multipart/form-data",
    );
  });

  it("still serializes plain object bodies as json", async () => {
    const seen: { config?: AxiosRequestConfig } = {};

    await http.post(
      "/v1/messages",
      { content: "hi" },
      { adapter: captureAdapter(seen) },
    );

    expect(seen.config?.data).toBe(JSON.stringify({ content: "hi" }));
    expect(String(seen.config?.headers?.["Content-Type"])).toContain(
      "application/json",
    );
  });

  it("never retries a request under an account that signed in after it was sent", async () => {
    const { authAPI } = jest.requireMock("../../../features/auth/api/auth");
    useAuthStore.setState({ user: { id: "a" } as never, tokens: { accessToken: "token-a", refreshToken: "r" } as never });
    authAPI.handleRefreshFlow.mockImplementation(async () => {
      useAuthStore.setState({ user: { id: "b" } as never, tokens: { accessToken: "token-b", refreshToken: "r" } as never });
    });
    const sent: string[] = [];
    const adapter = (async (config: AxiosRequestConfig) => {
      sent.push(String(config.headers?.Authorization));
      const error = Object.assign(new Error("unauthorized"), {
        isAxiosError: true,
        config,
        response: { status: 401, data: {}, headers: {}, statusText: "", config },
      });
      throw error;
    }) as never;

    await expect(http.delete("/v1/search/history", { adapter })).rejects.toBeTruthy();

    expect(sent).toEqual(["Bearer token-a"]);
  });

  it("retries with the refreshed token when the account stayed the same", async () => {
    const { authAPI } = jest.requireMock("../../../features/auth/api/auth");
    useAuthStore.setState({ user: { id: "a" } as never, tokens: { accessToken: "old", refreshToken: "r" } as never });
    authAPI.handleRefreshFlow.mockImplementation(async () => {
      useAuthStore.setState({ tokens: { accessToken: "fresh", refreshToken: "r" } as never });
    });
    const sent: string[] = [];
    const adapter = (async (config: AxiosRequestConfig) => {
      sent.push(String(config.headers?.Authorization));
      if (sent.length === 1) {
        throw Object.assign(new Error("unauthorized"), {
          isAxiosError: true,
          config,
          response: { status: 401, data: {}, headers: {}, statusText: "", config },
        });
      }
      return { data: { data: {} }, status: 200, statusText: "OK", headers: {}, config } as AxiosResponse;
    }) as never;

    await http.delete("/v1/search/history", { adapter });

    expect(sent).toEqual(["Bearer old", "Bearer fresh"]);
  });

  it("does not refresh tokens for a response that belongs to a signed-out account", async () => {
    const { authAPI } = jest.requireMock("../../../features/auth/api/auth");
    authAPI.handleRefreshFlow.mockReset();
    useAuthStore.setState({ user: { id: "a" } as never, tokens: { accessToken: "token-a", refreshToken: "r" } as never });
    const adapter = (async (config: AxiosRequestConfig) => {
      useAuthStore.setState({ user: { id: "b" } as never, tokens: { accessToken: "token-b", refreshToken: "rb" } as never });
      throw Object.assign(new Error("unauthorized"), {
        isAxiosError: true,
        config,
        response: { status: 401, data: {}, headers: {}, statusText: "", config },
      });
    }) as never;

    await expect(http.get("/v1/search/history", { adapter })).rejects.toBeTruthy();

    expect(authAPI.handleRefreshFlow).not.toHaveBeenCalled();
    expect(useAuthStore.getState().tokens).toEqual(expect.objectContaining({ accessToken: "token-b" }));
  });
});
