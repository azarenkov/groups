import { apiBaseUrl, publicAppOrigin, wsEndpoint } from "../runtime";

describe("runtime config", () => {
  afterEach(() => {
    delete window.__RIFT_CONFIG__;
  });

  it("prefers the injected runtime values over build-time ones", () => {
    window.__RIFT_CONFIG__ = { apiUrl: "https://api.rift.test/api" };
    expect(apiBaseUrl()).toBe("https://api.rift.test/api");
    expect(wsEndpoint()).toBe("wss://api.rift.test/ws");
  });

  it("resolves relative urls against the page origin", () => {
    window.__RIFT_CONFIG__ = { apiUrl: "/api", wsUrl: "/ws" };
    expect(apiBaseUrl()).toBe(`${window.location.origin}/api`);
    expect(wsEndpoint()).toBe(`${window.location.origin.replace(/^http/, "ws")}/ws`);
  });

  it("falls back to the build-time default", () => {
    expect(apiBaseUrl()).toBe(process.env.REACT_APP_API_URL || "http://localhost:8080/api");
  });

  it("builds share links on the configured app origin", () => {
    window.__RIFT_CONFIG__ = { apiUrl: "https://api.rift.test/api", appUrl: "https://rift.test/" };
    expect(publicAppOrigin()).toBe("https://rift.test");
  });

  it("uses the page origin for share links in the browser, not the api host", () => {
    window.__RIFT_CONFIG__ = { apiUrl: "https://api.rift.test/api" };
    expect(publicAppOrigin()).toBe(window.location.origin);
  });
});
