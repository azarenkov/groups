import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";

interface SidebarHeaderProps {
  tab: "dms" | "communities";
  onSetTab: (tab: "dms" | "communities") => void;
  openCommunityId: string | null;
  onCloseCommunity: () => void;
  dmDotVisible: boolean;
  communityDotVisible: boolean;
  onCollapse: () => void;
}

const SidebarHeader: React.FC<SidebarHeaderProps> = ({
  tab,
  onSetTab,
  openCommunityId,
  onCloseCommunity,
  dmDotVisible,
  communityDotVisible,
  onCollapse,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  return (
    <div className="sb-head">
      <div className="sb-head-top">
        <span className="sb-head-title">{t("chats")}</span>
        <button
          type="button"
          className="sb-rail-btn sb-head-collapse"
          title={t("collapseSidebar")}
          aria-label={t("collapseSidebar")}
          onClick={onCollapse}
        >
          <Icon name="shrink" size={15} />
        </button>
      </div>
      <div className={"sb-seg " + tab}>
        <div className="sb-seg-thumb" />
        <button
          type="button"
          className={"sb-seg-btn" + (tab === "dms" ? " active" : "")}
          onClick={() => onSetTab("dms")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z" />
          </svg>
          {t("tabDirects")}
          {dmDotVisible && <span className="sb-seg-dot" />}
        </button>
        <button
          type="button"
          className={"sb-seg-btn" + (tab === "communities" ? " active" : "")}
          onClick={() =>
            tab === "communities" && openCommunityId ? onCloseCommunity() : onSetTab("communities")
          }
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
          {t("tabCommunities")}
          {communityDotVisible && <span className="sb-seg-dot" />}
        </button>
      </div>
    </div>
  );
};

export default SidebarHeader;
