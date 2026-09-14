import React, { useEffect } from "react";
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
  useLocation,
  useNavigate,
} from "react-router-dom";
import { AnimatePresence } from "framer-motion";
import LoginPage from "./pages/auth/LoginPage";
import RegisterPage from "./pages/auth/RegisterPage";
import ServersPage from "./pages/ServersPage";
import JoinPage from "./pages/JoinPage";
import ScreenSharePicker from "./components/rift/ScreenSharePicker";
import { useAuthStore } from "./store/authStore";
import { applyAppearance, useAppearanceStore } from "./store/appearanceStore";
import { initPushToTalk } from "./services/pushToTalk";
import { initDesktopNotifications } from "./utils/desktopNotification";
import { decodeAccessToken, isAccessTokenVerified } from "./utils/jwt";
import { clearRefreshToken, getRefreshToken } from "./utils/secureStorage";
import { authAPI } from "./api/auth";
import { ApiError } from "./utils/apiError";
import { ws } from "./services/websocket";
import { installWsBootstrap } from "./services/websocketBootstrap";
import { initGameActivity } from "./services/gameActivity";
import { initHotkeys } from "./services/hotkeys";
import { authRedirectTarget, extractInviteToken } from "./utils/inviteLinks";
import "./styles/App.css";

const AuthRedirect: React.FC = () => {
  const { search } = useLocation();
  return <Navigate to={authRedirectTarget(search)} replace />;
};

const DeepLinkListener: React.FC = () => {
  const navigate = useNavigate();
  useEffect(() => {
    const bridge = window.electron;
    if (!bridge?.onDeepLink) return;
    const open = (url: string | null) => {
      const token = url ? extractInviteToken(url) : null;
      if (!token) return;
      const { isAuthenticated: signedIn, tokens: current } = useAuthStore.getState();
      const verified =
        !!current?.accessToken && isAccessTokenVerified(current.accessToken);
      navigate(
        signedIn && verified
          ? `/app?join=${encodeURIComponent(token)}`
          : `/join/${token}`,
      );
    };
    void bridge.getPendingDeepLink?.().then(open).catch(() => {});
    return bridge.onDeepLink(open);
  }, [navigate]);
  return null;
};

const App: React.FC = () => {
  const { isAuthenticated, tokens, isHydrating } = useAuthStore();
  const mode = useAppearanceStore((s) => s.mode);
  const accent = useAppearanceStore((s) => s.accent);
  const chatFontSize = useAppearanceStore((s) => s.chatFontSize);
  const reduceMotion = useAppearanceStore((s) => s.reduceMotion);

  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);

  useEffect(() => {
    applyAppearance({ mode, accent, chatFontSize, reduceMotion });
  }, [mode, accent, chatFontSize, reduceMotion]);

  useEffect(() => {
    if (mode !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () =>
      applyAppearance({ mode, accent, chatFontSize, reduceMotion });
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [mode, accent, chatFontSize, reduceMotion]);

  useEffect(() => {
    initPushToTalk();
    initDesktopNotifications();
  }, []);

  useEffect(() => {
    installWsBootstrap();
    initGameActivity();
    initHotkeys();
  }, []);

  useEffect(() => {
    if (!window.electron) return;
    document.body.classList.add("electron");
    const apply = (v: boolean) =>
      document.body.classList.toggle("electron-fullscreen", v);
    void window.electron.isFullscreen?.().then(apply).catch(() => {});
    const off = window.electron.onFullscreenChange?.(apply);
    return () => off?.();
  }, []);

  const hasAccessToken = !!tokens?.accessToken;
  useEffect(() => {
    const ready = isAuthenticated && !hasUnverifiedSession && hasAccessToken;
    if (ready) {
      ws.connect();
      let cancelled = false;
      void (async () => {
        try {
          const { useNotificationPrefsStore } = await import(
            "./store/notificationPrefsStore"
          );
          if (cancelled) return;
          let prefsLoaded = false;
          let delayMs = 1000;
          while (!cancelled && !prefsLoaded) {
            try {
              await useNotificationPrefsStore.getState().load();
              prefsLoaded = true;
            } catch (err) {
              console.warn("[App] failed to load notification prefs:", err);
              const wait = delayMs;
              await new Promise((r) => setTimeout(r, wait));
              delayMs = Math.min(delayMs * 2, 15000);
            }
          }
          if (cancelled) return;
          const { refetchUnread } = await import("./store/unreadStore");
          if (cancelled) return;
          await refetchUnread();
        } catch (err) {
          if (!cancelled) console.warn("[App] failed to load unread counts:", err);
        }
      })();
      void (async () => {
        try {
          const { dmAPI } = await import("./api/dm");
          const { useDmPreviewStore } = await import("./store/dmPreviewStore");
          const res = await dmAPI.listPreviews();
          if (cancelled) return;
          useDmPreviewStore.getState().setAll(res.previews);
          useDmPreviewStore.getState().setNextCursor(res.nextCursor);
        } catch (err) {
          if (!cancelled) console.warn("[App] failed to load dm previews:", err);
        }
      })();
      return () => {
        cancelled = true;
        ws.disconnect();
      };
    }
    ws.disconnect();
    void import("./store/unreadStore").then(({ useUnreadStore }) =>
      useUnreadStore.getState().clearAll(),
    );
    void import("./store/presenceStore").then(({ usePresenceStore }) =>
      usePresenceStore.getState().clearAll(),
    );
    void import("./store/notificationPrefsStore").then(
      ({ useNotificationPrefsStore }) =>
        useNotificationPrefsStore.getState().clearAll(),
    );
    void import("./store/rolesStore").then(({ useRolesStore }) =>
      useRolesStore.getState().clearAll(),
    );
    void import("./store/voicePresenceStore").then(({ useVoicePresenceStore }) =>
      useVoicePresenceStore.getState().clearAll(),
    );
    void import("./store/homeStore").then(({ useHomeStore }) =>
      useHomeStore.getState().clear(),
    );
    void import("./store/userProfileStore").then(({ useUserProfileStore }) =>
      useUserProfileStore.getState().clear(),
    );
    void import("./store/blockStore").then(({ useBlockStore }) =>
      useBlockStore.getState().clearAll(),
    );
    void import("./store/friendshipStore").then(({ useFriendshipStore }) =>
      useFriendshipStore.getState().clearAll(),
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
        <DeepLinkListener />
        <AnimatePresence mode="wait">
          <Routes>
            <Route
              path="/login"
              element={
                !isAuthenticated || hasUnverifiedSession ? (
                  <LoginPage />
                ) : (
                  <AuthRedirect />
                )
              }
            />
            <Route
              path="/register"
              element={
                !isAuthenticated || hasUnverifiedSession ? (
                  <RegisterPage />
                ) : (
                  <AuthRedirect />
                )
              }
            />
            <Route path="/join/:token" element={<JoinPage />} />
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
        <ScreenSharePicker />
      </div>
    </Router>
  );
};

export default App;
