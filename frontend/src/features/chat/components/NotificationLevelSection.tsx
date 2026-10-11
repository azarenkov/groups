import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { useNotificationSettingsStore } from "../../settings/store/notificationSettingsStore";
import { dmLevelOf, useNotificationPrefsStore } from "../../settings/store/notificationPrefsStore";

type Level = "all" | "mentions" | "none";

interface NotificationLevelSectionProps {
  dmId?: string;
  channelId?: string;
}

const NotificationLevelSection: React.FC<NotificationLevelSectionProps> = ({ dmId, channelId }) => {
  const { t } = useTranslation("chat");
  const globalMentions = useNotificationSettingsStore((s) => s.mentions);
  const level = useNotificationPrefsStore((s) =>
    dmId ? dmLevelOf(s.dmLevels, dmId, globalMentions) : channelId ? (s.channelLevels[channelId] ?? globalMentions) : "all",
  );

  const options: { k: Level; ttl: string; sub: string }[] = dmId
    ? [
        { k: "all", ttl: t("notifications.all"), sub: t("notifications.allHint") },
        { k: "none", ttl: t("notifications.nothing"), sub: t("notifications.dmNothingHint") },
      ]
    : [
        { k: "all", ttl: t("notifications.all"), sub: t("notifications.allHint") },
        { k: "mentions", ttl: t("notifications.mentions"), sub: t("notifications.mentionsHint") },
        { k: "none", ttl: t("notifications.nothing"), sub: t("notifications.nothingHint") },
      ];

  const apply = (next: Level) => {
    const prefs = useNotificationPrefsStore.getState();
    const action = dmId
      ? next !== "mentions" && prefs.setDmLevel(dmId, next)
      : channelId
        ? prefs.setChannelLevel(channelId, next)
        : null;
    if (action) action.catch((e) => console.warn("[notifications] failed to save preference:", e));
  };

  return (
    <section className="details-section" aria-label={t("notifications.bell")}>
      <div className="details-label">{t("notifications.bell")}</div>
      <div className="details-options" role="radiogroup" aria-label={t("notifications.bell")}>
        {options.map((opt) => (
          <button
            key={opt.k}
            type="button"
            role="radio"
            aria-checked={level === opt.k}
            className={"details-option" + (level === opt.k ? " on" : "")}
            onClick={() => apply(opt.k)}
          >
            <span className="details-option-check">{level === opt.k && <Icon name="check" size={14} />}</span>
            <span className="details-option-body">
              <span className="details-option-title">{opt.ttl}</span>
              <span className="details-option-sub">{opt.sub}</span>
            </span>
          </button>
        ))}
      </div>
    </section>
  );
};

export default NotificationLevelSection;
