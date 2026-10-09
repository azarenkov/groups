import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import PlanUntil from "../../subscription/components/PlanUntil";

interface AccountMenuProps {
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  onOpenSettings: () => void;
  onSetStatus?: (status: "online" | "offline") => void;
}

const AccountMenu: React.FC<AccountMenuProps> = ({
  currentUserId,
  currentUserName,
  currentUserStatus,
  onOpenSettings,
  onSetStatus,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const [statusMenuOpen, setStatusMenuOpen] = React.useState(false);
  const statusMenuRef = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    if (!statusMenuOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (statusMenuRef.current && !statusMenuRef.current.contains(e.target as Node)) {
        setStatusMenuOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setStatusMenuOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [statusMenuOpen]);

  const offline = currentUserStatus === "offline";

  return (
    <div className="account-menu">
      <button type="button" className="icon-btn account-settings" title={t("settings")} aria-label={t("settings")} onClick={onOpenSettings}>
        <Icon name="settings" size={17} />
      </button>
      <div className="account-me" ref={statusMenuRef}>
        <button
          type="button"
          className="account-avatar"
          title={t("changeStatus")}
          aria-label={`${currentUserName}, ${t("changeStatus")}`}
          aria-expanded={statusMenuOpen}
          onClick={() => setStatusMenuOpen((v) => !v)}
        >
          <UserAvatar userId={currentUserId} name={currentUserName} className="account-ava">
            <span className={"dot" + (offline ? " offline" : "")} />
          </UserAvatar>
        </button>
        {statusMenuOpen && onSetStatus && (
          <div className="sb-status-menu account-status-menu" onClick={(e) => e.stopPropagation()}>
            <div className="account-status-name">{currentUserName}</div>
            <button
              type="button"
              className="sb-status-menu-item"
              onClick={() => {
                onSetStatus("online");
                setStatusMenuOpen(false);
              }}
            >
              <span className="sb-status-dot online" />
              <span>{t("statusOnline")}</span>
            </button>
            <button
              type="button"
              className="sb-status-menu-item"
              onClick={() => {
                onSetStatus("offline");
                setStatusMenuOpen(false);
              }}
            >
              <span className="sb-status-dot offline" />
              <span>{t("statusOffline")}</span>
            </button>
            <PlanUntil className="sb-status-menu-plan" />
          </div>
        )}
      </div>
    </div>
  );
};

export default AccountMenu;
