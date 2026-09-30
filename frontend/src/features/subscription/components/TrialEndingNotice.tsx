import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { dismissTrialNotice, trialEndsSoon, trialNoticeDismissed } from "../lib/trialNotice";
import { useEntitlementsStore } from "../store/entitlementsStore";
import { formatPlanDate } from "./PlanUntil";

const RECHECK_MS = 60 * 1000;

const TrialEndingNotice: React.FC<{ onOpenSubscription: () => void }> = ({ onOpenSubscription }) => {
  const { t, i18n } = useTranslation("subscription");
  const trial = useEntitlementsStore((s) => s.trial);
  const validUntil = useEntitlementsStore((s) => s.validUntil);
  const [now, setNow] = React.useState(() => Date.now());
  const [dismissed, setDismissed] = React.useState(false);

  React.useEffect(() => {
    setDismissed(validUntil ? trialNoticeDismissed(validUntil) : false);
  }, [validUntil]);

  React.useEffect(() => {
    if (!trial) return;
    const timer = window.setInterval(() => setNow(Date.now()), RECHECK_MS);
    return () => window.clearInterval(timer);
  }, [trial]);

  if (dismissed || !validUntil || !trialEndsSoon(trial, validUntil, now)) return null;

  const close = () => {
    dismissTrialNotice(validUntil);
    setDismissed(true);
  };

  return (
    <div className="trial-ending-notice" role="status">
      <Icon name="sparkles" size={14} />
      <span className="trial-ending-text">
        {t("trial.endingSoon", { date: formatPlanDate(validUntil, i18n.language) })}
      </span>
      <button
        type="button"
        className="trial-ending-link"
        onClick={() => {
          close();
          onOpenSubscription();
        }}
      >
        {t("trial.endingSoonAction")}
      </button>
      <button type="button" className="trial-ending-close" onClick={close} aria-label={t("trial.dismiss")}>
        <Icon name="x" size={12} />
      </button>
    </div>
  );
};

export default TrialEndingNotice;
