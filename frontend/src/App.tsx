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
import { getRefreshToken } from "./utils/secureStorage";
import { authAPI } from "./api/auth";
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
      } catch (err) {
        console.error("[App] hydrate failed:", err);
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
