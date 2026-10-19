import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";

interface SidebarHeaderProps {
  tab: "dms" | "communities";
  onAddFriend: () => void;
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  onCollapse: () => void;
  canCreateGroups?: boolean;
}

const SidebarHeader: React.FC<SidebarHeaderProps> = ({
  tab,
  onAddFriend,
  onCreateCommunity,
  onJoinCommunity,
  onCollapse,
  canCreateGroups = true,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  return (
    <div className="sb-head">
      <div className="sb-head-top">
        <span className="sb-head-title">{tab === "dms" ? t("tabDirects") : t("tabCommunities")}</span>
        <div className="sb-head-actions">
          {tab === "dms" ? (
            <button
              type="button"
              className="sb-rail-btn"
              title={t("addFriend")}
              aria-label={t("addFriend")}
              onClick={onAddFriend}
            >
              <Icon name="user-plus" size={18} />
            </button>
          ) : (
            <>
              <button
                type="button"
                className="sb-rail-btn"
                title={t("joinCommunity")}
                aria-label={t("joinCommunity")}
                onClick={onJoinCommunity}
              >
                <Icon name="link" size={18} />
              </button>
              {canCreateGroups && (
                <button
                  type="button"
                  className="sb-rail-btn"
                  title={t("createCommunity")}
                  aria-label={t("createCommunity")}
                  onClick={onCreateCommunity}
                >
                  <Icon name="plus" size={18} />
                </button>
              )}
            </>
          )}
          <button
            type="button"
            className="sb-rail-btn sb-head-collapse"
            title={t("collapseSidebar")}
            aria-label={t("collapseSidebar")}
            onClick={onCollapse}
          >
            <Icon name="panel-close" size={20} />
          </button>
        </div>
      </div>
    </div>
  );
};

export default SidebarHeader;
