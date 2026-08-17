import React, { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useForm } from "react-hook-form";
import { Trans, useTranslation } from "react-i18next";
import { useAuthStore } from "../../store/authStore";
import { currentLocale } from "../../i18n";
import { authAPI, TokenPair } from "../../api/auth";
import { userAPI } from "../../api/user";
import { bustAvatar } from "../../utils/avatarCache";
import { ApiError } from "../../utils/apiError";
import { isAccessTokenVerified, decodeAccessToken } from "../../utils/jwt";
import Icon, { Mark } from "../../components/rift/Icon";
import LanguageSwitcher from "../../components/i18n/LanguageSwitcher";
import GoogleSignInButton from "../../components/auth/GoogleSignInButton";
import {
  ArrowLeft,
  OtpInput,
  passwordStrength,
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
  const tokens = useAuthStore((s) => s.tokens);
  const storedUser = useAuthStore((s) => s.user);
  const hasUnverifiedSession =
    !!tokens?.accessToken && !isAccessTokenVerified(tokens.accessToken);

  const [mode, setMode] = useState<Mode>(
    hasUnverifiedSession ? "verify-email" : defaultMode,
  );
  const [email, setEmail] = useState(storedUser?.email ?? "");

  useEffect(() => {
    if (hasUnverifiedSession) {
      setMode("verify-email");
      if (storedUser?.email) setEmail(storedUser.email);
    }
  }, [hasUnverifiedSession, storedUser?.email]);

  const onSignedIn = () => navigate("/app");
  const [pendingAvatar, setPendingAvatar] = useState<File | null>(null);

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
        <LanguageSwitcher />
      </div>

      <div className="auth-form-wrap">
        <div className="auth-card">
          {showTabs && <AuthTabs mode={mode} onSwitch={switchTab} />}

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
              onAvatarPicked={setPendingAvatar}
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
              pendingAvatar={pendingAvatar}
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

      <AuthLegal />
    </div>
  );
};

const AuthLegal: React.FC = () => {
  const { t } = useTranslation("auth");
  return (
    <div className="auth-legal">
      <span>© 2026 Rift</span>
      <button type="button" className="auth-link" onClick={() => {}}>{t("privacy")}</button>
      <button type="button" className="auth-link" onClick={() => {}}>{t("terms")}</button>
      <button type="button" className="auth-link" onClick={() => {}}>{t("status")}</button>
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
  onAvatarPicked: (file: File | null) => void;
}

const AuthTabs: React.FC<{
  mode: "signin" | "signup";
  onSwitch: (m: "signin" | "signup") => void;
}> = ({ mode, onSwitch }) => {
  const { t } = useTranslation("auth");
  return (
    <div className="auth-tabs" role="tablist">
      <button
        role="tab"
        type="button"
        className={mode === "signin" ? "on" : ""}
        onClick={() => onSwitch("signin")}
      >
        {t("signIn")}
      </button>
      <button
        role="tab"
        type="button"
        className={mode === "signup" ? "on" : ""}
        onClick={() => onSwitch("signup")}
      >
        {t("createAccount")}
      </button>
    </div>
  );
};

const SignInUp: React.FC<SignInUpProps> = ({
  mode,
  email,
  onEmailChange,
  onSwitchMode,
  onSignedIn,
  goVerify,
  goForgot,
  onAvatarPicked,
}) => {
  const isSignup = mode === "signup";
  const { login, setLoading, setError, error, isLoading } = useAuthStore();
  const { t } = useTranslation("auth");
  const locale = currentLocale();

  const signInForm = useForm<SignInData>({
    defaultValues: { email, password: "" },
  });
  const signUpForm = useForm<SignUpData>({
    defaultValues: { name: "", email, password: "" },
  });

  const [showPw, setShowPw] = useState(false);
  const [pendingGoogleToken, setPendingGoogleToken] = useState<string | null>(
    null,
  );
  const [avatarFile, setAvatarFile] = useState<File | null>(null);
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null);
  const avatarInputRef = React.useRef<HTMLInputElement>(null);
  const [googleUsername, setGoogleUsername] = useState("");

  const finishGoogleSession = async (response: TokenPair) => {
    authAPI.setAuthToken(response.accessToken);
    const tokens = {
      accessToken: response.accessToken,
      refreshToken: response.refreshToken,
      tokenType: response.tokenType,
      expiresIn: response.expiresIn,
    };
    const claims = decodeAccessToken(response.accessToken);
    login(tokens, { id: claims?.sub ?? "", name: "", email: "" });
    try {
      await useAuthStore.getState().fetchMe();
    } catch (meErr) {
      console.warn("[Auth] fetchMe failed:", meErr);
    }
    if (isSignup && avatarFile) {
      try {
        await userAPI.uploadAvatar(avatarFile);
        const uid = useAuthStore.getState().user?.id;
        if (uid) bustAvatar("user", uid);
      } catch (avatarErr) {
        console.warn("[Auth] avatar upload failed:", avatarErr);
      }
    }
    onSignedIn();
  };

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
        try {
          await useAuthStore.getState().fetchMe();
        } catch (meErr) {
          console.warn("[Auth] fetchMe failed:", meErr);
        }
        onSignedIn();
      }
    } catch (err: any) {
      setError(err.message || t("couldNotSignIn"));
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
        locale,
      );
      authAPI.setAuthToken(response.accessToken);
      useAuthStore.getState().setTokens({
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        tokenType: response.tokenType,
        expiresIn: response.expiresIn,
      });
      const newUserId = decodeAccessToken(response.accessToken)?.sub ?? "";
      useAuthStore.getState().setUser({
        id: newUserId,
        name: data.name,
        email: data.email,
      });
      onAvatarPicked(avatarFile);
      try {
        await authAPI.requestVerificationCode();
      } catch (codeErr) {
        console.error("[Auth] requestVerificationCode failed:", codeErr);
      }
      goVerify();
    } catch (err: any) {
      setError(err.message || t("couldNotRegister"));
    } finally {
      setLoading(false);
    }
  };

  const handleGoogleToken = async (idToken: string) => {
    try {
      setLoading(true);
      setError(null);
      const response = await authAPI.googleLogin(idToken, locale);
      if (response.needsUsername) {
        setGoogleUsername("");
        setPendingGoogleToken(idToken);
        return;
      }
      await finishGoogleSession(response);
    } catch (err: any) {
      setError(err.message || t("couldNotSignIn"));
    } finally {
      setLoading(false);
    }
  };

  const handleGoogleComplete = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pendingGoogleToken) return;
    const name = googleUsername.trim();
    if (name.length < 3 || name.length > 15 || !/^[a-zA-Z0-9_]+$/.test(name)) {
      setError(t("usernameInvalid"));
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const response = await authAPI.googleComplete(
        pendingGoogleToken,
        name,
        locale,
      );
      setPendingGoogleToken(null);
      await finishGoogleSession(response);
    } catch (err: any) {
      if (err.code === "GOOGLE_USERNAME_TAKEN") {
        setError(t("usernameTaken"));
      } else {
        setError(err.message || t("couldNotSignIn"));
      }
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

  if (pendingGoogleToken) {
    return (
      <>
        <h1>{t("chooseUsernameTitle")}</h1>
        <p className="sub">{t("chooseUsernameSubtitle")}</p>

        <form onSubmit={handleGoogleComplete} autoComplete="off">
          <div className="field">
            <label>{t("displayName")}</label>
            <div className="field-input">
              <span className="lead"><Icon name="user" size={15} /></span>
              <input
                type="text"
                autoFocus
                placeholder={t("displayNamePlaceholder")}
                value={googleUsername}
                onChange={(e) => setGoogleUsername(e.target.value)}
              />
            </div>
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
            {isLoading ? <span className="loading-spinner" /> : t("continue")}
          </button>
        </form>

        <div className="auth-foot">
          <button
            type="button"
            className="auth-link"
            onClick={() => {
              setPendingGoogleToken(null);
              setError(null);
            }}
          >
            {t("backToSignIn")}
          </button>
        </div>
      </>
    );
  }

  return (
    <>
      <h1>{isSignup ? t("createTitle") : t("welcomeBack")}</h1>
      <p className="sub">{isSignup ? t("createSubtitle") : t("signInSubtitle")}</p>

      <form onSubmit={submit} autoComplete="off">
        {isSignup && (
          <div className="auth-avatar-pick">
            <input
              ref={avatarInputRef}
              type="file"
              accept="image/png,image/jpeg,image/webp"
              hidden
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) {
                  setAvatarFile(f);
                  setAvatarPreview((prev) => {
                    if (prev) URL.revokeObjectURL(prev);
                    return URL.createObjectURL(f);
                  });
                }
                e.target.value = "";
              }}
            />
            <button
              type="button"
              className="auth-avatar-circle"
              title={t("avatarOptional")}
              onClick={() => avatarInputRef.current?.click()}
            >
              {avatarPreview ? (
                <img src={avatarPreview} alt="" />
              ) : (
                <Icon name="camera" size={18} />
              )}
            </button>
            <div className="auth-avatar-text">
              <span>{t("avatarOptional")}</span>
              {avatarFile && (
                <button
                  type="button"
                  className="auth-avatar-clear"
                  onClick={() => {
                    setAvatarFile(null);
                    setAvatarPreview((prev) => {
                      if (prev) URL.revokeObjectURL(prev);
                      return null;
                    });
                  }}
                >
                  {t("avatarRemove")}
                </button>
              )}
            </div>
          </div>
        )}
        {isSignup && (
          <div className="field">
            <label>{t("displayName")}</label>
            <div className="field-input">
              <span className="lead"><Icon name="user" size={15} /></span>
              <input
                type="text"
                placeholder={t("displayNamePlaceholder")}
                {...signUpForm.register("name", {
                  required: t("displayNameRequired"),
                  minLength: { value: 3, message: t("displayNameMin") },
                  maxLength: { value: 15, message: t("displayNameMax") },
                  pattern: { value: /^[a-zA-Z0-9_]+$/, message: t("displayNameInvalid") },
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
          <label>{t("email")}</label>
          <div className="field-input">
            <span className="lead"><Icon name="mail" size={15} /></span>
            <input
              type="email"
              placeholder={t("emailPlaceholder")}
              {...registerEmail("email", {
                required: t("emailRequired"),
                pattern: {
                  value: /^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i,
                  message: t("emailInvalid"),
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
          <label>{t("password")}</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              type={showPw ? "text" : "password"}
              placeholder={isSignup ? t("passwordPlaceholderSignup") : t("passwordPlaceholder")}
              {...registerPassword("password", {
                required: t("passwordRequired"),
                minLength: { value: 4, message: t("passwordMin") },
              })}
            />
            <button
              type="button"
              className="trail"
              onClick={() => setShowPw((v) => !v)}
              title={showPw ? t("hidePassword") : t("showPassword")}
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
              <span>{t("sendUpdates")}</span>
            </label>
          ) : (
            <span />
          )}
          {!isSignup && (
            <button type="button" className="auth-link" onClick={goForgot}>
              {t("forgotPassword")}
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
          {isLoading ? (
            <span className="loading-spinner" />
          ) : isSignup ? (
            t("createAccount")
          ) : (
            t("signIn")
          )}
        </button>
      </form>

      <GoogleSignInButton
        onToken={handleGoogleToken}
        text={isSignup ? "signup_with" : "signin_with"}
      />

      <div className="auth-foot">
        {isSignup ? (
          <button
            type="button"
            className="auth-link"
            onClick={() => onSwitchMode("signin")}
          >
            {t("haveAccount")}
          </button>
        ) : (
          <button
            type="button"
            className="auth-link"
            onClick={() => onSwitchMode("signup")}
          >
            {t("needAccount")}
          </button>
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
}> = ({ email, onEmailChange, onSubmit, onBack }) => {
  const { t } = useTranslation("auth");
  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> {t("backToSignIn")}
      </button>
      <h1>{t("forgotTitle")}</h1>
      <p className="sub">{t("forgotSubtitle")}</p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit();
        }}
        autoComplete="off"
      >
        <div className="field">
          <label>{t("email")}</label>
          <div className="field-input">
            <span className="lead"><Icon name="mail" size={15} /></span>
            <input
              type="email"
              autoFocus
              placeholder={t("emailPlaceholder")}
              value={email}
              onChange={(e) => onEmailChange(e.target.value)}
            />
          </div>
        </div>
        <button type="submit" className="btn primary" style={{ marginTop: 14 }}>
          {t("sendResetLink")}
        </button>
      </form>
      <div className="auth-foot">
        <button type="button" className="auth-link" onClick={onBack}>
          {t("rememberItNow")}
        </button>
      </div>
    </>
  );
};


const ResetSent: React.FC<{
  email: string;
  onResend: () => void;
  onOpenLink: () => void;
  onBack: () => void;
}> = ({ email, onResend, onOpenLink, onBack }) => {
  const { t } = useTranslation("auth");
  const [secondsLeft, setSecondsLeft] = useState(30);
  useEffect(() => {
    if (secondsLeft <= 0) return;
    const timer = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(timer);
  }, [secondsLeft]);

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> {t("backToSignIn")}
      </button>
      <div className="auth-hero">
        <Icon name="mail" size={26} />
      </div>
      <h1>{t("checkInbox")}</h1>
      <p className="sub">
        <Trans i18nKey="checkInboxBody" t={t} values={{ email }} components={[<b key="e" style={{ color: "var(--fg-1)" }} />]} />
      </p>
      <div className="auth-note">
        <span className="auth-note-ico">
          <Icon name="bell" size={14} />
        </span>
        <span>{t("checkInboxHint")}</span>
      </div>
      <button type="button" className="btn primary" onClick={onOpenLink}>
        {t("clickedLink")}
      </button>
      <div className="auth-resend">
        {secondsLeft > 0 ? (
          <>
            {t("resendIn")} <span className="countdown">{secondsLeft}s</span>
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
            {t("resendEmail")}
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
  const { t } = useTranslation("auth");
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
        <ArrowLeft /> {t("backToSignIn")}
      </button>
      <h1>{t("setNewPassword")}</h1>
      <p className="sub">{t("setNewPasswordHint")}</p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (canSubmit) onSubmit();
        }}
        autoComplete="off"
      >
        <div className="field has-trail">
          <label>{t("newPassword")}</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              autoFocus
              type={showPw ? "text" : "password"}
              placeholder={t("passwordPlaceholderSignup")}
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
                {tooShort && <span style={{ color: "#ff453a" }}>{t("tooShort")}</span>}
              </div>
            </>
          )}
        </div>

        <div className="field has-trail">
          <label>{t("confirmNewPassword")}</label>
          <div className="field-input">
            <span className="lead"><Icon name="lock2" size={15} /></span>
            <input
              type={showPw ? "text" : "password"}
              placeholder={t("repeatPassword")}
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
            <div className="pw-strength-label" style={{ color: "#ff453a" }}>
              {t("passwordsDontMatch")}
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
          {t("updatePassword")}
        </button>
      </form>
    </>
  );
};


const ResetSuccess: React.FC<{ onContinue: () => void }> = ({ onContinue }) => {
  const { t } = useTranslation("auth");
  return (
    <>
      <div className="auth-hero success">
        <Icon name="check" size={28} />
      </div>
      <h1>{t("passwordUpdated")}</h1>
      <p className="sub">{t("passwordUpdatedHint")}</p>
      <button type="button" className="btn primary" onClick={onContinue}>
        {t("continueToSignIn")}
      </button>
    </>
  );
};


const VerifyEmail: React.FC<{
  email: string;
  onVerified: () => void;
  onBack: () => void;
  pendingAvatar?: File | null;
}> = ({ email, onVerified, onBack, pendingAvatar }) => {
  const { t } = useTranslation("auth");
  const [code, setCode] = useState("");
  const [secondsLeft, setSecondsLeft] = useState(30);
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [avatarWarning, setAvatarWarning] = useState<string | null>(null);

  useEffect(() => {
    if (secondsLeft <= 0) return;
    const timer = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(timer);
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
          throw new Error(t("verifyTookNoEffect"));
        }
        useAuthStore.getState().setTokens(fresh);
        authAPI.setAuthToken(fresh.accessToken);
        const { user } = useAuthStore.getState();
        if (user) {
          useAuthStore.getState().login(fresh, user);
        }
        try {
          await useAuthStore.getState().fetchMe();
        } catch (meErr) {
          console.warn("[Auth] fetchMe failed:", meErr);
        }
        if (pendingAvatar) {
          try {
            await userAPI.uploadAvatar(pendingAvatar);
            const uid = useAuthStore.getState().user?.id;
            if (uid) bustAvatar("user", uid);
          } catch (avatarErr) {
            console.warn("[Auth] avatar upload failed:", avatarErr);
            const apiErr = avatarErr instanceof ApiError ? avatarErr : null;
            const reason =
              apiErr?.code === "IMAGE_CONTENT_REJECTED"
                ? t("avatarRejectedReason")
                : apiErr?.message || t("avatarUploadGenericReason");
            setAvatarWarning(t("avatarUploadFailed", { reason }));
            return;
          }
        }
        onVerified();
      } catch (err: any) {
        setError(err?.message || t("invalidOrExpiredCode"));
      } finally {
        setIsSubmitting(false);
      }
    },
    [isSubmitting, onVerified, t, pendingAvatar],
  );

  useEffect(() => {
    if (code.length === 6) {
      submitCode(code);
    }
  }, [code, submitCode]);

  return (
    <>
      {avatarWarning ? (
        <>
          <div className="auth-hero">
            <Icon name="camera" size={26} />
          </div>
          <h1>{t("verifyTitle")}</h1>
          <p className="sub">{avatarWarning}</p>
          <button
            type="button"
            className="btn primary"
            onClick={onVerified}
          >
            {t("continue")}
          </button>
        </>
      ) : (
        <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> {t("backToSignIn")}
      </button>
      <div className="auth-hero">
        <Icon name="mail" size={26} />
      </div>
      <h1>{t("verifyTitle")}</h1>
      <p className="sub">
        {email ? (
          <Trans i18nKey="verifyBody" t={t} values={{ email }} components={[<b key="e" style={{ color: "var(--fg-1)" }} />]} />
        ) : (
          <Trans i18nKey="verifyBodyNoEmail" t={t} components={[<b key="e" style={{ color: "var(--fg-1)" }} />]} />
        )}
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
          {isSubmitting ? <span className="loading-spinner" /> : t("verifyAndContinue")}
        </button>
      </form>

      <div className="auth-resend">
        {secondsLeft > 0 ? (
          <>
            {t("didntGetCode")}{" "}
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
            {t("resendCode")}
          </button>
        )}
      </div>
        </>
      )}
    </>
  );
};


const TwoFa: React.FC<{ onVerified: () => void; onBack: () => void }> = ({
  onVerified,
  onBack,
}) => {
  const { t } = useTranslation("auth");
  const [code, setCode] = useState("");

  useEffect(() => {
    if (code.length === 6) {
      const timer = setTimeout(() => onVerified(), 250);
      return () => clearTimeout(timer);
    }
  }, [code, onVerified]);

  return (
    <>
      <button type="button" className="auth-back" onClick={onBack}>
        <ArrowLeft /> {t("backToSignIn")}
      </button>
      <div className="auth-hero">
        <Icon name="shield" size={26} />
      </div>
      <h1>{t("twoFaTitle")}</h1>
      <p className="sub">{t("twoFaBody")}</p>
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
          {t("verify")}
        </button>
      </form>
      <div className="auth-resend">
        {t("lostAccess")}{" "}
        <button type="button" className="auth-link muted" onClick={() => {}}>
          {t("useRecoveryCode")}
        </button>
      </div>
    </>
  );
};

export default AuthPage;
