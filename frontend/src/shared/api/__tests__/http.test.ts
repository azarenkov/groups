import type { AxiosRequestConfig, AxiosResponse } from "axios";

import { http } from "../http";

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
});
