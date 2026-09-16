import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft, passwordStrength } from "../authHelpers";


export const ResetPassword: React.FC<{
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
