import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";

interface AppRailProps {
  homeActive: boolean;
  groupUnread: number;
  onGoHome: () => void;
  onOpenGroups: () => void;
  onOpenSettings: () => void;
}

const AppRail: React.FC<AppRailProps> = ({ homeActive, groupUnread, onGoHome, onOpenGroups, onOpenSettings }) => {
  const { t } = useTranslation(["sidebar"]);
  const groupsActive = !homeActive;
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
          className={"app-rail-btn" + (groupsActive ? " active" : "")}
          aria-current={groupsActive ? "page" : undefined}
          onClick={onOpenGroups}
        >
          <span className="app-rail-ico">
            <Icon name="users" size={22} />
            {groupUnread > 0 && <span className="app-rail-badge">{groupUnread > 99 ? "99+" : groupUnread}</span>}
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
