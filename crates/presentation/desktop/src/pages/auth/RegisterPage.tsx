import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useForm } from 'react-hook-form';
import { motion } from 'framer-motion';
import { useAuthStore } from '../../store/authStore';
import { authAPI } from '../../api/auth';
import '../../styles/AuthPage.css';

interface RegisterFormData {
  name: string;
  email: string;
  password: string;
  confirmPassword: string;
}

const RegisterPage: React.FC = () => {
  const navigate = useNavigate();
  const { setLoading, setError, error, isLoading } = useAuthStore();
  const {
    register,
    handleSubmit,
    watch,
    formState: { errors },
  } = useForm<RegisterFormData>();
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  const password = watch('password');

  const onSubmit = async (data: RegisterFormData) => {
    try {
      setLoading(true);
      setError(null);

      await authAPI.register(data.name, data.email, data.password);

      // After successful registration, redirect to login
      navigate('/login', {
        state: { message: 'Registration successful! Please log in.' },
      });
    } catch (err: any) {
      setError(err.message || 'Failed to register. Please try again.');
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
        className="auth-container register"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
      >
        <div className="auth-card">
          <div className="auth-header">
            <h1>Create an account</h1>
          </div>

          <form onSubmit={handleSubmit(onSubmit)} className="auth-form">
            <div className="form-group">
              <label htmlFor="name">
                USERNAME <span className="required">*</span>
              </label>
              <input
                id="name"
                type="text"
                {...register('name', {
                  required: 'Username is required',
                  minLength: {
                    value: 3,
                    message: 'Username must be at least 3 characters',
                  },
                  maxLength: {
                    value: 15,
                    message: 'Username must be less than 15 characters',
                  },
                  pattern: {
                    value: /^[a-zA-Z0-9_]+$/,
                    message: 'Username can only contain letters, numbers, and underscores',
                  },
                })}
                className={errors.name ? 'error' : ''}
                disabled={isLoading}
                placeholder="Enter your username"
              />
              {errors.name && (
                <span className="error-message">{errors.name.message}</span>
              )}
            </div>

            <div className="form-group">
              <label htmlFor="email">
                EMAIL <span className="required">*</span>
              </label>
              <input
                id="email"
                type="email"
                {...register('email', {
                  required: 'Email is required',
                  pattern: {
                    value: /^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i,
                    message: 'Invalid email address',
                  },
                })}
                className={errors.email ? 'error' : ''}
                disabled={isLoading}
                placeholder="Enter your email"
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
                  type={showPassword ? 'text' : 'password'}
                  {...register('password', {
                    required: 'Password is required',
                    minLength: {
                      value: 4,
                      message: 'Password must be at least 4 characters',
                    },
                    maxLength: {
                      value: 30,
                      message: 'Password must be less than 30 characters',
                    },
                  })}
                  className={errors.password ? 'error' : ''}
                  disabled={isLoading}
                  placeholder="Enter your password"
                />
                <button
                  type="button"
                  className="password-toggle"
                  onClick={() => setShowPassword(!showPassword)}
                  tabIndex={-1}
                >
                  {showPassword ? '👁️' : '👁️‍🗨️'}
                </button>
              </div>
              {errors.password && (
                <span className="error-message">{errors.password.message}</span>
              )}
            </div>

            <div className="form-group">
              <label htmlFor="confirmPassword">
                CONFIRM PASSWORD <span className="required">*</span>
              </label>
              <div className="password-input-wrapper">
                <input
                  id="confirmPassword"
                  type={showConfirmPassword ? 'text' : 'password'}
                  {...register('confirmPassword', {
                    required: 'Please confirm your password',
                    validate: (value) =>
                      value === password || 'Passwords do not match',
                  })}
                  className={errors.confirmPassword ? 'error' : ''}
                  disabled={isLoading}
                  placeholder="Confirm your password"
                />
                <button
                  type="button"
                  className="password-toggle"
                  onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                  tabIndex={-1}
                >
                  {showConfirmPassword ? '👁️' : '👁️‍🗨️'}
                </button>
              </div>
              {errors.confirmPassword && (
                <span className="error-message">
                  {errors.confirmPassword.message}
                </span>
              )}
            </div>

            {error && (
              <motion.div
                className="auth-error"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
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
              {isLoading ? (
                <span className="loading-spinner"></span>
              ) : (
                'Continue'
              )}
            </button>

            <div className="terms-notice">
              By registering, you agree to Rift's{' '}
              <a href="#terms" onClick={(e) => e.preventDefault()}>
                Terms of Service
              </a>{' '}
              and{' '}
              <a href="#privacy" onClick={(e) => e.preventDefault()}>
                Privacy Policy
              </a>
              .
            </div>

            <div className="auth-footer">
              <Link to="/login">Already have an account?</Link>
            </div>
          </form>
        </div>
      </motion.div>
    </div>
  );
};

export default RegisterPage;
