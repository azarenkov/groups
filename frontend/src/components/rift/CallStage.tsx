import React, { useMemo, useState } from "react";
import VoiceTheater, { TheaterEntry } from "./VoiceTheater";
import { useCallStore } from "../../store/callStore";
import { useAuthStore } from "../../store/authStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { useParticipants } from "../../hooks/useParticipants";
import { useVideoTiles } from "../../hooks/useVideoTiles";
import { getLiveKitService } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

const CallStage: React.FC<{ onMinimize: () => void }> = ({ onMinimize }) => {
  const participants = useCallStore((s) => s.participants);
  const ringing = useCallStore((s) => s.ringing);
  const leave = useCallStore((s) => s.leave);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const user = useAuthStore((s) => s.user);
  const friends = useFriendshipStore((s) => s.friends);
  const voiceParticipants = useParticipants();
  const videoTiles = useVideoTiles();

  const selfId = user?.id || "you";
  const selfName = user?.name || "You";
  const [muted, setMuted] = useState(false);

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
      speaking: speakingIds.has(id),
      muted: id === selfId ? muted : mutedIds.has(id),
      camera: cameraByIdentity.get(id) ?? null,
    }));
  }, [
    participants,
    voiceParticipants,
    selfId,
    selfName,
    speakingIds,
    mutedIds,
    muted,
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

  const toggleMute = () => {
    const next = !muted;
    getLiveKitService().setMicrophoneMuted(next);
    setMuted(next);
  };

  return (
    <VoiceTheater
      roster={roster}
      screenTile={screenTile}
      liveName={sharerName}
      isMuted={muted}
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
      invite={{
        candidates,
        onAdd: (ids) =>
          addParticipants(ids).catch((e) =>
            console.warn("[call] add participants failed:", e),
          ),
      }}
    />
  );
};

export default CallStage;
