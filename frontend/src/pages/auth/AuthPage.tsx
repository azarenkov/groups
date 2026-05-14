import React, { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useForm } from "react-hook-form";
import { useAuthStore } from "../../store/authStore";
import { authAPI } from "../../api/auth";
import { isAccessTokenVerified, decodeAccessToken } from "../../utils/jwt";
import Icon, { Mark } from "../../components/rift/Icon";
import {
  ArrowLeft,
  OtpInput,
  passwordStrength,
  GoogleGlyph,
  AppleGlyph,
} from "../../components/auth/authHelpers";
import "../../styles/rift.css";

type Mode =
  | "signin"
  | "signup"
  | "forgot"
  | "reset-sent"
  | "reset-password"
  | "reset-success"
  | "verify-email"
  | "2fa";

interface AuthPageProps {
  defaultMode?: "signin" | "signup";
}

const AuthPage: React.FC<AuthPageProps> = ({ defaultMode = "signin" }) => {
  const navigate = useNavigate();
  const [mode, setMode] = useState<Mode>(defaultMode);
  const [email, setEmail] = useState("");

  const onSignedIn = () => navigate("/app");

  const switchTab = (m: "signin" | "signup") => {
    setMode(m);
    navigate(m === "signin" ? "/login" : "/register", { replace: true });
  };

  const showTabs = mode === "signin" || mode === "signup";

  return (
    <div className="auth-page">
      <div className="auth-topbar">
        <span className="auth-brand">
          <Mark size={22} />
          <span>Rift</span>
        </span>
      </div>

      <div className="auth-form-wrap">
        <div className="auth-card">
          {showTabs && (
            <div className="auth-tabs" role="tablist">
              <button
                role="tab"
                type="button"
                className={mode === "signin" ? "on" : ""}
                onClick={() => switchTab("signin")}
              >
                Sign in
              </button>
              <button
                role="tab"
                type="button"
                className={mode === "signup" ? "on" : ""}
                onClick={() => switchTab("signup")}
              >
                Create account
              </button>
            </div>
          )}

          {(mode === "signin" || mode === "signup") && (
            <SignInUp
              mode={mode}
              email={email}
              onEmailChange={setEmail}
              onSwitchMode={switchTab}
              onSignedIn={onSignedIn}
              go2fa={() => setMode("2fa")}
              goVerify={() => setMode("verify-email")}
              goForgot={() => setMode("forgot")}
            />
          )}
          {mode === "forgot" && (
            <Forgot
              email={email}
              onEmailChange={setEmail}
              onSubmit={() => setMode("reset-sent")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-sent" && (
            <ResetSent
              email={email}
              onResend={() => {}}
              onOpenLink={() => setMode("reset-password")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-password" && (
            <ResetPassword
              onSubmit={() => setMode("reset-success")}
              onBack={() => setMode("signin")}
            />
          )}
          {mode === "reset-success" && (
            <ResetSuccess onContinue={() => setMode("signin")} />
          )}
          {mode === "verify-email" && (
            <VerifyEmail
              email={email}
              onVerified={onSignedIn}
              onBack={() => setMode("signup")}
            />
          )}
          {mode === "2fa" && (
            <TwoFa onVerified={onSignedIn} onBack={() => setMode("signin")} />
          )}
        </div>
      </div>

      <div className="auth-legal">
        <span>© 2026 Rift</span>
        <button type="button" className="auth-link" onClick={() => {}}>Privacy</button>
        <button type="button" className="auth-link" onClick={() => {}}>Terms</button>
        <button type="button" className="auth-link" onClick={() => {}}>Status</button>
      </div>
    </div>
  );
};


interface SignInData {
  email: string;
  password: string;
}
interface SignUpData {
  name: string;
  email: string;
  password: string;
}

interface SignInUpProps {
  mode: "signin" | "signup";
  email: string;
  onEmailChange: (e: string) => void;
  onSwitchMode: (m: "signin" | "signup") => void;
  onSignedIn: () => void;
  go2fa: () => void;
  goVerify: () => void;
  goForgot: () => void;
}

const SignInUp: React.FC<SignInUpProps> = ({
  mode,
  email,
  onEmailChange,
  onSwitchMode,
  onSignedIn,
  goVerify,
  goForgot,
}) => {
  const isSignup = mode === "signup";
  const { login, setLoading, setError, error, isLoading } = useAuthStore();

  const signInForm = useForm<SignInData>({
    values: { email, password: "" },
  });
  const signUpForm = useForm<SignUpData>({
    values: { name: "", email, password: "" },
  });

  const [showPw, setShowPw] = useState(false);
  const [remember, setRemember] = useState(true);

  const handleSignIn = async (data: SignInData) => {
    try {
      setLoading(true);
      setError(null);
      const response = await authAPI.login(data.email, data.password);
      authAPI.setAuthToken(response.accessToken);
      const tokens = {
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        tokenType: response.tokenType,
        expiresIn: response.expiresIn,
      };
      const user = {
        id: decodeAccessToken(response.accessToken)?.sub ?? "",
        name: data.email.split("@")[0],
        email: data.email,
      };

      const verified = isAccessTokenVerified(response.accessToken);
      if (!verified) {
        useAuthStore.getState().setTokens(tokens);
        useAuthStore.getState().setUser(user);
        try {
          await authAPI.requestVerificationCode();
        } catch (codeErr) {
          console.error("[Auth] requestVerificationCode failed:", codeErr);
        }
        onEmailChange(data.email);
        goVerify();
      } else {
        login(tokens, user);
        onSignedIn();
      }
    } catch (err: any) {
      setError(err.message || "Could not sign you in.");
    } finally {
      setLoading(false);
    }
  };

  const handleSignUp = async (data: SignUpData) => {
    try {
      setLoading(true);
      setError(null);
      const response = await authAPI.register(
        data.name,
        data.email,
        data.password,
      );
      authAPI.setAuthToken(response.accessToken);
      useAuthStore.getState().setTokens({
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        tokenType: response.tokenType,
        expiresIn: response.expiresIn,
      });
      useAuthStore.getState().setUser({
        id: decodeAccessToken(response.accessToken)?.sub ?? "",
        name: data.name,
        email: data.email,
      });
      try {
        await authAPI.requestVerificationCode();
      } catch (codeErr) {
        console.error("[Auth] requestVerificationCode failed:", codeErr);
      }
      goVerify();
    } catch (err: any) {
      setError(err.message || "Could not create your account.");
    } finally {
      setLoading(false);
    }
  };

  const submit = isSignup
    ? signUpForm.handleSubmit(handleSignUp)
    : signInForm.handleSubmit(handleSignIn);

  const errors: Record<string, { message?: string } | undefined> = isSignup
    ? (signUpForm.formState.errors as any)
    : (signInForm.formState.errors as any);

  type AnyRegister = (name: string, opts?: any) => any;
  const registerEmail: AnyRegister = isSignup
    ? (signUpForm.register as unknown as AnyRegister)
    : (signInForm.register as unknown as AnyRegister);
  const registerPassword: AnyRegister = registerEmail;

  return (
    <>
      <h1>{isSignup ? "Create your account" : "Welcome back"}</h1>
      <p className="sub">
        {isSignup
          ? "A free account. No invites or waitlists."
          : "Sign in to rejoin your rooms and people."}
      </p>

      <form onSubmit={submit} autoComplete="off">
        {isSignup && (
          <div className="field">
            <label>Display name</label>
            <div className="field-input">
              <span className="lead"><Icon name="user" size={15} /></span>
              <input
                type="text"
                placeholder="How should we call you?"
                {...signUpForm.register("name", {
                  required: "Display name is required",
                  minLength: { value: 3, message: "At least 3 characters" },
                  maxLength: { value: 15, message: "At most 15 characters" },
                  pattern: {
                    value: /^[a-zA-Z0-9_]+$/,
                    message: "Letters, numbers, and underscores only",
                  },
                })}
              />
            </div>
            {errors.name && (
              <span style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
                {errors.name.message}
              </span>
            )}
          </div>
        )}

        <div className="field">
          <label>Email</label>
          <div className="field-input">
            <span className="lead"><Icon name="mail" size={15} /></span>
            <input
              type="email"
              placeholder="you@rift.app"
              {...registerEmail("email", {
                required: "Email is required",
                pattern: {
                  value: /^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i,
                  message: "Enter a valid email address",
                },
                onChange: (e: React.ChangeEvent<HTMLInputElement>) =>
                  onEmailChange(e.target.value),
              })}
            />
          </div>
          {errors.email && (
            <span style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
              {errors.email.message}
            </span>
          )}
        </div>

        <div className="field has-trail">
          <label>Password</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              type={showPw ? "text" : "password"}
              placeholder={isSignup ? "At least 8 characters" : "Your password"}
              {...registerPassword("password", {
                required: "Password is required",
                minLength: { value: 4, message: "At least 4 characters" },
              })}
            />
            <button
              type="button"
              className="trail"
              onClick={() => setShowPw((v) => !v)}
              title={showPw ? "Hide password" : "Show password"}
            >
              <Icon name={showPw ? "eye-off" : "eye"} size={15} />
            </button>
          </div>
          {errors.password && (
            <span style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
              {errors.password.message}
            </span>
          )}
        </div>

        <div className="auth-row">
          {isSignup ? (
            <label className="auth-check">
              <input type="checkbox" defaultChecked />
              <span className="box" />
              <span>Send me product updates</span>
            </label>
          ) : (
            <label className="auth-check">
              <input
                type="checkbox"
                checked={remember}
                onChange={(e) => setRemember(e.target.checked)}
              />
              <span className="box" />
              <span>Remember me</span>
            </label>
          )}
          {!isSignup && (
            <button type="button" className="auth-link" onClick={goForgot}>
              Forgot password?
            </button>
          )}
        </div>

        {error && (
          <div
            style={{
              fontSize: 12,
              color: "var(--danger)",
              background: "var(--danger-tint)",
              padding: "8px 12px",
              borderRadius: 8,
              marginBottom: 10,
              textAlign: "center",
            }}
          >
            {error}
          </div>
        )}

        <button type="submit" className="btn primary" disabled={isLoading}>
          {isLoading ? <span className="loading-spinner" /> : isSignup ? "Create account" : "Sign in"}
        </button>
      </form>

      <div className="auth-divider">or continue with</div>

      <div className="auth-social">
        <button type="button" className="btn social" onClick={onSignedIn} disabled>
          <GoogleGlyph />
          Continue with Google
        </button>
        <button type="button" className="btn social" onClick={onSignedIn} disabled>
          <AppleGlyph />
          Continue with Apple
        </button>
        <button type="button" className="btn social" onClick={onSignedIn} disabled>
          <Icon name="lock2" size={14} />
          Continue with SSO
        </button>
      </div>

      <div className="auth-foot">
        {isSignup ? (
          <>
            Already have an account?{" "}
            <button
              type="button"
              className="auth-link"
              onClick={() => onSwitchMode("signin")}
            >
              Sign in
            </button>
          </>
        ) : (
          <>
            New to Rift?{" "}
            <button
              type="button"
              className="auth-link"
              onClick={() => onSwitchMode("signup")}
            >
              Create an account
            </button>
          </>
        )}
      </div>
    </>
  );
};


const Forgot: React.FC<{
  email: string;
  onEmailChange: (e: string) => void;
  onSubmit: () => void;
  onBack: () => void;
}> = ({ email, onEmailChange, onSubmit, onBack }) => (
  <>
    <button type="button" className="auth-back" onClick={onBack}>
      <ArrowLeft /> Back to sign in
    </button>
    <h1>Forgot your password?</h1>
    <p className="sub">
      No worries — enter the email tied to your account and we'll send a reset
      link.
    </p>
    <form
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
      autoComplete="off"
    >
      <div className="field">
        <label>Email</label>
        <div className="field-input">
          <span className="lead"><Icon name="mail" size={15} /></span>
          <input
            type="email"
            autoFocus
            placeholder="you@rift.app"
            value={email}
            onChange={(e) => onEmailChange(e.target.value)}
          />
        </div>
      </div>
      <button type="submit" className="btn primary" style={{ marginTop: 14 }}>
        Send reset link
      </button>
    </form>
    <div className="auth-foot">
      Remember it now?{" "}
      <button type="button" className="auth-link" onClick={onBack}>
        Sign in
      </button>
    </div>
  </>
);


const ResetSent: React.FC<{
  email: string;
  onResend: () => void;
  onOpenLink: () => void;
  onBack: () => void;
}> = ({ email, onResend, onOpenLink, onBack }) => {
  const [secondsLeft, setSecondsLeft] = useState(30);
  useEffect(() => {
    if (secondsLeft <= 0) return;
    const t = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(t);
  }, [secondsLeft]);

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> Back to sign in
      </button>
      <div className="auth-hero">
        <Icon name="mail" size={26} />
      </div>
      <h1>Check your inbox</h1>
      <p className="sub">
        We sent a reset link to <b style={{ color: "var(--fg-1)" }}>{email}</b>.
        It will expire in 15 minutes.
      </p>
      <div className="auth-note">
        <span className="auth-note-ico">
          <Icon name="bell" size={14} />
        </span>
        <span>Didn't get it? Check your spam folder, or try a different email address.</span>
      </div>
      <button type="button" className="btn primary" onClick={onOpenLink}>
        I clicked the link
      </button>
      <div className="auth-resend">
        {secondsLeft > 0 ? (
          <>
            Resend available in <span className="countdown">{secondsLeft}s</span>
          </>
        ) : (
          <button
            type="button"
            className="auth-link muted"
            onClick={() => {
              setSecondsLeft(30);
              onResend();
            }}
          >
            Resend the email
          </button>
        )}
      </div>
    </>
  );
};


const ResetPassword: React.FC<{
  onSubmit: () => void;
  onBack: () => void;
}> = ({ onSubmit, onBack }) => {
  const [pw, setPw] = useState("");
  const [pw2, setPw2] = useState("");
  const [showPw, setShowPw] = useState(false);

  const { score, label } = passwordStrength(pw);
  const tooShort = pw.length > 0 && pw.length < 8;
  const mismatch = pw2.length > 0 && pw !== pw2;
  const canSubmit = pw.length >= 8 && pw === pw2;

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> Back to sign in
      </button>
      <h1>Set a new password</h1>
      <p className="sub">
        Pick something strong — at least 8 characters, with a mix of letters and
        numbers.
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (canSubmit) onSubmit();
        }}
        autoComplete="off"
      >
        <div className="field has-trail">
          <label>New password</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              autoFocus
              type={showPw ? "text" : "password"}
              placeholder="At least 8 characters"
              value={pw}
              onChange={(e) => setPw(e.target.value)}
            />
            <button
              type="button"
              className="trail"
              onClick={() => setShowPw((v) => !v)}
            >
              <Icon name={showPw ? "eye-off" : "eye"} size={15} />
            </button>
          </div>
          {pw && (
            <>
              <div className={"pw-strength s-" + score}>
                <i /><i /><i /><i />
              </div>
              <div className="pw-strength-label">
                <span>{label}</span>
                {tooShort && <span style={{ color: "#ff453a" }}>Too short</span>}
              </div>
            </>
          )}
        </div>

        <div className="field has-trail">
          <label>Confirm new password</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              type={showPw ? "text" : "password"}
              placeholder="Repeat your password"
              value={pw2}
              onChange={(e) => setPw2(e.target.value)}
              style={
                mismatch
                  ? {
                      borderColor: "#ff453a",
                      boxShadow: "0 0 0 3px rgba(255,69,58,0.18)",
                    }
                  : undefined
              }
            />
          </div>
          {mismatch && (
            <div
              className="pw-strength-label"
              style={{ color: "#ff453a" }}
            >
              Passwords don't match
            </div>
          )}
        </div>

        <button
          type="submit"
          className="btn primary"
          disabled={!canSubmit}
          style={{
            marginTop: 14,
            opacity: canSubmit ? 1 : 0.55,
            cursor: canSubmit ? "pointer" : "not-allowed",
          }}
        >
          Update password
        </button>
      </form>
    </>
  );
};


const ResetSuccess: React.FC<{ onContinue: () => void }> = ({ onContinue }) => (
  <>
    <div className="auth-hero success">
      <Icon name="check" size={28} />
    </div>
    <h1>Password updated</h1>
    <p className="sub">All done. You can now sign in with your new password.</p>
    <button type="button" className="btn primary" onClick={onContinue}>
      Continue to sign in
    </button>
  </>
);


const VerifyEmail: React.FC<{
  email: string;
  onVerified: () => void;
  onBack: () => void;
}> = ({ email, onVerified, onBack }) => {
  const [code, setCode] = useState("");
  const [secondsLeft, setSecondsLeft] = useState(30);
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (secondsLeft <= 0) return;
    const t = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(t);
  }, [secondsLeft]);

  const submitCode = useCallback(
    async (value: string) => {
      if (value.length !== 6 || isSubmitting) return;
      setIsSubmitting(true);
      setError("");
      try {
        await authAPI.confirmRegistration(value);
        const fresh = await authAPI.refresh();
        if (!isAccessTokenVerified(fresh.accessToken)) {
          throw new Error("Verification didn't take effect. Please try again.");
        }
        useAuthStore.getState().setTokens(fresh);
        authAPI.setAuthToken(fresh.accessToken);
        const { user } = useAuthStore.getState();
        if (user) {
          useAuthStore.getState().login(fresh, user);
        }
        onVerified();
      } catch (err: any) {
        setError(err?.message || "Invalid or expired code");
      } finally {
        setIsSubmitting(false);
      }
    },
    [isSubmitting, onVerified],
  );

  useEffect(() => {
    if (code.length === 6) {
      submitCode(code);
    }
  }, [code, submitCode]);

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> Back
      </button>
      <div className="auth-hero">
        <Icon name="mail" size={26} />
      </div>
      <h1>Verify your email</h1>
      <p className="sub">
        Enter the 6-digit code we sent to{" "}
        <b style={{ color: "var(--fg-1)" }}>{email}</b> to finish creating your account.
      </p>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          submitCode(code);
        }}
      >
        <OtpInput
          value={code}
          onChange={(v) => {
            setCode(v);
            setError("");
          }}
          autoFocus
        />
        {error && (
          <div
            className="pw-strength-label"
            style={{
              color: "#ff453a",
              justifyContent: "center",
              marginBottom: 8,
            }}
          >
            {error}
          </div>
        )}
        <button
          type="submit"
          className="btn primary"
          disabled={code.length !== 6 || isSubmitting}
          style={{
            opacity: code.length === 6 && !isSubmitting ? 1 : 0.55,
            cursor:
              code.length === 6 && !isSubmitting ? "pointer" : "not-allowed",
          }}
        >
          {isSubmitting ? (
            <span className="loading-spinner" />
          ) : (
            "Verify and continue"
          )}
        </button>
      </form>

      <div className="auth-resend">
        {secondsLeft > 0 ? (
          <>
            Didn't get a code? Resend in{" "}
            <span className="countdown">{secondsLeft}s</span>
          </>
        ) : (
          <button
            type="button"
            className="auth-link muted"
            onClick={async () => {
              setSecondsLeft(30);
              try {
                await authAPI.requestVerificationCode();
              } catch (err) {
                console.error("[VerifyEmail] resend failed:", err);
              }
            }}
          >
            Resend the code
          </button>
        )}
      </div>
    </>
  );
};


const TwoFa: React.FC<{ onVerified: () => void; onBack: () => void }> = ({
  onVerified,
  onBack,
}) => {
  const [code, setCode] = useState("");

  useEffect(() => {
    if (code.length === 6) {
      const t = setTimeout(() => onVerified(), 250);
      return () => clearTimeout(t);
    }
  }, [code, onVerified]);

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> Back to sign in
      </button>
      <div className="auth-hero">
        <Icon name="shield" size={26} />
      </div>
      <h1>Two-factor authentication</h1>
      <p className="sub">
        Open your authenticator app and enter the 6-digit code for Rift.
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (code.length === 6) onVerified();
        }}
      >
        <OtpInput value={code} onChange={setCode} autoFocus />
        <button
          type="submit"
          className="btn primary"
          disabled={code.length !== 6}
          style={{
            opacity: code.length === 6 ? 1 : 0.55,
            cursor: code.length === 6 ? "pointer" : "not-allowed",
          }}
        >
          Verify
        </button>
      </form>
      <div className="auth-resend">
        Lost access?{" "}
        <button type="button" className="auth-link muted" onClick={() => {}}>
          Use a recovery code
        </button>
      </div>
    </>
  );
};

export default AuthPage;
