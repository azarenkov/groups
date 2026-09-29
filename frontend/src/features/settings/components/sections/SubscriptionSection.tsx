import { useTranslation } from "react-i18next";

import { SettingsRow } from "../controls";
import ProBadge from "../../../subscription/components/ProBadge";
import {
  VIDEO_NOTE_SIZE_BY_TIER,
  VOICE_MESSAGE_MAX_MS_BY_TIER,
  type Tier,
} from "../../../subscription/lib/plans";
import { useEntitlementsStore } from "../../../subscription/store/entitlementsStore";

const PLANS: { tier: Tier; features: string[] }[] = [
  { tier: "free", features: ["noise", "attachments", "voice", "videoNote", "stream"] },
  { tier: "pro", features: ["attachments", "voice", "videoNote", "stream", "badge", "banner", "avatar"] },
  { tier: "premium", features: ["attachments", "voice", "videoNote", "stream", "badge", "banner", "avatar", "support"] },
];

export function SubscriptionSection() {
  const { t } = useTranslation("subscription");
  const current = useEntitlementsStore((s) => s.tier);
  return (
    <>
      <h1>{t("page.title")}</h1>
      <p className="lead">{t("page.lead")}</p>

      <div className="s-section">
        <div className="plan-grid">
          {PLANS.map(({ tier, features }) => (
            <div
              key={tier}
              className={
                "plan-tile" +
                (tier === current ? " current" : "") +
                (tier === "premium" ? " soon" : "")
              }
            >
              <div className="plan-tile-head">
                <span className="plan-tile-name">{t(`page.plans.${tier}.name`)}</span>
                {tier !== "free" && <ProBadge tier={tier} badgeStyle="tint" size="sm" />}
              </div>
              <div className="plan-tile-price">{t(`page.plans.${tier}.price`)}</div>
              <ul className="plan-tile-features">
                {features.map((feature) => (
                  <li key={feature}>
                    {t(`page.plans.${tier}.${feature}`, {
                      minutes: VOICE_MESSAGE_MAX_MS_BY_TIER[tier] / 60000,
                      size: VIDEO_NOTE_SIZE_BY_TIER[tier],
                    })}
                  </li>
                ))}
              </ul>
              <div className="plan-tile-foot">
                {tier === current
                  ? t("page.current")
                  : tier === "premium"
                    ? t("page.soon")
                    : tier === "pro"
                      ? t("page.paymentLater")
                      : ""}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="s-section">
        <h2>{t("page.billing.title")}</h2>
        <SettingsRow label={t("page.billing.paymentMethod")} hint={t("page.billing.paymentMethodHint")}>
          <button className="s-btn secondary" disabled>{t("page.billing.addCard")}</button>
        </SettingsRow>
        <SettingsRow label={t("page.billing.history")} hint={t("page.billing.historyHint")}>
          <button className="s-btn secondary" disabled>{t("page.billing.view")}</button>
        </SettingsRow>
      </div>
    </>
  );
}
