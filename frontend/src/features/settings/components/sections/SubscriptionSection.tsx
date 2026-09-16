import { useTranslation } from "react-i18next";

import { SettingsRow } from "../controls";

export function SubscriptionSection() {
  const { t } = useTranslation("settings");
  return (
    <>
      <h1>{t("subscription.title")}</h1>
      <p className="lead">{t("subscription.lead")}</p>

      <div className="s-section">
        <h2>{t("subscription.currentPlan")}</h2>
        <div className="plan-card current">
          <div className="plan-info">
            <div className="plan-name">{t("subscription.free")}</div>
            <div className="plan-price">{t("subscription.noPaymentMethod")}</div>
          </div>
          <span className="plan-badge">{t("subscription.current")}</span>
        </div>
        <div className="plan-card">
          <div className="plan-info">
            <div className="plan-name">{t("subscription.premium")}</div>
            <div className="plan-price">$8/month</div>
          </div>
          <button className="s-btn primary">{t("subscription.upgrade")}</button>
        </div>
        <div className="plan-card">
          <div className="plan-info">
            <div className="plan-name">Rift Studio</div>
            <div className="plan-price">$16/month · Everything in Pro plus screen-share recording, AI summaries</div>
          </div>
          <button className="s-btn secondary">See features</button>
        </div>
      </div>

      <div className="s-section">
        <h2>Billing</h2>
        <SettingsRow label="Payment method" hint="Add a card to upgrade.">
          <button className="s-btn secondary">Add card</button>
        </SettingsRow>
        <SettingsRow label="Billing history" hint="Download past invoices.">
          <button className="s-btn secondary">View</button>
        </SettingsRow>
      </div>
    </>
  );
}
