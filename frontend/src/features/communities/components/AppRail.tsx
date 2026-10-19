import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";

interface AppRailProps {
  tab: "dms" | "communities";
  homeActive: boolean;
  dmUnread: number;
  groupUnread: number;
  onGoHome: () => void;
  onSetTab: (tab: "dms" | "communities") => void;
  onOpenSettings: () => void;
}

const AppRail: React.FC<AppRailProps> = ({
  tab,
  homeActive,
  dmUnread,
  groupUnread,
  onGoHome,
  onSetTab,
  onOpenSettings,
}) => {
  const { t } = useTranslation(["sidebar"]);
  const chatActive = !homeActive && tab === "dms";
  const groupsActive = !homeActive && tab === "communities";
  const badge = (count: number) =>
    count > 0 ? <span className="app-rail-badge">{count > 99 ? "99+" : count}</span> : null;
  return (
    <nav className="app-rail" aria-label={t("appNavigation")}>
      <div className="app-rail-top">
        <button
          type="button"
          className={"app-rail-btn" + (homeActive ? " active" : "")}
          aria-current={homeActive ? "page" : undefined}
          onClick={onGoHome}
        >
          <span className="app-rail-ico">
            <Icon name="home" size={22} />
          </span>
          <span className="app-rail-label">{t("home")}</span>
        </button>
        <button
          type="button"
          className={"app-rail-btn" + (chatActive ? " active" : "")}
          aria-current={chatActive ? "page" : undefined}
          onClick={() => onSetTab("dms")}
        >
          <span className="app-rail-ico">
            <Icon name="message" size={22} />
            {badge(dmUnread)}
          </span>
          <span className="app-rail-label">{t("tabDirects")}</span>
        </button>
        <button
          type="button"
          className={"app-rail-btn" + (groupsActive ? " active" : "")}
          aria-current={groupsActive ? "page" : undefined}
          onClick={() => onSetTab("communities")}
        >
          <span className="app-rail-ico">
            <Icon name="users" size={22} />
            {badge(groupUnread)}
          </span>
          <span className="app-rail-label">{t("tabCommunities")}</span>
        </button>
      </div>
      <div className="app-rail-bottom">
        <button type="button" className="app-rail-btn" onClick={onOpenSettings}>
          <span className="app-rail-ico">
            <Icon name="settings" size={22} />
          </span>
          <span className="app-rail-label">{t("settings")}</span>
        </button>
      </div>
    </nav>
  );
};

export default AppRail;
