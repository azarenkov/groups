import React, { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import VoicePill from "./VoicePill";
import { useCallStore } from "../store/callStore";
import { useHotkey } from "../../settings/lib/hotkeys";
import { useVideoTiles } from "../hooks/useVideoTiles";
import { useSpeakingStore } from "../store/speakingStore";
import { speakingName } from "../lib/speakingName";
import { useAuthStore } from "../../auth/store/authStore";
import { getLiveKitService } from "../lib/livekit";
import { useFriendshipStore } from "../../social/store/friendshipStore";

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
  const ringing = useCallStore((s) => s.ringing);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const friends = useFriendshipStore((s) => s.friends);
  const candidates = useMemo(() => {
    const busy = new Set([...Object.keys(participants), ...ringing]);
    return friends
      .filter((f) => f.status === "accepted" && !busy.has(f.friendId))
      .map((f) => ({ id: f.friendId, name: f.friendName }));
  }, [friends, participants, ringing]);
  const muted = useCallStore((s) => s.micMuted);
  const setMicMuted = useCallStore((s) => s.setMicMuted);
  const deafened = useCallStore((s) => s.deafened);
  const reconnecting = useCallStore((s) => s.reconnecting);
  const setDeafened = useCallStore((s) => s.setDeafened);
  const videoTiles = useVideoTiles();
  const speakingIds = useSpeakingStore((s) => s.ids);
  const selfId = useAuthStore((s) => s.user?.id ?? null);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const { t } = useTranslation(["call", "voice"]);

  const stageVisible = focused && dockId === "voice-dock-slot";

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
  const localCameraOn = videoTiles.some((v) => v.isLocal && v.source === "camera");
  const localScreenOn = videoTiles.some((v) => v.isLocal && v.source === "screen");
  useEffect(() => {
    if (remoteScreenShare) flagAttention();
  }, [remoteScreenShare, flagAttention]);

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => setMicMuted(!muted);

  const count = Math.max(Object.keys(participants).length, 1);
  const speaker = speakingName(participants, speakingIds, selfId);
  const meta = reconnecting
    ? t("voice:reconnecting")
    : speaker
    ? `${speaker} — ${t("voice:speaking")}`
    : `${formatDuration(startedAt, now)} · ${count} ${t("voice:connected").toLowerCase()}`;

  return createPortal(
    <VoicePill
      title={t("inCall")}
      meta={meta}
      isMuted={muted}
      onToggleMute={toggleMute}
      isDeafened={deafened}
      onToggleDeafen={() => setDeafened(!deafened)}
      onLeave={() => leave()}
      onOpen={stageVisible ? undefined : open}
      openTitle={t("openCall")}
      active={stageVisible}
      badge={attention && !focused}
      invite={{
        candidates,
        onOpen: () => void useFriendshipStore.getState().loadAll(),
        onAdd: (ids) =>
          addParticipants(ids).catch((e) => console.warn("[call] add participants failed:", e)),
        labels: {
          title: t("addParticipants"),
          add: t("add"),
          cancel: t("cancel"),
          empty: t("noFriendsToAdd"),
        },
      }}
      cameraOn={localCameraOn}
      onToggleCamera={() =>
        getLiveKitService()
          .setCameraEnabled(!localCameraOn)
          .catch((e) => console.error("[call] camera toggle failed:", e))
      }
      screenOn={localScreenOn}
      onToggleScreen={() =>
        getLiveKitService()
          .setScreenShareEnabled(!localScreenOn)
          .catch((e) => console.error("[call] screen toggle failed:", e))
      }
    />,
    mountPoint,
  );
};

export default ActiveCallBar;
