import React, { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import VoicePill from "./VoicePill";
import VoiceTheater, { TheaterEntry } from "./VoiceTheater";
import { channelAPI } from "../../api/channel";
import { useServerStore } from "../../store/serverStore";
import {
  PERM_MUTE_MEMBERS,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";
import { useLiveKit } from "../../hooks/useLiveKit";
import { qualityClass, qualityLabel } from "../../utils/quality";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";
import { colorForId, initialFor } from "../../utils/avatar";

interface VoiceViewProps {
  channelId: string;
  serverId: string;
  channelName: string;
  password?: string;
  currentUserId?: string;
  userName?: string;
  stageActive?: boolean;
  viewersFor?: (id: string) => number | undefined;
  onStreamWatch?: (streamerId: string, watching: boolean) => void;
  onLeave: () => void;
  onJoinFailed?: (message: string) => void;
}

const VoiceView: React.FC<VoiceViewProps> = ({
  channelId,
  serverId,
  channelName,
  password,
  currentUserId,
  userName,
  stageActive,
  viewersFor,
  onStreamWatch,
  onLeave,
  onJoinFailed,
}) => {
  const { t } = useTranslation(["voice", "common"]);
  const onLeaveRef = useRef(onLeave);
  onLeaveRef.current = onLeave;
  const {
    participants,
    videoTiles,
    isConnected,
    isMuted,
    isCameraEnabled,
    isScreenShareEnabled,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
    toggleCamera,
    toggleScreenShare,
  } = useLiveKit({
    onUnexpectedDisconnect: (reason) => {
      console.warn("[VoiceView] unexpected disconnect, leaving:", reason);
      onLeaveRef.current();
    },
  });

  const hasJoinedRef = useRef(false);
  const currentChannelRef = useRef<string | null>(null);
  const isJoiningRef = useRef(false);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const [stageMount, setStageMount] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

  useEffect(() => {
    setStageMount(
      stageActive && isConnected
        ? document.getElementById("voice-stage-slot")
        : null,
    );
  }, [stageActive, isConnected, channelId]);

  useEffect(() => {
    const shouldJoin =
      (!hasJoinedRef.current || currentChannelRef.current !== channelId) &&
      !isJoiningRef.current;

    if (shouldJoin) {
      hasJoinedRef.current = true;
      currentChannelRef.current = channelId;
      isJoiningRef.current = true;
      joinChannel(channelId, serverId, password)
        .then(() => {
          isJoiningRef.current = false;
        })
        .catch((err) => {
          console.error("[VoiceView] join failed:", err);
          hasJoinedRef.current = false;
          isJoiningRef.current = false;
          const message =
            err instanceof Error ? err.message : "Failed to join voice channel";
          onJoinFailed?.(message);
        });
    }

    return () => {
      hasJoinedRef.current = false;
      currentChannelRef.current = null;
      isJoiningRef.current = false;
      leaveChannel();
    };
  }, [channelId, serverId, password, joinChannel, leaveChannel, onJoinFailed]);

  const handleLeave = async () => {
    await leaveChannel();
    onLeave();
  };

  const presenceForChannel = useVoicePresenceStore(
    (s) => s.byChannel[channelId],
  );
  const speakingPerson = participants.find((p) => p.isSpeaking);
  const livekitCount = participants.length + 1;
  const presenceCount = presenceForChannel?.length ?? 0;
  const totalConnected = isConnected
    ? Math.max(livekitCount, presenceCount)
    : 0;

  const selfId = currentUserId || "you";
  const serverOwnerId = useServerStore(
    (s) => s.servers.find((sv) => sv.id === serverId)?.ownerId,
  );
  const rolesData = useRolesStore((s) => s.byServer[serverId]);
  const fetchRoles = useRolesStore((s) => s.fetch);
  useEffect(() => {
    void fetchRoles(serverId).catch(() => {});
  }, [serverId, fetchRoles]);
  const canModerateMute =
    !!currentUserId &&
    (serverOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_MUTE_MEMBERS) !== 0);
  const presenceMembers = useVoicePresenceStore((s) => s.byChannel[channelId]);
  const moderateMute = (id: string, currentlyMuted: boolean) => {
    if (id === serverOwnerId) return;
    const rec = presenceMembers?.find((m) => m.id === id);
    const moderatorMuted =
      currentlyMuted && !!rec?.mutedBy && rec.mutedBy !== id;
    const call = moderatorMuted
      ? channelAPI.unmute(serverId, channelId, id)
      : channelAPI.mute(serverId, channelId, id);
    call.catch((err) => console.warn("[voice] moderate mute failed:", err));
  };
  const selfName = userName || "You";

  const cameraByIdentity = useMemo(() => {
    const m = new Map<string, (typeof videoTiles)[number]>();
    for (const v of videoTiles) if (v.source === "camera") m.set(v.identity, v);
    return m;
  }, [videoTiles]);
  const screenTile = useMemo(
    () => videoTiles.find((v) => v.source === "screen") ?? null,
    [videoTiles],
  );
  const speakingIds = useMemo(
    () => new Set(participants.filter((p) => p.isSpeaking).map((p) => p.identity)),
    [participants],
  );
  const mutedIds = useMemo(() => {
    const m = new Map<string, boolean>();
    for (const p of participants) m.set(p.identity, p.isMuted);
    for (const mem of presenceForChannel ?? []) m.set(mem.id, mem.isMuted);
    return m;
  }, [participants, presenceForChannel]);
  const roster = useMemo<TheaterEntry[]>(() => {
    const names = new Map<string, string>();
    names.set(selfId, selfName);
    for (const m of presenceForChannel ?? []) names.set(m.id, m.name);
    for (const p of participants) {
      if (!names.has(p.identity)) names.set(p.identity, p.name || p.identity);
    }
    return Array.from(names.entries()).map(([id, name]) => ({
      id,
      name,
      isSelf: id === selfId,
      color: colorForId(id),
      initials: initialFor(name),
      speaking: id === selfId ? false : speakingIds.has(id),
      muted: id === selfId ? isMuted : (mutedIds.get(id) ?? false),
      camera: cameraByIdentity.get(id) ?? null,
    }));
  }, [
    participants,
    presenceForChannel,
    selfId,
    selfName,
    speakingIds,
    mutedIds,
    isMuted,
    cameraByIdentity,
  ]);
  const sharerName = screenTile
    ? (roster.find((r) => r.id === screenTile.identity)?.name ?? selfName)
    : null;

  if (!mountPoint) return null;

  if (error) {
    return createPortal(
      <div
        className="voice-pill in-dock"
        style={{ background: "var(--danger-tint)", color: "var(--danger)" }}
      >
        <span>{error}</span>
        <button
          className="vp-btn leave"
          onClick={handleLeave}
          title={t("leave")}
        >
          <Icon name="logout" size={15} />
        </button>
      </div>,
      mountPoint,
    );
  }

  if (!isConnected) {
    return createPortal(
      <div className="voice-pill in-dock connecting">
        <div className="vp-meta-wrap">
          <div className="vp-title">{channelName}</div>
          <div className="vp-meta">{t("connecting")}</div>
        </div>
        <button
          className="vp-btn leave"
          onClick={handleLeave}
          title={t("common:cancel")}
        >
          <Icon name="logout" size={15} />
        </button>
      </div>,
      mountPoint,
    );
  }

  const meta = speakingPerson
    ? `${speakingPerson.name || speakingPerson.identity} — ${t("speaking")}`
    : `${totalConnected} ${t("connected").toLowerCase()}`;

  return (
    <>
      {createPortal(
        <VoicePill
          title={channelName}
          meta={meta}
          isMuted={isMuted}
          onToggleMute={toggleMute}
          onLeave={handleLeave}
          className={qualityClass(localQuality)}
          titleAttr={`${t("connected")}: ${qualityLabel(localQuality)}`}
        />,
        mountPoint,
      )}
      {stageMount &&
        createPortal(
          <VoiceTheater
            roster={roster}
            screenTile={screenTile}
            liveName={sharerName}
            isMuted={isMuted}
            cameraOn={isCameraEnabled}
            screenOn={isScreenShareEnabled}
            onToggleMute={toggleMute}
            onToggleCamera={() => toggleCamera()}
            onToggleScreen={() => toggleScreenShare()}
            onLeave={handleLeave}
            viewersFor={viewersFor}
            onWatchChange={onStreamWatch}
            onModerateMute={canModerateMute ? moderateMute : undefined}
            moderationExcluded={serverOwnerId ? [serverOwnerId] : undefined}
          />,
          stageMount,
        )}
    </>
  );
};

export default VoiceView;
