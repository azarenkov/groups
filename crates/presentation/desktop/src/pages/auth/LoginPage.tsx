import React, { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useForm } from "react-hook-form";
import { motion } from "framer-motion";
import { useAuthStore } from "../../store/authStore";
import { authAPI } from "../../api/auth";
import "../../styles/AuthPage.css";

interface LoginFormData {
  email: string;
  password: string;
}

const LoginPage: React.FC = () => {
  const navigate = useNavigate();
  const { login, setLoading, setError, error, isLoading } = useAuthStore();
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginFormData>();
  const [showPassword, setShowPassword] = useState(false);

  const onSubmit = async (data: LoginFormData) => {
    try {
      setLoading(true);
      setError(null);

      const response = await authAPI.login(data.email, data.password);

      // Set auth token for future requests
      authAPI.setAuthToken(response.accessToken);

      // Update store with user data and tokens
      login(
        {
          accessToken: response.accessToken,
          refreshToken: response.refreshToken,
        },
        {
          id: response.userId || "",
          name: data.email.split("@")[0], // Temporary until we get name from API
          email: data.email,
        },
      );

      navigate("/app");
    } catch (err: any) {
      setError(
        err.message || "Failed to login. Please check your credentials.",
      );
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="auth-page">
      <div className="auth-background">
        <div className="auth-background-gradient"></div>
      </div>

      <motion.div
        className="auth-container"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
      >
        <div className="auth-card">
          <div className="auth-header">
            <h1>Welcome back!</h1>
            <p>We're so excited to see you again!</p>
          </div>

          <form onSubmit={handleSubmit(onSubmit)} className="auth-form">
            <div className="form-group">
              <label htmlFor="email">
                EMAIL <span className="required">*</span>
              </label>
              <input
                id="email"
                type="email"
                {...register("email", {
                  required: "Email is required",
                  pattern: {
                    value: /^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i,
                    message: "Invalid email address",
                  },
                })}
                className={errors.email ? "error" : ""}
                disabled={isLoading}
              />
              {errors.email && (
                <span className="error-message">{errors.email.message}</span>
              )}
            </div>

            <div className="form-group">
              <label htmlFor="password">
                PASSWORD <span className="required">*</span>
              </label>
              <div className="password-input-wrapper">
                <input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  {...register("password", {
                    required: "Password is required",
                    minLength: {
                      value: 4,
                      message: "Password must be at least 4 characters",
                    },
                    maxLength: {
                      value: 30,
                      message: "Password must be less than 30 characters",
                    },
                  })}
                  className={errors.password ? "error" : ""}
                  disabled={isLoading}
                />
                <button
                  type="button"
                  className="password-toggle"
                  onClick={() => setShowPassword(!showPassword)}
                  tabIndex={-1}
                >
                  {showPassword ? "👁️" : "👁️‍🗨️"}
                </button>
              </div>
              {errors.password && (
                <span className="error-message">{errors.password.message}</span>
              )}
            </div>

            <div className="forgot-password">
              <Link to="/forgot-password">Forgot your password?</Link>
            </div>

            {error && (
              <motion.div
                className="auth-error"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
              >
                {error}
              </motion.div>
            )}

            <button
              type="submit"
              className="auth-submit-button"
              disabled={isLoading}
            >
              {isLoading ? <span className="loading-spinner"></span> : "Log In"}
            </button>

            <div className="auth-footer">
              <span>Need an account?</span> <Link to="/register">Register</Link>
            </div>
          </form>
        </div>

        <div className="auth-qr-section">
          <div className="qr-code-placeholder">
            <div className="qr-icon">📱</div>
          </div>
          <h3>Log in with QR Code</h3>
          <p>
            Scan this with the <strong>Rift mobile app</strong> to log in
            instantly.
          </p>
        </div>
      </motion.div>
    </div>
  );
};

export default LoginPage;
