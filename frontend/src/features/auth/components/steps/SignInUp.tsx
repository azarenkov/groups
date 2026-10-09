import React, { useState } from "react";
import i18n from "../../../../i18n";
import { cropOf, renderCropPreview } from "../../../../shared/lib/imageCrop";
import { useImageCrop } from "../../../../shared/hooks/useImageCrop";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "../../store/authStore";
import { currentLocale } from "../../../../i18n";
import { authAPI, TokenPair } from "../../api/auth";
import { ApiError } from "../../../../shared/lib/apiError";
import { validateUsername, UsernameIssue } from "../../../../shared/lib/username";
import { isAccessTokenVerified, decodeAccessToken } from "../../../../shared/lib/jwt";
import Icon from "../../../../shared/ui/Icon";
import { CaptchaSolveError } from "../../lib/captcha";
import { isCaptchaError, useCaptcha } from "../../lib/useCaptcha";


interface SignInData {
  email: string;
  password: string;
}
interface SignUpData {
  name: string;
  email: string;
  password: string;
  confirmPassword: string;
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
    if (err.code === "NAME_TAKEN") return t("usernameTaken");
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
    defaultValues: { name: "", email, password: "", confirmPassword: "" },
  });

  const [showPw, setShowPw] = useState(false);
  const [avatarFile, setAvatarFile] = useState<File | null>(null);
  const { requestCrop, cropModal } = useImageCrop();
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null);
  const avatarInputRef = React.useRef<HTMLInputElement>(null);
  const captcha = useCaptcha();
  const { warmUp: warmUpCaptcha } = captcha;
  const loginCaptchaRef = React.useRef(false);

  React.useEffect(() => {
    if (isSignup) warmUpCaptcha();
  }, [isSignup, warmUpCaptcha]);

  const captchaMessage = (err: unknown): string | null => {
    if (err instanceof CaptchaSolveError || isCaptchaError(err)) return t("captchaFailed");
    return null;
  };

  const handleSignIn = async (data: SignInData) => {
    try {
      setLoading(true);
      setError(null);
      let response: TokenPair;
      try {
        response = await authAPI.login(
          data.email,
          data.password,
          loginCaptchaRef.current ? await captcha.take() : null,
        );
      } catch (err) {
        if (!isCaptchaError(err)) throw err;
        loginCaptchaRef.current = true;
        response = await authAPI.login(data.email, data.password, await captcha.take());
      }
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
      if (loginCaptchaRef.current) captcha.warmUp();
      setError(captchaMessage(err) || err.message || t("couldNotSignIn"));
    } finally {
      setLoading(false);
    }
  };

  const handleSignUp = async (data: SignUpData) => {
    const name = data.name.trim();
    try {
      setLoading(true);
      setError(null);
      const register = async () =>
        authAPI.register(name, data.email, data.password, locale, await captcha.take());
      let response: TokenPair;
      try {
        response = await register();
      } catch (err) {
        if (!isCaptchaError(err)) throw err;
        response = await register();
      }
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
      captcha.warmUp();
      setError(
        usernameServerError(err) || captchaMessage(err) || err.message || t("couldNotRegister"),
      );
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
      {cropModal}
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
                e.target.value = "";
                if (!f) return;
                void requestCrop(f, {
                  aspect: 1,
                  shape: "circle",
                  title: i18n.t("cropAvatar", { ns: "common" }),
                }).then(async (cropped) => {
                  if (!cropped) return;
                  const rect = cropOf(cropped);
                  const preview = rect ? await renderCropPreview(cropped, rect, 192) : null;
                  setAvatarFile(cropped);
                  setAvatarPreview((prev) => {
                    if (prev && prev.startsWith("blob:")) URL.revokeObjectURL(prev);
                    return preview ?? URL.createObjectURL(cropped);
                  });
                });
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
                      if (prev && prev.startsWith("blob:")) URL.revokeObjectURL(prev);
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
              <span className="error-text" style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
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
            <span className="error-text" style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
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
            <span className="error-text" style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
              {errors.password.message}
            </span>
          )}
        </div>

        {isSignup && (
          <div className="field">
            <label>{t("confirmNewPassword")}</label>
            <div className="field-input">
              <span className="lead"><Icon name="lock2" size={15} /></span>
              <input
                type={showPw ? "text" : "password"}
                placeholder={t("repeatPassword")}
                autoComplete="new-password"
                {...signUpForm.register("confirmPassword", {
                  required: t("passwordsDontMatch"),
                  validate: (value) => value === signUpForm.getValues("password") || t("passwordsDontMatch"),
                })}
              />
            </div>
            {errors.confirmPassword && (
              <span className="error-text" style={{ fontSize: 11, color: "var(--danger)", marginTop: 4 }}>
                {errors.confirmPassword.message}
              </span>
            )}
          </div>
        )}

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
            className="error-text"
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
          {captcha.solving ? (
            <>
              <span className="loading-spinner" /> {t("captchaChecking")}
            </>
          ) : isLoading ? (
            <span className="loading-spinner" />
          ) : isSignup ? (
            t("createAccount")
          ) : (
            t("signIn")
          )}
        </button>
      </form>

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
