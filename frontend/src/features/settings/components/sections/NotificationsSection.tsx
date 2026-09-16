import { useTranslation } from "react-i18next";
import { useNotificationSettingsStore } from "../../store/notificationSettingsStore";
import { requestDesktopNotificationPermission } from "../../../../shared/lib/desktopNotification";

import { SettingsRow, Switch } from "../controls";

export function NotificationsSection() {
  const { t } = useTranslation("settings");
  const desktopEnabled = useNotificationSettingsStore((s) => s.desktopEnabled);
  const soundsEnabled = useNotificationSettingsStore((s) => s.soundsEnabled);
  const mentions = useNotificationSettingsStore((s) => s.mentions);
  const quietHoursEnabled = useNotificationSettingsStore((s) => s.quietHoursEnabled);
  const quietStart = useNotificationSettingsStore((s) => s.quietStart);
  const quietEnd = useNotificationSettingsStore((s) => s.quietEnd);
  const callsEnabled = useNotificationSettingsStore((s) => s.callsEnabled);
  const update = useNotificationSettingsStore((s) => s.update);

  const toggleDesktop = () => {
    const next = !desktopEnabled;
    update({ desktopEnabled: next });
    if (next) requestDesktopNotificationPermission();
  };

  return (
    <>
      <h1>{t("notifications.title")}</h1>
      <p className="lead">{t("notifications.lead")}</p>

      <div className="s-section">
        <h2>{t("notifications.desktop")}</h2>
        <SettingsRow label={t("notifications.enableDesktop")} hint={t("notifications.enableDesktopHint")}>
          <Switch on={desktopEnabled} onClick={toggleDesktop}/>
        </SettingsRow>
        <SettingsRow label={t("notifications.enableSounds")} hint={t("notifications.enableSoundsHint")}>
          <Switch on={soundsEnabled} onClick={() => update({ soundsEnabled: !soundsEnabled })}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("notifications.mentions")}</h2>
        <SettingsRow label={t("notifications.mentions")} hint={t("notifications.mentionsHint")}>
          <div className="s-seg">
            <button className={mentions === "all" ? "on" : ""} onClick={() => update({ mentions: "all" })}>{t("notifications.mentionsAll")}</button>
            <button className={mentions === "mentions" ? "on" : ""} onClick={() => update({ mentions: "mentions" })}>{t("notifications.mentionsMentions")}</button>
            <button className={mentions === "none" ? "on" : ""} onClick={() => update({ mentions: "none" })}>{t("notifications.mentionsNothing")}</button>
          </div>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("notifications.calls")}</h2>
        <SettingsRow label={t("notifications.callAlerts")} hint={t("notifications.callAlertsHint")}>
          <Switch on={callsEnabled} onClick={() => update({ callsEnabled: !callsEnabled })}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("notifications.quietTitle")}</h2>
        <SettingsRow label={t("notifications.quietEnable")} hint={t("notifications.quietEnableHint")}>
          <Switch on={quietHoursEnabled} onClick={() => update({ quietHoursEnabled: !quietHoursEnabled })}/>
        </SettingsRow>
        {quietHoursEnabled && (
          <SettingsRow label={t("notifications.schedule")} hint={t("notifications.scheduleHint")}>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input
                type="time"
                className="s-input"
                style={{ width: 110 }}
                value={quietStart}
                onChange={(e) => {
                  if (e.target.value) update({ quietStart: e.target.value });
                }}
              />
              <span style={{ color: "var(--fg-4)" }}>→</span>
              <input
                type="time"
                className="s-input"
                style={{ width: 110 }}
                value={quietEnd}
                onChange={(e) => {
                  if (e.target.value) update({ quietEnd: e.target.value });
                }}
              />
            </div>
          </SettingsRow>
        )}
      </div>
    </>
  );
}
