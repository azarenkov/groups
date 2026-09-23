import React, { useEffect, useMemo, useRef, useState } from "react";
import { useHotkey } from "../../settings/lib/hotkeys";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import VoicePill from "./VoicePill";
import VoiceTheater, { TheaterEntry } from "./VoiceTheater";
import { channelAPI } from "../../communities/api/channel";
import { useCommunityStore } from "../../communities/store/communityStore";
import {
  PERM_DEAFEN_MEMBERS,
  PERM_MUTE_MEMBERS,
  topRoleFor,
  useRolesStore,
  userPermissions,
} from "../../communities/store/rolesStore";
import { useLiveKit } from "../hooks/useLiveKit";
import { useScreenShares } from "../hooks/useScreenShares";
import { qualityClass, qualityLabel } from "../lib/quality";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { useSpeakingStore } from "../store/speakingStore";
import { useVoiceChannelStore } from "../store/voiceChannelStore";
import { colorForId, initialFor } from "../../../shared/lib/avatar";

interface VoiceViewProps {
  channelId: string;
  communityId: string;
  channelName: string;
  password?: string;
  currentUserId?: string;
  userName?: string;
  stageActive?: boolean;
  viewersFor?: (id: string) => number | undefined;
  onStreamWatch?: (streamerId: string, watching: boolean) => void;
  onOpenStage?: () => void;
  onLeave: () => void;
  onJoinFailed?: (message: string) => void;
}

const VoiceView: React.FC<VoiceViewProps> = ({
  channelId,
  communityId,
  channelName,
  password,
  currentUserId,
  userName,
  stageActive,
  viewersFor,
  onStreamWatch,
  onOpenStage,
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
    isModeratorMuted,
    isDeafened,
    isModeratorDeafened,
    isCameraEnabled,
    isScreenShareEnabled,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
    toggleDeafen,
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
      joinChannel(channelId, communityId, password)
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
  }, [channelId, communityId, password, joinChannel, leaveChannel, onJoinFailed]);

  const leaveChannelRef = useRef(leaveChannel);
  leaveChannelRef.current = leaveChannel;

  useEffect(() => {
    useVoiceChannelStore.getState().setSession(channelName, async () => {
      await leaveChannelRef.current();
      onLeaveRef.current();
    });
    return () => {
      if (useVoiceChannelStore.getState().name === channelName) {
        useVoiceChannelStore.getState().setSession(null, null);
      }
    };
  }, [channelName]);

  const handleLeave = async () => {
    await leaveChannel();
    onLeave();
  };

  const presenceForChannel = useVoicePresenceStore(
    (s) => s.byChannel[channelId],
  );
  const speakingIds = useSpeakingStore((state) => state.ids);
  const speakingPerson = participants.find((p) => speakingIds.has(p.identity));
  const livekitCount = participants.length + 1;
  const presenceCount = presenceForChannel?.length ?? 0;
  const totalConnected = isConnected
    ? Math.max(livekitCount, presenceCount)
    : 0;

  const selfId = currentUserId || "you";
  const communityOwnerId = useCommunityStore(
    (s) => s.communities.find((sv) => sv.id === communityId)?.ownerId,
  );
  const rolesData = useRolesStore((s) => s.byCommunity[communityId]);
  const fetchRoles = useRolesStore((s) => s.fetch);
  useEffect(() => {
    void fetchRoles(communityId).catch(() => {});
  }, [communityId, fetchRoles]);
  const muteLocked =
    (isModeratorMuted || isModeratorDeafened) && communityOwnerId !== currentUserId;
  useHotkey("toggleMute", () => {
    if (!muteLocked) toggleMute();
  });
  useHotkey("leaveVoice", () => {
    void handleLeave();
  });
  const deafenLocked = isModeratorDeafened && communityOwnerId !== currentUserId;
  useHotkey("toggleDeafen", () => {
    if (!deafenLocked) toggleDeafen();
  });
  const canModerateDeafen =
    !!currentUserId &&
    (communityOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_DEAFEN_MEMBERS) !== 0);
  const canModerateMute =
    !!currentUserId &&
    (communityOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_MUTE_MEMBERS) !== 0);
  const presenceMembers = useVoicePresenceStore((s) => s.byChannel[channelId]);
  const moderationExcluded = React.useMemo(() => {
    if (!currentUserId) return undefined;
    const isOwner = communityOwnerId === currentUserId;
    const myTop = topRoleFor(rolesData, currentUserId)?.position;
    return (presenceMembers ?? [])
      .filter((m) => {
        if (m.id === currentUserId) return false;
        if (m.id === communityOwnerId) return true;
        if (isOwner) return false;
        if (myTop === undefined) return true;
        const targetTop = topRoleFor(rolesData, m.id)?.position;
        return targetTop !== undefined && targetTop >= myTop;
      })
      .map((m) => m.id);
  }, [currentUserId, communityOwnerId, rolesData, presenceMembers]);
  const moderateMute = (id: string, currentlyMuted: boolean) => {
    if (id === communityOwnerId) return;
    const rec = presenceMembers?.find((m) => m.id === id);
    const moderatorMuted =
      currentlyMuted && !!rec?.mutedBy && rec.mutedBy !== id;
    const call = moderatorMuted
      ? channelAPI.unmute(communityId, channelId, id)
      : channelAPI.mute(communityId, channelId, id);
    call.catch((err) => console.warn("[voice] moderate mute failed:", err));
  };
  const deafenExcluded = React.useMemo(() => {
    if (!currentUserId || currentUserId === communityOwnerId) return undefined;
    return (presenceMembers ?? [])
      .filter(
        (m) => !!m.deafenedBy && m.deafenedBy !== m.id && m.deafenedBy === communityOwnerId,
      )
      .map((m) => m.id);
  }, [currentUserId, communityOwnerId, presenceMembers]);
  const moderateDeafen = (id: string, currentlyDeafened: boolean) => {
    if (id === communityOwnerId) return;
    const call = currentlyDeafened
      ? channelAPI.undeafen(communityId, channelId, id)
      : channelAPI.deafen(communityId, channelId, id);
    call.catch((err) => console.warn("[voice] moderate deafen failed:", err));
  };
  const selfName = userName || "You";

  const cameraByIdentity = useMemo(() => {
    const m = new Map<string, (typeof videoTiles)[number]>();
    for (const v of videoTiles) if (v.source === "camera") m.set(v.identity, v);
    return m;
  }, [videoTiles]);
  const { shares, watching, watch } = useScreenShares(!!stageMount);
  const screenTile = useMemo(
    () =>
      videoTiles.find((v) => v.source === "screen" && v.identity === watching) ??
      null,
    [videoTiles, watching],
  );
  const mutedIds = useMemo(() => {
    const m = new Map<string, boolean>();
    for (const p of participants) m.set(p.identity, p.isMuted);
    for (const mem of presenceForChannel ?? []) m.set(mem.id, mem.isMuted);
    return m;
  }, [participants, presenceForChannel]);
  const deafenedIds = useMemo(() => {
    const m = new Map<string, { deafened: boolean; moderator: boolean }>();
    for (const p of participants) {
      m.set(p.identity, { deafened: p.isDeafened, moderator: false });
    }
    for (const mem of presenceForChannel ?? []) {
      const moderator = !!mem.deafenedBy && mem.deafenedBy !== mem.id;
      const prev = m.get(mem.id);
      m.set(mem.id, {
        deafened: moderator || (prev?.deafened ?? false),
        moderator,
      });
    }
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
      speaking: speakingIds.has(id),
      muted: id === selfId ? isMuted : (mutedIds.get(id) ?? false),
      deafened:
        id === selfId
          ? isDeafened || isModeratorDeafened
          : (deafenedIds.get(id)?.deafened ?? false),
      moderatorDeafened:
        id === selfId
          ? isModeratorDeafened
          : (deafenedIds.get(id)?.moderator ?? false),
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
    deafenedIds,
    isDeafened,
    isModeratorDeafened,
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
          muteLocked={muteLocked}
          onToggleMute={toggleMute}
          isDeafened={isDeafened || isModeratorDeafened}
          deafenLocked={deafenLocked}
          onToggleDeafen={toggleDeafen}
          onLeave={handleLeave}
          onOpen={stageActive ? undefined : onOpenStage}
          openTitle={t("openChannel")}
          active={stageActive}
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
            muteLocked={muteLocked}
            isDeafened={isDeafened || isModeratorDeafened}
            deafenLocked={deafenLocked}
            onToggleDeafen={toggleDeafen}
            cameraOn={isCameraEnabled}
            screenOn={isScreenShareEnabled}
            onToggleMute={toggleMute}
            onToggleCamera={() => toggleCamera()}
            onToggleScreen={() => toggleScreenShare()}
            onLeave={handleLeave}
            viewersFor={viewersFor}
            onWatchChange={onStreamWatch}
            shares={shares}
            watching={watching}
            onWatch={watch}
            onModerateMute={canModerateMute ? moderateMute : undefined}
            onModerateDeafen={canModerateDeafen ? moderateDeafen : undefined}
            moderationExcluded={moderationExcluded}
            deafenExcluded={deafenExcluded}
          />,
          stageMount,
        )}
    </>
  );
};

export default VoiceView;
