import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import ConfirmModal from "../../../shared/ui/ConfirmModal";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { useProfileModalStore } from "../../social/store/profileModalStore";

interface AccountMenuProps {
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  onOpenSettings: () => void;
  onSetStatus?: (status: "online" | "offline") => void;
  onSignOut?: () => void;
}

const AccountMenu: React.FC<AccountMenuProps> = ({
  currentUserId,
  currentUserName,
  currentUserStatus,
  onOpenSettings,
  onSetStatus,
  onSignOut,
}) => {
  const { t } = useTranslation(["sidebar", "settings"]);
  const [open, setOpen] = React.useState(false);
  const [confirmLogout, setConfirmLogout] = React.useState(false);
  const menuRef = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const offline = currentUserStatus === "offline";
  const run = (action: () => void) => () => {
    setOpen(false);
    action();
  };

  return (
    <div className="account-menu">
      <div className="account-me" ref={menuRef}>
        <button
          type="button"
          className={"account-avatar" + (open ? " on" : "")}
          title={currentUserName}
          aria-label={`${currentUserName}, ${t("accountMenu")}`}
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={() => setOpen((v) => !v)}
        >
          <UserAvatar userId={currentUserId} name={currentUserName} className="account-ava">
            <span className={"dot" + (offline ? " offline" : "")} />
          </UserAvatar>
        </button>
        {open && (
          <div className="account-dropdown" role="menu" aria-label={t("accountMenu")}>
            <div className="account-dropdown-name">{currentUserName}</div>
            {currentUserId && (
              <button
                type="button"
                role="menuitem"
                className="account-dropdown-item"
                onClick={run(() => useProfileModalStore.getState().open(currentUserId))}
              >
                {t("myProfile")}
              </button>
            )}
            <button type="button" role="menuitem" className="account-dropdown-item" onClick={run(onOpenSettings)}>
              {t("settings")}
            </button>
            {onSetStatus && (
              <>
                <div className="account-dropdown-sep" />
                <div className="account-dropdown-label">{t("status")}</div>
                {(["online", "offline"] as const).map((status) => (
                  <button
                    key={status}
                    type="button"
                    role="menuitemradio"
                    aria-checked={(status === "offline") === offline}
                    className="account-dropdown-item"
                    onClick={run(() => onSetStatus(status))}
                  >
                    <span className={"sb-status-dot " + status} />
                    <span className="account-dropdown-text">
                      {t(status === "online" ? "statusOnline" : "statusOffline")}
                    </span>
                    {(status === "offline") === offline && <Icon name="check" size={16} />}
                  </button>
                ))}
              </>
            )}
            {onSignOut && (
              <>
                <div className="account-dropdown-sep" />
                <button
                  type="button"
                  role="menuitem"
                  className="account-dropdown-item"
                  onClick={run(() => setConfirmLogout(true))}
                >
                  {t("logOut")}
                </button>
              </>
            )}
          </div>
        )}
      </div>
      {confirmLogout && onSignOut && (
        <ConfirmModal
          title={t("settings:logOutConfirmTitle")}
          subtitle={t("settings:logOutConfirmSubtitle")}
          confirmLabel={t("logOut")}
          danger
          onCancel={() => setConfirmLogout(false)}
          onConfirm={() => {
            setConfirmLogout(false);
            onSignOut();
          }}
        />
      )}
    </div>
  );
};

export default AccountMenu;
