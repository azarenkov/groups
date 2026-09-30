import React from "react";
import { useTranslation } from "react-i18next";

import { ApiError } from "../../../shared/lib/apiError";
import { TRIAL_DAYS } from "../lib/plans";
import { useEntitlementsStore } from "../store/entitlementsStore";

const TRIAL_ERRORS: Record<string, string> = {
  TRIAL_ALREADY_USED: "trial.errors.used",
  EMAIL_NOT_VERIFIED: "trial.errors.unverified",
  TRIAL_LIMIT_REACHED: "trial.errors.network",
  ALREADY_PAID: "trial.errors.paid",
};

interface TrialOfferProps {
  compact?: boolean;
  reason?: string;
}

const TrialOffer: React.FC<TrialOfferProps> = ({ compact = false, reason }) => {
  const { t } = useTranslation("subscription");
  const available = useEntitlementsStore((s) => s.trialAvailable);
  const startTrial = useEntitlementsStore((s) => s.startTrial);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  if (!available) return null;

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      await startTrial();
    } catch (err) {
      const code = err instanceof ApiError ? err.code : undefined;
      setError(t(code && TRIAL_ERRORS[code] ? TRIAL_ERRORS[code] : "trial.errors.generic"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={"trial-offer" + (compact ? " compact" : "")}>
      {reason && <span className="trial-offer-reason">{reason}</span>}
      <button type="button" className="s-btn primary trial-offer-button" disabled={busy} onClick={() => void start()}>
        {t("trial.cta", { days: TRIAL_DAYS })}
      </button>
      {!compact && <span className="trial-offer-hint">{t("trial.hint")}</span>}
      {error && (
        <span className="trial-offer-error" role="alert">
          {error}
        </span>
      )}
    </div>
  );
};

export default TrialOffer;
