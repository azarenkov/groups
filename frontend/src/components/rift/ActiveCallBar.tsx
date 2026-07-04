import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import VoicePill from "./VoicePill";
import { useCallStore } from "../../store/callStore";

const ActiveCallBar: React.FC = () => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const leave = useCallStore((s) => s.leave);
  const setFocused = useCallStore((s) => s.setFocused);
  const participants = useCallStore((s) => s.participants);
  const muted = useCallStore((s) => s.micMuted);
  const setMicMuted = useCallStore((s) => s.setMicMuted);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const { t } = useTranslation(["call", "voice"]);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => setMicMuted(!muted);

  const count = Math.max(Object.keys(participants).length, 1);

  return createPortal(
    <VoicePill
      title={t("inCall")}
      meta={`${count} ${t("voice:connected").toLowerCase()}`}
      isMuted={muted}
      onToggleMute={toggleMute}
      onLeave={() => leave()}
      onOpen={() => setFocused(true)}
      openTitle={t("openCall")}
    />,
    mountPoint,
  );
};

export default ActiveCallBar;
