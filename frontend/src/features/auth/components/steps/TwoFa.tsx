import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft, OtpInput } from "../authHelpers";


export const TwoFa: React.FC<{ onVerified: () => void; onBack: () => void }> = ({
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

