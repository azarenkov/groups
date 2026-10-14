import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft, FormError } from "../authHelpers";
import { useForgotPassword } from "../../lib/useForgotPassword";
import LoadingDots from "../../../../shared/ui/LoadingDots";


export const Forgot: React.FC<{
  email: string;
  onEmailChange: (e: string) => void;
  onSent: () => void;
  onBack: () => void;
}> = ({ email, onEmailChange, onSent, onBack }) => {
  const { t } = useTranslation("auth");
  const { send, sending, solving, error } = useForgotPassword();
  const canSubmit = email.trim().length > 0 && !sending;
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
          if (!canSubmit) return;
          void send(email).then((ok) => {
            if (ok) onSent();
          });
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
              required
              placeholder={t("emailPlaceholder")}
              value={email}
              onChange={(e) => onEmailChange(e.target.value)}
            />
          </div>
        </div>
        {error && <FormError>{error}</FormError>}
        <button type="submit" className="btn primary" disabled={!canSubmit} style={{ marginTop: 14 }}>
          {solving ? (
            <>
              <LoadingDots /> {t("captchaChecking")}
            </>
          ) : sending ? (
            <LoadingDots />
          ) : (
            t("sendResetLink")
          )}
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
