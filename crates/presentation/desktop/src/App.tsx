import React from "react";
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
} from "react-router-dom";
import { AnimatePresence } from "framer-motion";
import LoginPage from "./pages/auth/LoginPage";
import RegisterPage from "./pages/auth/RegisterPage";
import { useAuthStore } from "./store/authStore";
import "./styles/App.css";

const App: React.FC = () => {
  const { isAuthenticated } = useAuthStore();

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
                isAuthenticated ? (
                  <div className="main-app">
                    <h1>Main App (Coming Soon)</h1>
                  </div>
                ) : (
                  <Navigate to="/login" />
                )
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
