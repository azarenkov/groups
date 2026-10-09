import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { effectiveBinding, useActiveBindings } from "../../../settings/lib/hotkeys";
import { IS_MAC, bindingKeycaps } from "../../../settings/lib/hotkeyMatching";

interface SidebarHeaderProps {
  homeActive: boolean;
  onGoHome: () => void;
  navState: { back: boolean; forward: boolean };
  goHistory: (delta: -1 | 1) => void;
  query: string;
  onQueryChange: (value: string) => void;
  tab: "dms" | "communities";
  onSetTab: (tab: "dms" | "communities") => void;
  openCommunityId: string | null;
  onCloseCommunity: () => void;
  dmDotVisible: boolean;
  communityDotVisible: boolean;
  onCollapse: () => void;
  focusSearchToken: number;
}

const SidebarHeader: React.FC<SidebarHeaderProps> = ({
  homeActive,
  onGoHome,
  navState,
  goHistory,
  query,
  onQueryChange,
  tab,
  onSetTab,
  openCommunityId,
  onCloseCommunity,
  dmDotVisible,
  communityDotVisible,
  onCollapse,
  focusSearchToken,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const searchInputRef = React.useRef<HTMLInputElement>(null);
  const switcherBinding = effectiveBinding(useActiveBindings(), "quickSwitcher");
  const switcherLabel = switcherBinding ? bindingKeycaps(switcherBinding).join(IS_MAC ? "" : "+") : null;
  React.useEffect(() => {
    if (focusSearchToken === 0) return;
    searchInputRef.current?.focus();
    searchInputRef.current?.select();
  }, [focusSearchToken]);
  return (
    <>
      <div className="sb-nav">
        <button
          type="button"
          className={"sb-nav-btn sb-nav-home" + (homeActive ? " active" : "")}
          title={t("home")}
          onClick={onGoHome}
        >
          <Icon name="home" size={15} />
        </button>
        <button
          type="button"
          className="sb-nav-btn"
          title={t("common:back")}
          disabled={!navState.back}
          onClick={() => goHistory(-1)}
        >
          <Icon name="chev-left" size={15} />
        </button>
        <button
          type="button"
          className="sb-nav-btn"
          title={t("common:forward")}
          disabled={!navState.forward}
          onClick={() => goHistory(1)}
        >
          <Icon name="chev-left" size={15} className="flip-x" />
        </button>
        <button
          type="button"
          className="sb-nav-btn sb-nav-collapse"
          title={t("collapseSidebar")}
          aria-label={t("collapseSidebar")}
          onClick={onCollapse}
        >
          <Icon name="shrink" size={14} />
        </button>
      </div>
      <div className="sb-search">
        <Icon name="search" size={14} />
        <input
          ref={searchInputRef}
          placeholder={t("searchPlaceholder")}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              onQueryChange("");
              searchInputRef.current?.blur();
            }
          }}
        />
        {query ? (
          <button
            type="button"
            tabIndex={-1}
            className="sb-search-clear"
            title={t("close", { ns: "common" })}
            onClick={() => {
              onQueryChange("");
              searchInputRef.current?.focus();
            }}
          >
            <Icon name="x" size={11} />
          </button>
        ) : (
          switcherLabel && <kbd>{switcherLabel}</kbd>
        )}
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
    </>
  );
};

export default SidebarHeader;
