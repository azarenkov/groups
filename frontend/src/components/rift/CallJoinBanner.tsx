import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { callAPI } from "../../api/call";
import { useCallStore } from "../../store/callStore";

const CallJoinBanner: React.FC<{ friendId: string }> = ({ friendId }) => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const joinCall = useCallStore((s) => s.joinCall);
  const [callId, setCallId] = useState<string | null>(null);
  const { t } = useTranslation("call");

  useEffect(() => {
    let cancelled = false;
    const check = () => {
      callAPI
        .activeCallWith(friendId)
        .then((id) => {
          if (!cancelled) setCallId(id);
        })
        .catch(() => {});
    };
    check();
    const iv = window.setInterval(check, 12000);
    return () => {
      cancelled = true;
      window.clearInterval(iv);
    };
  }, [friendId]);

  if (!callId || activeCallId === callId) return null;

  return (
    <button
      type="button"
      className="call-join-banner"
      onClick={() =>
        joinCall(callId, friendId).catch((e) =>
          console.warn("[call] join failed:", e),
        )
      }
    >
      <span className="cjb-dot" />
      <Icon name="phone" size={15} />
      <span className="cjb-label">{t("ongoingCall")}</span>
      <span className="cjb-cta">{t("joinCall")}</span>
    </button>
  );
};

export default CallJoinBanner;
