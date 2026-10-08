import React, { useEffect } from "react";
import { clearPersistentSessionData } from "./shared/lib/persistentSessionData";
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
  useLocation,
  useNavigate,
  useParams,
} from "react-router-dom";
import { AnimatePresence } from "framer-motion";
import LoginPage from "./features/auth/components/LoginPage";
import RegisterPage from "./features/auth/components/RegisterPage";
import CommunitiesPage from "./features/communities/components/CommunitiesPage";
import JoinPage from "./features/communities/components/JoinPage";
import ScreenSharePicker from "./features/voice/components/ScreenSharePicker";
import { useAuthStore } from "./features/auth/store/authStore";
import { resetSessionStores } from "./shared/store/resetSession";
import { applyAppearance, useAppearanceStore } from "./features/settings/store/appearanceStore";
import { initPushToTalk } from "./features/voice/lib/pushToTalk";
import { initDesktopNotifications } from "./shared/lib/desktopNotification";
import { decodeAccessToken, isAccessTokenVerified } from "./shared/lib/jwt";
import { clearRefreshToken, getRefreshToken } from "./shared/lib/secureStorage";
import { authAPI } from "./features/auth/api/auth";
import { ApiError } from "./shared/lib/apiError";
import { ws } from "./shared/ws/websocket";
import { installWsBootstrap } from "./shared/ws/websocketBootstrap";
import { initGameActivity } from "./features/social/lib/gameActivity";
import { initHotkeys } from "./features/settings/lib/hotkeys";
import { authRedirectTarget, extractInviteToken, extractPackSlug } from "./shared/lib/inviteLinks";
import "styles/App.css";

const AuthRedirect: React.FC = () => {
  const { search } = useLocation();
  return <Navigate to={authRedirectTarget(search)} replace />;
};

const StickerPackLink: React.FC = () => {
  const { slug: raw = "" } = useParams();
  const { isAuthenticated, tokens } = useAuthStore();
  const slug = extractPackSlug(raw);
  const verified = !!tokens?.accessToken && isAccessTokenVerified(tokens.accessToken);
  if (!slug) return <Navigate to="/app" replace />;
  if (isAuthenticated && verified) return <Navigate to={`/app?stickers=${encodeURIComponent(slug)}`} replace />;
  return <Navigate to={`/login?next=${encodeURIComponent(`/stickers/${slug}`)}`} replace />;
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
    const bridge = window.electron;
    if (isHydrating || !bridge?.reportRendered) return;
    let frame = window.requestAnimationFrame(() => {
      frame = window.requestAnimationFrame(() => bridge.reportRendered?.());
    });
    return () => window.cancelAnimationFrame(frame);
  }, [isHydrating]);

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
            "./features/settings/store/notificationPrefsStore"
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
          const { refetchUnread } = await import("./features/chat/store/unreadStore");
          if (cancelled) return;
          await refetchUnread();
        } catch (err) {
          if (!cancelled) console.warn("[App] failed to load unread counts:", err);
        }
      })();
      void import("./features/dm/store/dmPreviewStore").then(({ refetchDmPreviews }) => {
        if (!cancelled) refetchDmPreviews(true);
      });
      return () => {
        cancelled = true;
        ws.disconnect();
      };
    }
    ws.disconnect();
    resetSessionStores();
  }, [isAuthenticated, hasUnverifiedSession, hasAccessToken]);

  useEffect(() => {
    let cancelled = false;
    const hydrate = async () => {
      try {
        const refreshToken = await getRefreshToken();
        if (!refreshToken) {
          clearPersistentSessionData();
          return;
        }
        const fresh = await authAPI.refreshWith(refreshToken);
        if (cancelled) return;
        const claims = decodeAccessToken(fresh.accessToken);
        const userId = claims?.sub ?? "";
        useAuthStore.getState().login(fresh, {
          id: userId,
          name: "",
          email: "",
        });
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
          clearPersistentSessionData();
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
            <Route path="/stickers/:slug" element={<StickerPackLink />} />
            <Route
              path="/app"
              element={
                isAuthenticated && !hasUnverifiedSession ? (
                  <CommunitiesPage />
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
