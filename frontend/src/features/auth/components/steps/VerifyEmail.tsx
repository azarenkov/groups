import React, { useCallback, useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { useAuthStore } from "../../store/authStore";
import { authAPI } from "../../api/auth";
import { userAPI } from "../../../social/api/user";
import { bustAvatar } from "../../../../shared/lib/avatarCache";
import { ApiError } from "../../../../shared/lib/apiError";
import { isAccessTokenVerified } from "../../../../shared/lib/jwt";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft, OtpInput } from "../authHelpers";


export const VerifyEmail: React.FC<{
  email: string;
  onVerified: () => void;
  onBack: () => void;
  onEmailChanged?: (email: string) => void;
  pendingAvatar?: File | null;
}> = ({ email, onVerified, onBack, onEmailChanged, pendingAvatar }) => {
  const { t } = useTranslation("auth");
  const [code, setCode] = useState("");
  const [secondsLeft, setSecondsLeft] = useState(30);
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [avatarWarning, setAvatarWarning] = useState<string | null>(null);
  const [editingEmail, setEditingEmail] = useState(false);
  const [newEmail, setNewEmail] = useState("");
  const [changePassword, setChangePassword] = useState("");
  const [changeBusy, setChangeBusy] = useState(false);
  const [changeError, setChangeError] = useState("");
  const [pendingNewEmail, setPendingNewEmail] = useState<string | null>(null);
  const [pendingChangePassword, setPendingChangePassword] = useState("");

  const submitEmailChange = async () => {
    const target = newEmail.trim();
    if (!target.includes("@") || !changePassword || changeBusy) return;
    setChangeBusy(true);
    setChangeError("");
    try {
      await userAPI.requestEmailChange({ password: changePassword }, target);
      setPendingNewEmail(target);
      setPendingChangePassword(changePassword);
      onEmailChanged?.(target);
      setEditingEmail(false);
      setCode("");
      setError("");
      setSecondsLeft(30);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setChangeError(
        apiErr?.code === "INVALID_PASSWORD"
          ? t("wrongPassword")
          : apiErr?.code === "EMAIL_TAKEN"
            ? t("emailTaken")
            : apiErr?.message || t("changeEmailFailed"),
      );
    } finally {
      setChangeBusy(false);
    }
  };

  useEffect(() => {
    if (secondsLeft <= 0) return;
    const timer = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(timer);
  }, [secondsLeft]);

  const submitCode = useCallback(
    async (value: string) => {
      if (value.length !== 6 || isSubmitting || avatarWarning) return;
      setIsSubmitting(true);
      setError("");
      try {
        if (pendingNewEmail) {
          await userAPI.confirmEmailChange(value);
        } else {
          await authAPI.confirmRegistration(value);
        }
        const fresh = await authAPI.refresh();
        if (!isAccessTokenVerified(fresh.accessToken)) {
          throw new Error(t("verifyTookNoEffect"));
        }
        useAuthStore.getState().setTokens(fresh);
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
    [isSubmitting, onVerified, t, pendingAvatar, avatarWarning, pendingNewEmail],
  );

  useEffect(() => {
    if (code.length === 6 && !avatarWarning) {
      submitCode(code);
    }
  }, [code, submitCode, avatarWarning]);

  return (
    <>
      {avatarWarning ? (
        <>
          <div className="auth-hero">
            <Icon name="camera" size={26} />
          </div>
          <h1>{t("verifyTitle")}</h1>
          <p className="sub error-text">{avatarWarning}</p>
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
      {editingEmail ? (
        <form
          className="verify-change-email"
          onSubmit={(e) => {
            e.preventDefault();
            void submitEmailChange();
          }}
        >
          <div className="field">
            <label>{t("newEmail")}</label>
            <div className="field-input">
              <input
                type="email"
                autoFocus
                autoComplete="email"
                value={newEmail}
                onChange={(e) => setNewEmail(e.target.value)}
              />
            </div>
          </div>
          <div className="field">
            <label>{t("password")}</label>
            <div className="field-input">
              <input
                type="password"
                autoComplete="current-password"
                value={changePassword}
                onChange={(e) => setChangePassword(e.target.value)}
              />
            </div>
          </div>
          {changeError && (
            <div className="pw-strength-label error-text" style={{ color: "#ff453a" }}>{changeError}</div>
          )}
          <div className="verify-change-email-actions">
            <button type="button" className="auth-link muted" onClick={() => setEditingEmail(false)}>
              {t("cancel")}
            </button>
            <button
              type="submit"
              className="auth-link"
              disabled={changeBusy || !newEmail.includes("@") || !changePassword}
            >
              {changeBusy ? <span className="loading-spinner" /> : t("sendCodeToNewEmail")}
            </button>
          </div>
        </form>
      ) : (
        <div className="auth-resend verify-wrong-email">
          {t("wrongEmail")}{" "}
          <button
            type="button"
            className="auth-link"
            onClick={() => {
              setNewEmail("");
              setChangePassword("");
              setChangeError("");
              setEditingEmail(true);
            }}
          >
            {t("changeEmail")}
          </button>
        </div>
      )}

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
            className="pw-strength-label error-text"
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
                if (pendingNewEmail) {
                  await userAPI.requestEmailChange({ password: pendingChangePassword }, pendingNewEmail);
                } else {
                  await authAPI.requestVerificationCode();
                }
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
