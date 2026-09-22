interface RuntimeConfig {
  apiUrl?: string;
  wsUrl?: string;
  googleClientId?: string;
}

declare global {
  interface Window {
    __RIFT_CONFIG__?: RuntimeConfig;
  }
}

const runtime = (): RuntimeConfig =>
  (typeof window !== "undefined" && window.__RIFT_CONFIG__) || {};

const nonEmpty = (value: string | undefined): string | undefined =>
  value && value.length > 0 ? value : undefined;

const absolute = (value: string): string => {
  if (typeof window === "undefined" || /^[a-z]+:\/\//i.test(value)) return value;
  return new URL(value, window.location.origin).toString().replace(/\/$/, "");
};

export const apiBaseUrl = (): string =>
  absolute(
    nonEmpty(runtime().apiUrl) ??
      nonEmpty(process.env.REACT_APP_API_URL) ??
      "http://localhost:8080/api",
  );

export const wsEndpoint = (): string => {
  const injected = runtime();
  const explicit =
    nonEmpty(injected.wsUrl) ??
    (nonEmpty(injected.apiUrl) ? undefined : nonEmpty(process.env.REACT_APP_WS_URL));
  if (explicit) return absolute(explicit).replace(/^http/, "ws");
  try {
    const u = new URL(apiBaseUrl());
    const proto = u.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${u.host}/ws`;
  } catch {
    return "ws://localhost:8080/ws";
  }
};

export const googleClientId = (): string =>
  nonEmpty(runtime().googleClientId) ?? process.env.REACT_APP_GOOGLE_CLIENT_ID ?? "";
