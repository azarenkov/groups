import React, { useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "../../store/authStore";
import { currentLocale } from "../../../../i18n";
import { authAPI, TokenPair } from "../../api/auth";
import { userAPI } from "../../../social/api/user";
import { bustAvatar } from "../../../../shared/lib/avatarCache";
import { ApiError } from "../../../../shared/lib/apiError";
import { validateUsername, UsernameIssue } from "../../../../shared/lib/username";
import { isAccessTokenVerified, decodeAccessToken } from "../../../../shared/lib/jwt";
import Icon from "../../../../shared/ui/Icon";
import GoogleSignInButton from "../GoogleSignInButton";


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

export const AuthTabs: React.FC<{
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

export const SignInUp: React.FC<SignInUpProps> = ({
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

  const usernameIssueMessage = (issue: UsernameIssue): string => {
    switch (issue) {
      case "required":
        return t("displayNameRequired");
      case "tooShort":
        return t("displayNameMin");
      case "tooLong":
        return t("displayNameMax");
      case "charset":
        return t("displayNameInvalid");
      case "underscoreEdge":
      case "doubleUnderscore":
        return t("displayNameUnderscore");
      case "digitsOnly":
        return t("displayNameDigits");
      case "reserved":
        return t("displayNameReserved");
    }
  };

  const usernameServerError = (err: any): string | null => {
    if (!(err instanceof ApiError)) return null;
    if (err.code === "NAME_TAKEN" || err.code === "GOOGLE_USERNAME_TAKEN") {
      return t("usernameTaken");
    }
    if (err.code === "NAME_NOT_ALLOWED") return t("usernameNotAllowed");
    if (err.code === "NAME_INVALID_FORMAT") return t("usernameInvalid");
    if (err.code === "MODERATION_UNAVAILABLE") {
      return t("usernameModerationUnavailable");
    }
    return null;
  };

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
    const name = data.name.trim();
    try {
      setLoading(true);
      setError(null);
      const response = await authAPI.register(
        name,
        data.email,
        data.password,
        locale,
      );
        useAuthStore.getState().setTokens({
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        tokenType: response.tokenType,
        expiresIn: response.expiresIn,
      });
      const newUserId = decodeAccessToken(response.accessToken)?.sub ?? "";
      useAuthStore.getState().setUser({
        id: newUserId,
        name,
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
      setError(usernameServerError(err) || err.message || t("couldNotRegister"));
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
    const issue = validateUsername(name);
    if (issue !== null) {
      setError(usernameIssueMessage(issue));
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
      setError(usernameServerError(err) || err.message || t("couldNotSignIn"));
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
                  validate: (value: string) => {
                    const issue = validateUsername(value);
                    return issue === null || usernameIssueMessage(issue);
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
