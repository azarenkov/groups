import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import UserBadge from "../../../subscription/components/UserBadge";

interface SidebarFooterProps {
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  onOpenSettings: () => void;
  onSetStatus?: (status: "online" | "offline") => void;
}

const SidebarFooter: React.FC<SidebarFooterProps> = ({
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
      if (
        statusMenuRef.current &&
        !statusMenuRef.current.contains(e.target as Node)
      ) {
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

  return (
    <div className="sb-footer">
      <div className="user-pill">
        <UserAvatar
          userId={currentUserId}
          name={currentUserName}
          className="ava"
        >
          <span
            className={
              "dot" + (currentUserStatus === "offline" ? " offline" : "")
            }
          />
        </UserAvatar>
        <div
          className={"info user-pill-info-btn" + (statusMenuOpen ? " open" : "")}
          ref={statusMenuRef}
          role="button"
          tabIndex={0}
          title={t("changeStatus")}
          onClick={() => setStatusMenuOpen((v) => !v)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              setStatusMenuOpen((v) => !v);
            }
          }}
        >
          <div className="n">
            <span className="n-text">{currentUserName}</span>
            <UserBadge userId={currentUserId} size="sm" />
          </div>
          <div className="s">
            <span>
              {currentUserStatus === "offline"
                ? t("statusOffline")
                : t("statusOnline")}
            </span>
            <Icon
              name="chev"
              size={9}
              className={"user-pill-chev" + (statusMenuOpen ? " up" : "")}
            />
          </div>
          {statusMenuOpen && onSetStatus && (
            <div
              className="sb-status-menu"
              onClick={(e) => e.stopPropagation()}
            >
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
            </div>
          )}
        </div>
        <button
          type="button"
          className="icon-btn"
          title={t("settings")}
          onClick={onOpenSettings}
        >
          <Icon name="settings" size={14} />
        </button>
      </div>
    </div>
  );
};

export default SidebarFooter;
