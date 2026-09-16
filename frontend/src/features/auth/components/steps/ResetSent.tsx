import React, { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft } from "../authHelpers";


export const ResetSent: React.FC<{
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
