import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { authAPI } from "../../api/auth";
import { ApiError } from "../../../../shared/lib/apiError";
import { ArrowLeft, FormError, passwordStrength } from "../authHelpers";

export const RESET_PASSWORD_MIN = 4;
export const RESET_PASSWORD_MAX = 30;

export const ResetPassword: React.FC<{
  token: string | null;
  onDone: () => void;
  onRequestNew: () => void;
  onBack: () => void;
}> = ({ token, onDone, onRequestNew, onBack }) => {
  const { t } = useTranslation("auth");
  const [pw, setPw] = useState("");
  const [pw2, setPw2] = useState("");
  const [showPw, setShowPw] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [linkDead, setLinkDead] = useState(!token);

  const { score, label } = passwordStrength(pw);
  const tooShort = pw.length > 0 && pw.length < RESET_PASSWORD_MIN;
  const mismatch = pw2.length > 0 && pw !== pw2;
  const canSubmit = !!token && !saving && pw.length >= RESET_PASSWORD_MIN && pw === pw2;

  const submit = async () => {
    if (!token || !canSubmit) return;
    setSaving(true);
    setError(null);
    try {
      await authAPI.resetPassword(token, pw);
      onDone();
    } catch (err) {
      const code = err instanceof ApiError ? err.code : undefined;
      if (code === "RESET_TOKEN_EXPIRED") {
        setLinkDead(true);
        setError(t("resetLinkExpired"));
      } else if (code === "RESET_TOKEN_INVALID") {
        setLinkDead(true);
        setError(t("resetLinkInvalid"));
      } else {
        setError(t("resetFailed"));
      }
    } finally {
      setSaving(false);
    }
  };

  if (linkDead) {
    return (
      <>
        <button type="button" className="auth-back" onClick={onBack}>
          <ArrowLeft /> {t("backToSignIn")}
        </button>
        <h1>{t("resetLinkDeadTitle")}</h1>
        <p className="sub">{error ?? t("resetLinkInvalid")}</p>
        <button type="button" className="btn primary" onClick={onRequestNew}>
          {t("requestNewLink")}
        </button>
      </>
    );
  }

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
          void submit();
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
              maxLength={RESET_PASSWORD_MAX}
              autoComplete="new-password"
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
              maxLength={RESET_PASSWORD_MAX}
              autoComplete="new-password"
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

        {error && <FormError>{error}</FormError>}

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
          {saving ? <span className="loading-spinner" /> : t("updatePassword")}
        </button>
      </form>
    </>
  );
};
