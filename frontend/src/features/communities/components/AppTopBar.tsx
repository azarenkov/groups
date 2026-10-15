import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { effectiveBinding, useActiveBindings, useHotkey } from "../../settings/lib/hotkeys";
import { IS_MAC, bindingKeycaps } from "../../settings/lib/hotkeyMatching";
import AccountMenu from "./AccountMenu";
import { useSearchFocusStore, type SearchFocusMode } from "../../search/store/searchFocusStore";

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
  onSignOut?: () => void;
  renderResults?: (done: () => void) => React.ReactNode;
  inbox?: React.ReactNode;
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
  onSignOut,
  renderResults,
  inbox,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const searchRef = React.useRef<HTMLInputElement>(null);
  const switcherBinding = effectiveBinding(useActiveBindings(), "quickSwitcher");
  const switcherLabel = switcherBinding ? bindingKeycaps(switcherBinding).join(IS_MAC ? "" : "+") : null;
  const wrapRef = React.useRef<HTMLDivElement>(null);
  const [open, setOpen] = React.useState(false);
  useHotkey("quickSwitcher", () => {
    searchRef.current?.focus();
    searchRef.current?.select();
  });
  const focusRequest = useSearchFocusStore((s) => s.request);
  const handledFocusSeq = React.useRef(focusRequest?.seq ?? 0);
  const [focusMode, setFocusMode] = React.useState<SearchFocusMode | null>(null);
  React.useEffect(() => {
    if (!focusRequest || focusRequest.seq === handledFocusSeq.current) return;
    handledFocusSeq.current = focusRequest.seq;
    setFocusMode(focusRequest.mode);
    setOpen(true);
    searchRef.current?.focus();
    searchRef.current?.select();
  }, [focusRequest]);
  const placeholder = focusMode === "people" ? t("searchPlaceholderPeople") : t("searchPlaceholder");

  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!wrapRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const items = () =>
    Array.from(wrapRef.current?.querySelectorAll<HTMLButtonElement>(".search-dd-list button:not(:disabled):not([data-search-manage])") ?? []);
  const moveFocus = (from: Element | null, delta: 1 | -1) => {
    const list = items();
    if (list.length === 0) return;
    const index = from ? list.indexOf(from as HTMLButtonElement) : -1;
    const next = index + delta;
    if (next < 0) searchRef.current?.focus();
    else list[Math.min(next, list.length - 1)].focus();
  };
  const done = () => {
    setOpen(false);
    setFocusMode(null);
    onQueryChange("");
    searchRef.current?.blur();
  };
  const showResults = open && !!renderResults;

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
        <div
          className="topbar-search-wrap"
          ref={wrapRef}
          onKeyDown={(e) => {
            if (e.target === searchRef.current) return;
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              moveFocus(document.activeElement, e.key === "ArrowDown" ? 1 : -1);
            } else if (e.key === "Escape") {
              e.preventDefault();
              done();
            }
          }}
        >
          <div
            className={"topbar-search" + (showResults ? " open" : "")}
            onMouseDown={(e) => {
              const target = e.target as HTMLElement;
              if (target.closest("input, button")) return;
              e.preventDefault();
              e.currentTarget.querySelector("input")?.focus();
            }}
          >
            <Icon name="search" size={18} />
            <input
              ref={searchRef}
              placeholder={placeholder}
              aria-label={placeholder}
              value={query}
              onChange={(e) => {
                onQueryChange(e.target.value);
                setOpen(true);
              }}
              onFocus={() => setOpen(true)}
              onBlur={() => {
                if (!query) setFocusMode(null);
              }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  done();
                } else if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setOpen(true);
                  moveFocus(null, 1);
                } else if (e.key === "Enter") {
                  const first = items()[0];
                  if (first) {
                    e.preventDefault();
                    first.click();
                  }
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
          {showResults && renderResults?.(done)}
        </div>
      </div>

      <div className="topbar-side topbar-end">
        {inbox}
        <AccountMenu
          currentUserId={currentUserId}
          currentUserName={currentUserName}
          currentUserStatus={currentUserStatus}
          onOpenSettings={onOpenSettings}
          onSetStatus={onSetStatus}
          onSignOut={onSignOut}
        />
      </div>
    </header>
  );
};

export default AppTopBar;
