import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { isTextEntryTarget } from "../../settings/lib/hotkeyMatching";
import VoiceTheater, { TheaterEntry } from "./VoiceTheater";
import { useCallStore } from "../store/callStore";
import { useAuthStore } from "../../auth/store/authStore";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useParticipants } from "../hooks/useParticipants";
import { useVideoTiles } from "../hooks/useVideoTiles";
import { getLiveKitService } from "../lib/livekit";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { useIsSpeaking } from "../store/speakingStore";

const CallStage: React.FC<{ onMinimize: () => void }> = ({ onMinimize }) => {
  const participants = useCallStore((s) => s.participants);
  const ringing = useCallStore((s) => s.ringing);
  const leave = useCallStore((s) => s.leave);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const muted = useCallStore((s) => s.micMuted);
  const setMicMuted = useCallStore((s) => s.setMicMuted);
  const deafened = useCallStore((s) => s.deafened);
  const setDeafened = useCallStore((s) => s.setDeafened);
  const user = useAuthStore((s) => s.user);
  const friends = useFriendshipStore((s) => s.friends);
  const voiceParticipants = useParticipants();
  const videoTiles = useVideoTiles();
  const { t } = useTranslation("call");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || isTextEntryTarget(e.target)) return;
      if (
        document.querySelector(
          ".settings-overlay, .modal-overlay, .ssp-overlay, .incoming-call, .cs-menu-pop, .call-invite-pop, .vp-popover, .acb-add",
        )
      ) {
        return;
      }
      e.preventDefault();
      onMinimize();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onMinimize]);

  const selfId = user?.id || "you";
  const selfName = user?.name || "You";

  const localCameraOn = videoTiles.some(
    (v) => v.isLocal && v.source === "camera",
  );
  const localScreenOn = videoTiles.some(
    (v) => v.isLocal && v.source === "screen",
  );

  const speakingIds = useMemo(
    () =>
      new Set(
        voiceParticipants.filter((p) => p.isSpeaking).map((p) => p.identity),
      ),
    [voiceParticipants],
  );
  const mutedIds = useMemo(
    () =>
      new Set(voiceParticipants.filter((p) => p.isMuted).map((p) => p.identity)),
    [voiceParticipants],
  );
  const deafenedIds = useMemo(
    () =>
      new Set(
        voiceParticipants.filter((p) => p.isDeafened).map((p) => p.identity),
      ),
    [voiceParticipants],
  );
  const selfSpeaking = useIsSpeaking(selfId);
  const cameraByIdentity = useMemo(() => {
    const m = new Map<string, (typeof videoTiles)[number]>();
    for (const v of videoTiles) if (v.source === "camera") m.set(v.identity, v);
    return m;
  }, [videoTiles]);
  const screenTile = useMemo(
    () => videoTiles.find((v) => v.source === "screen") ?? null,
    [videoTiles],
  );

  const roster = useMemo<TheaterEntry[]>(() => {
    const names = new Map<string, string>();
    names.set(selfId, selfName);
    for (const [id, name] of Object.entries(participants)) names.set(id, name);
    for (const p of voiceParticipants) {
      if (!names.has(p.identity)) names.set(p.identity, p.name || p.identity);
    }
    return Array.from(names.entries()).map(([id, name]) => ({
      id,
      name,
      isSelf: id === selfId,
      color: colorForId(id),
      initials: initialFor(name),
      speaking: id === selfId ? selfSpeaking : speakingIds.has(id),
      muted: id === selfId ? muted : mutedIds.has(id),
      deafened: id === selfId ? deafened : deafenedIds.has(id),
      moderatorDeafened: false,
      camera: cameraByIdentity.get(id) ?? null,
    }));
  }, [
    participants,
    voiceParticipants,
    selfId,
    selfName,
    selfSpeaking,
    speakingIds,
    mutedIds,
    muted,
    deafenedIds,
    deafened,
    cameraByIdentity,
  ]);

  const busyIds = useMemo(
    () => new Set([...Object.keys(participants), ...ringing]),
    [participants, ringing],
  );
  const candidates = useMemo(
    () =>
      friends
        .filter((f) => f.status === "accepted" && !busyIds.has(f.friendId))
        .map((f) => ({ id: f.friendId, name: f.friendName })),
    [friends, busyIds],
  );

  const sharerName = screenTile
    ? (roster.find((r) => r.id === screenTile.identity)?.name ?? selfName)
    : null;

  const toggleMute = () => setMicMuted(!muted);

  return (
    <VoiceTheater
      roster={roster}
      screenTile={screenTile}
      liveName={sharerName}
      isMuted={muted}
      isDeafened={deafened}
      onToggleDeafen={() => setDeafened(!deafened)}
      cameraOn={localCameraOn}
      screenOn={localScreenOn}
      onToggleMute={toggleMute}
      onToggleCamera={() =>
        getLiveKitService()
          .setCameraEnabled(!localCameraOn)
          .catch((e) => console.error("[call] camera toggle failed:", e))
      }
      onToggleScreen={() =>
        getLiveKitService()
          .setScreenShareEnabled(!localScreenOn)
          .catch((e) => console.error("[call] screen toggle failed:", e))
      }
      onLeave={() => leave()}
      onMinimize={onMinimize}
      minimizeTitle={t("minimizeCall")}
      invite={{
        candidates,
        onOpen: () => void useFriendshipStore.getState().loadAll(),
        onAdd: (ids) =>
          addParticipants(ids).catch((e) =>
            console.warn("[call] add participants failed:", e),
          ),
      }}
    />
  );
};

export default CallStage;
