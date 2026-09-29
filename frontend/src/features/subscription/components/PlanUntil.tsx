import React from "react";
import { useTranslation } from "react-i18next";

import { useEntitlementsStore } from "../store/entitlementsStore";

export const formatPlanDate = (iso: string, locale: string) =>
  new Date(iso).toLocaleDateString(locale, { day: "numeric", month: "long" });

const PlanUntil: React.FC<{ className?: string }> = ({ className }) => {
  const { t, i18n } = useTranslation("subscription");
  const tier = useEntitlementsStore((s) => s.tier);
  const validUntil = useEntitlementsStore((s) => s.validUntil);
  const trial = useEntitlementsStore((s) => s.trial);
  if (tier === "free" || !validUntil) return null;
  const date = formatPlanDate(validUntil, i18n.language);
  return (
    <span className={"plan-until" + (className ? ` ${className}` : "")}>
      {trial
        ? t("trial.until", { date })
        : t("planUntil", { plan: t(`page.plans.${tier}.name`), date })}
    </span>
  );
};

export default PlanUntil;
