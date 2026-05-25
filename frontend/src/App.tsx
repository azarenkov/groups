import React, { useEffect } from "react";
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
} from "react-router-dom";
import { AnimatePresence } from "framer-motion";
import LoginPage from "./pages/auth/LoginPage";
import RegisterPage from "./pages/auth/RegisterPage";
import ServersPage from "./pages/ServersPage";
import { useAuthStore } from "./store/authStore";
import { useThemeStore, applyTheme } from "./store/themeStore";
import { decodeAccessToken, isAccessTokenVerified } from "./utils/jwt";
import { clearRefreshToken, getRefreshToken } from "./utils/secureStorage";
import { authAPI } from "./api/auth";
import { ApiError } from "./utils/apiError";
import { ws } from "./services/websocket";
import { installWsBootstrap } from "./services/websocketBootstrap";
import "./styles/App.css";

const App: React.FC = () => {
  const { isAuthenticated, tokens, isHydrating } = useAuthStore();
  const { theme } = useThemeStore();

  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    installWsBootstrap();
  }, []);

  const hasAccessToken = !!tokens?.accessToken;
  useEffect(() => {
    const ready = isAuthenticated && !hasUnverifiedSession && hasAccessToken;
    if (ready) {
      ws.connect();
      void (async () => {
        try {
          const { userAPI } = await import("./api/user");
          const { useUnreadStore } = await import("./store/unreadStore");
          const u = await userAPI.getUnread();
          const channels: Record<string, number> = {};
          for (const c of u.channels) channels[c.channelId] = c.count;
          const dms: Record<string, number> = {};
          for (const d of u.dms) dms[d.friendId] = d.count;
          useUnreadStore.getState().setAll(channels, dms);
        } catch (err) {
          console.warn("[App] failed to load unread counts:", err);
        }
      })();
      return () => {
        ws.disconnect();
      };
    }
    ws.disconnect();
    void import("./store/unreadStore").then(({ useUnreadStore }) =>
      useUnreadStore.getState().clearAll(),
    );
  }, [isAuthenticated, hasUnverifiedSession, hasAccessToken]);

  useEffect(() => {
    let cancelled = false;
    const hydrate = async () => {
      try {
        const refreshToken = await getRefreshToken();
        if (!refreshToken) return;
        const fresh = await authAPI.refreshWith(refreshToken);
        if (cancelled) return;
        const claims = decodeAccessToken(fresh.accessToken);
        const userId = claims?.sub ?? "";
        useAuthStore.getState().login(fresh, {
          id: userId,
          name: "",
          email: "",
        });
        authAPI.setAuthToken(fresh.accessToken);
        void useAuthStore
          .getState()
          .fetchMe()
          .catch((err) => {
            console.warn("[App] fetchMe failed:", err);
          });
      } catch (err) {
        console.error("[App] hydrate failed:", err);
        const apiErr = err instanceof ApiError ? err : null;
        const code = apiErr?.code;
        const status = apiErr?.status;
        const sessionDead =
          code === "REFRESH_TOKEN_REVOKED" ||
          code === "REFRESH_TOKEN_EXPIRED" ||
          code === "REFRESH_TOKEN_NOT_FOUND" ||
          code === "REFRESH_TOKEN_USER_MISMATCH" ||
          code === "SESSION_EXPIRED" ||
          code === "INVALID_CREDENTIALS" ||
          code === "TOKEN_INVALID_TYPE" ||
          code === "TOKEN_DECODING_FAILED" ||
          code === "USER_NOT_FOUND" ||
          code === "VALIDATION_FAILED" ||
          status === 400 ||
          status === 401 ||
          status === 403 ||
          status === 422;
        if (sessionDead) {
          await clearRefreshToken();
        }
      } finally {
        if (!cancelled) useAuthStore.getState().setHydrating(false);
      }
    };
    hydrate();
    return () => {
      cancelled = true;
    };
  }, []);

  if (isHydrating) {
    return (
      <div className="app">
        <div className="loading-screen">
          <div className="spinner" />
        </div>
      </div>
    );
  }

  return (
    <Router
      future={{
        v7_startTransition: true,
        v7_relativeSplatPath: true,
      }}
    >
      <div className="app">
        <AnimatePresence mode="wait">
          <Routes>
            <Route
              path="/login"
              element={
                !isAuthenticated || hasUnverifiedSession ? (
                  <LoginPage />
                ) : (
                  <Navigate to="/app" />
                )
              }
            />
            <Route
              path="/register"
              element={
                !isAuthenticated || hasUnverifiedSession ? (
                  <RegisterPage />
                ) : (
                  <Navigate to="/app" />
                )
              }
            />
            <Route
              path="/app"
              element={
                isAuthenticated && !hasUnverifiedSession ? (
                  <ServersPage />
                ) : (
                  <Navigate to={hasUnverifiedSession ? "/register" : "/login"} />
                )
              }
            />
            <Route
              path="*"
              element={
                <Navigate
                  to={
                    isAuthenticated && !hasUnverifiedSession
                      ? "/app"
                      : hasUnverifiedSession
                        ? "/register"
                        : "/login"
                  }
                />
              }
            />
          </Routes>
        </AnimatePresence>
      </div>
    </Router>
  );
};

export default App;
