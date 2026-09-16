import {
  AxiosError,
  AxiosInstance,
  AxiosRequestConfig,
  AxiosResponse,
} from "axios";

const MAX_RETRIES = 2;
const BASE_DELAY_MS = 600;
const JITTER_MS = 400;

type RetriedConfig = AxiosRequestConfig & { _rateLimitRetries?: number };

export function rateLimitRetry(
  client: AxiosInstance,
  error: AxiosError,
): Promise<AxiosResponse> | null {
  const config = error.config as RetriedConfig | undefined;
  const response = error.response;
  if (!config || !response || response.status !== 429) return null;
  const method = (config.method ?? "get").toLowerCase();
  if (method !== "get" && method !== "head") return null;
  const retries = config._rateLimitRetries ?? 0;
  if (retries >= MAX_RETRIES) return null;
  config._rateLimitRetries = retries + 1;
  const retryAfter = Number(response.headers?.["retry-after"]);
  const delay =
    Number.isFinite(retryAfter) && retryAfter > 0
      ? retryAfter * 1000
      : BASE_DELAY_MS * (retries + 1) + Math.random() * JITTER_MS;
  return new Promise<void>((resolve) => setTimeout(resolve, delay)).then(() =>
    client.request(config),
  );
}
