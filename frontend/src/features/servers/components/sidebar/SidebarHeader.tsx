import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { useHotkey } from "../../../settings/lib/hotkeys";

interface SidebarHeaderProps {
  homeActive: boolean;
  onGoHome: () => void;
  navState: { back: boolean; forward: boolean };
  goHistory: (delta: -1 | 1) => void;
  query: string;
  onQueryChange: (value: string) => void;
  tab: "dms" | "servers";
  onSetTab: (tab: "dms" | "servers") => void;
  openServerId: string | null;
  onCloseServer: () => void;
  dmDotVisible: boolean;
  serverDotVisible: boolean;
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
  openServerId,
  onCloseServer,
  dmDotVisible,
  serverDotVisible,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const searchInputRef = React.useRef<HTMLInputElement>(null);
  useHotkey("quickSwitcher", () => {
    searchInputRef.current?.focus();
    searchInputRef.current?.select();
  });
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
          <kbd>⌘K</kbd>
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
          className={"sb-seg-btn" + (tab === "servers" ? " active" : "")}
          onClick={() =>
            tab === "servers" && openServerId ? onCloseServer() : onSetTab("servers")
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
            <rect x="2" y="2" width="20" height="8" rx="2" />
            <rect x="2" y="14" width="20" height="8" rx="2" />
            <path d="M6 6h.01M6 18h.01" />
          </svg>
          {t("tabServers")}
          {serverDotVisible && <span className="sb-seg-dot" />}
        </button>
      </div>
    </>
  );
};

export default SidebarHeader;
