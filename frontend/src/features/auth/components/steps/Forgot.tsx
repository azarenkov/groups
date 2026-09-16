import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft } from "../authHelpers";


export const Forgot: React.FC<{
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
