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
import "./styles/App.css";

const App: React.FC = () => {
  const { isAuthenticated } = useAuthStore();
  const { theme } = useThemeStore();

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  return (
    <Router>
      <div className="app">
        <AnimatePresence mode="wait">
          <Routes>
            <Route
              path="/login"
              element={
                !isAuthenticated ? <LoginPage /> : <Navigate to="/app" />
              }
            />
            <Route
              path="/register"
              element={
                !isAuthenticated ? <RegisterPage /> : <Navigate to="/app" />
              }
            />
            <Route
              path="/app"
              element={
                isAuthenticated ? <ServersPage /> : <Navigate to="/login" />
              }
            />
            <Route
              path="*"
              element={<Navigate to={isAuthenticated ? "/app" : "/login"} />}
            />
          </Routes>
        </AnimatePresence>
      </div>
    </Router>
  );
};

export default App;
