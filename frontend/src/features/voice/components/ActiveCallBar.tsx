import React, { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import VoicePill from "./VoicePill";
import { useCallStore } from "../store/callStore";
import { useHotkey } from "../../settings/lib/hotkeys";
import { useVideoTiles } from "../hooks/useVideoTiles";

interface ActiveCallBarProps {
  dockId?: string;
  onOpen?: () => void;
}

const formatDuration = (startedAt: number | null, now: number): string => {
  const total = startedAt ? Math.max(0, Math.floor((now - startedAt) / 1000)) : 0;
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const sec = total % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(sec).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
};

const ActiveCallBar: React.FC<ActiveCallBarProps> = ({ dockId = "voice-dock-slot", onOpen }) => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const leave = useCallStore((s) => s.leave);
  const focused = useCallStore((s) => s.focused);
  const setFocused = useCallStore((s) => s.setFocused);
  const flagAttention = useCallStore((s) => s.flagAttention);
  const attention = useCallStore((s) => s.attention);
  const startedAt = useCallStore((s) => s.startedAt);
  const participants = useCallStore((s) => s.participants);
  const muted = useCallStore((s) => s.micMuted);
  const setMicMuted = useCallStore((s) => s.setMicMuted);
  const deafened = useCallStore((s) => s.deafened);
  const setDeafened = useCallStore((s) => s.setDeafened);
  const videoTiles = useVideoTiles();
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const { t } = useTranslation(["call", "voice"]);

  const open = () => {
    setFocused(true);
    onOpen?.();
  };

  useHotkey("toggleMute", () => setMicMuted(!muted), !!activeCallId);
  useHotkey("toggleDeafen", () => setDeafened(!deafened), !!activeCallId);
  useHotkey("leaveVoice", () => {
    void leave();
  }, !!activeCallId);
  useHotkey("toggleCallView", () => {
    if (focused) setFocused(false);
    else open();
  }, !!activeCallId);

  useEffect(() => {
    setMountPoint(document.getElementById(dockId));
  }, [dockId, activeCallId]);

  useEffect(() => {
    if (!activeCallId) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [activeCallId]);

  const remoteScreenShare = videoTiles.some((v) => !v.isLocal && v.source === "screen");
  useEffect(() => {
    if (remoteScreenShare) flagAttention();
  }, [remoteScreenShare, flagAttention]);

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => setMicMuted(!muted);

  const count = Math.max(Object.keys(participants).length, 1);

  return createPortal(
    <VoicePill
      title={t("inCall")}
      meta={`${formatDuration(startedAt, now)} · ${count} ${t("voice:connected").toLowerCase()}`}
      isMuted={muted}
      onToggleMute={toggleMute}
      isDeafened={deafened}
      onToggleDeafen={() => setDeafened(!deafened)}
      onLeave={() => leave()}
      onOpen={open}
      openTitle={t("openCall")}
      badge={attention && !focused}
    />,
    mountPoint,
  );
};

export default ActiveCallBar;
