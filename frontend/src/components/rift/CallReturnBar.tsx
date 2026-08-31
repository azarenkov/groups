import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useCallStore } from "../../store/callStore";

const CallReturnBar: React.FC = () => {
  const { t } = useTranslation(["call", "voice"]);
  const activeCallId = useCallStore((s) => s.activeCallId);
  const focused = useCallStore((s) => s.focused);
  const participants = useCallStore((s) => s.participants);
  const setFocused = useCallStore((s) => s.setFocused);
  const leave = useCallStore((s) => s.leave);

  if (!activeCallId || focused) return null;

  const count = Math.max(Object.keys(participants).length, 1);

  return (
    <button
      type="button"
      className="call-return-bar"
      onClick={() => setFocused(true)}
      title={t("openCall")}
    >
      <span className="crb-dot" />
      <Icon name="phone" size={15} />
      <span className="crb-label">{t("inCall")}</span>
      <span className="crb-meta">
        {count} {t("voice:connected").toLowerCase()}
      </span>
      <span className="crb-cta">{t("returnToCall")}</span>
      <span
        role="button"
        tabIndex={-1}
        className="crb-leave"
        title={t("leave")}
        onClick={(e) => {
          e.stopPropagation();
          leave();
        }}
      >
        <Icon name="logout" size={15} />
      </span>
    </button>
  );
};

export const CallErrorToast: React.FC = () => {
  const error = useCallStore((s) => s.callError);
  const setCallError = useCallStore((s) => s.setCallError);
  React.useEffect(() => {
    if (!error) return;
    const timer = window.setTimeout(() => setCallError(null), 4000);
    return () => window.clearTimeout(timer);
  }, [error, setCallError]);
  if (!error) return null;
  return (
    <div className="call-error-toast" role="alert">
      {error}
    </div>
  );
};

export default CallReturnBar;
