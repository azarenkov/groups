import type { AxiosError, AxiosInstance } from "axios";

import { rateLimitRetry } from "../rateLimitRetry";

const error = (
  status: number,
  config: Record<string, unknown> = { method: "get", url: "/v1/x" },
  headers: Record<string, string> = {},
): AxiosError =>
  ({ config, response: { status, headers } }) as unknown as AxiosError;

describe("rateLimitRetry", () => {
  const client = { request: jest.fn().mockResolvedValue({ status: 200 }) } as unknown as AxiosInstance;

  beforeEach(() => {
    (client.request as jest.Mock).mockClear();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("ignores everything that is not a rate limit", () => {
    expect(rateLimitRetry(client, error(500))).toBeNull();
    expect(rateLimitRetry(client, error(429, { method: "post", url: "/v1/x" }))).toBeNull();
  });

  it("retries a rate-limited read and honours Retry-After", async () => {
    const config: Record<string, unknown> = { method: "get", url: "/v1/x" };
    const retried = rateLimitRetry(client, error(429, config, { "retry-after": "2" }));
    expect(retried).not.toBeNull();
    expect(config._rateLimitRetries).toBe(1);
    jest.advanceTimersByTime(2000);
    await retried;
    expect(client.request).toHaveBeenCalledWith(config);
  });

  it("gives up after two retries", () => {
    const config: Record<string, unknown> = { method: "get", url: "/v1/x", _rateLimitRetries: 2 };
    expect(rateLimitRetry(client, error(429, config))).toBeNull();
  });
});
