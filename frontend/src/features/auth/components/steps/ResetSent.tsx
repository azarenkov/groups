import React, { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { ArrowLeft, FormError } from "../authHelpers";
import { useForgotPassword } from "../../lib/useForgotPassword";

export const RESEND_RESET_AFTER_S = 60;

export const ResetSent: React.FC<{
  email: string;
  onBack: () => void;
}> = ({ email, onBack }) => {
  const { t } = useTranslation("auth");
  const { send, sending, error } = useForgotPassword();
  const [resendAt, setResendAt] = useState(() => Date.now() + RESEND_RESET_AFTER_S * 1000);
  const [now, setNow] = useState(() => Date.now());
  const secondsLeft = Math.max(0, Math.ceil((resendAt - now) / 1000));
  const counting = secondsLeft > 0;
  useEffect(() => {
    if (!counting) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [counting]);

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
      {error && <FormError>{error}</FormError>}
      <div className="auth-resend">
        {secondsLeft > 0 ? (
          <>
            {t("resendIn")} <span className="countdown">{secondsLeft}s</span>
          </>
        ) : (
          <button
            type="button"
            className="auth-link muted"
            disabled={sending}
            onClick={() => {
              void send(email).then((ok) => {
                if (!ok) return;
                const sentAt = Date.now();
                setNow(sentAt);
                setResendAt(sentAt + RESEND_RESET_AFTER_S * 1000);
              });
            }}
          >
            {t("resendEmail")}
          </button>
        )}
      </div>
    </>
  );
};
