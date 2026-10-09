import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { effectiveBinding, useActiveBindings, useHotkey } from "../../settings/lib/hotkeys";
import { IS_MAC, bindingKeycaps } from "../../settings/lib/hotkeyMatching";
import AccountMenu from "./AccountMenu";

interface AppTopBarProps {
  navState: { back: boolean; forward: boolean };
  goHistory: (delta: -1 | 1) => void;
  homeActive: boolean;
  onGoHome: () => void;
  query: string;
  onQueryChange: (value: string) => void;
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  onOpenSettings: () => void;
  onSetStatus?: (status: "online" | "offline") => void;
}

const AppTopBar: React.FC<AppTopBarProps> = ({
  navState,
  goHistory,
  homeActive,
  onGoHome,
  query,
  onQueryChange,
  currentUserId,
  currentUserName,
  currentUserStatus,
  onOpenSettings,
  onSetStatus,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const searchRef = React.useRef<HTMLInputElement>(null);
  const switcherBinding = effectiveBinding(useActiveBindings(), "quickSwitcher");
  const switcherLabel = switcherBinding ? bindingKeycaps(switcherBinding).join(IS_MAC ? "" : "+") : null;
  useHotkey("quickSwitcher", () => {
    searchRef.current?.focus();
    searchRef.current?.select();
  });

  return (
    <header className="topbar">
      <div className="topbar-side topbar-start">
        <button
          type="button"
          className="topbar-nav"
          title={t("common:back")}
          aria-label={t("common:back")}
          disabled={!navState.back}
          onClick={() => goHistory(-1)}
        >
          <Icon name="chev-left" size={16} />
        </button>
        <button
          type="button"
          className="topbar-nav"
          title={t("common:forward")}
          aria-label={t("common:forward")}
          disabled={!navState.forward}
          onClick={() => goHistory(1)}
        >
          <Icon name="chev-left" size={16} className="flip-x" />
        </button>
      </div>

      <div className="topbar-center">
        <button
          type="button"
          className={"topbar-home" + (homeActive ? " active" : "")}
          title={t("home")}
          aria-label={t("home")}
          aria-current={homeActive ? "page" : undefined}
          onClick={onGoHome}
        >
          <Icon name="home" size={20} />
        </button>
        <div className="topbar-search">
          <Icon name="search" size={18} />
          <input
            ref={searchRef}
            placeholder={t("searchPlaceholder")}
            aria-label={t("searchPlaceholder")}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                onQueryChange("");
                searchRef.current?.blur();
              }
            }}
          />
          {query ? (
            <button
              type="button"
              tabIndex={-1}
              className="topbar-search-clear"
              title={t("common:close")}
              aria-label={t("common:close")}
              onClick={() => {
                onQueryChange("");
                searchRef.current?.focus();
              }}
            >
              <Icon name="x" size={12} />
            </button>
          ) : (
            switcherLabel && <kbd>{switcherLabel}</kbd>
          )}
        </div>
      </div>

      <div className="topbar-side topbar-end">
        <AccountMenu
          currentUserId={currentUserId}
          currentUserName={currentUserName}
          currentUserStatus={currentUserStatus}
          onOpenSettings={onOpenSettings}
          onSetStatus={onSetStatus}
        />
      </div>
    </header>
  );
};

export default AppTopBar;
