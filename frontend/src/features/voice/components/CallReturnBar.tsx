import React from "react";
import i18n from "../../../i18n";
import { useCallStore, type CallNotice } from "../store/callStore";

function useAutoDismiss<T>(value: T | null, clear: (value: null) => void, ms: number) {
  React.useEffect(() => {
    if (!value) return;
    const timer = window.setTimeout(() => clear(null), ms);
    return () => window.clearTimeout(timer);
  }, [value, clear, ms]);
}

const noticeText = (notice: CallNotice): string => {
  if (notice.kind === "missed") {
    return notice.name ? i18n.t("missedFrom", { ns: "call", name: notice.name }) : i18n.t("callMissed", { ns: "call" });
  }
  return notice.name ? i18n.t("declinedBy", { ns: "call", name: notice.name }) : i18n.t("declinedByUnknown", { ns: "call" });
};

export const CallToast: React.FC<{ onOpenDm: (peerId: string) => void }> = ({ onOpenDm }) => {
  const error = useCallStore((s) => s.callError);
  const notice = useCallStore((s) => s.callNotice);
  const setCallError = useCallStore((s) => s.setCallError);
  const setCallNotice = useCallStore((s) => s.setCallNotice);
  useAutoDismiss(error, setCallError, 4000);
  useAutoDismiss(notice, setCallNotice, 6000);
  if (error) {
    return (
      <div className="call-error-toast" role="alert">
        {error}
      </div>
    );
  }
  if (!notice) return null;
  if (notice.kind === "missed") {
    return (
      <button
        type="button"
        className="call-error-toast call-notice-toast"
        role="status"
        onClick={() => {
          setCallNotice(null);
          onOpenDm(notice.userId);
        }}
      >
        {noticeText(notice)}
      </button>
    );
  }
  return (
    <div className="call-error-toast call-notice-toast" role="status">
      {noticeText(notice)}
    </div>
  );
};

