import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import VoicePill from "./VoicePill";
import { useLiveKit } from "../../hooks/useLiveKit";
import { qualityClass, qualityLabel } from "../../utils/quality";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";

interface VoiceViewProps {
  channelId: string;
  serverId: string;
  channelName: string;
  password?: string;
  onLeave: () => void;
  onJoinFailed?: (message: string) => void;
}

const VoiceView: React.FC<VoiceViewProps> = ({
  channelId,
  serverId,
  channelName,
  password,
  onLeave,
  onJoinFailed,
}) => {
  const { t } = useTranslation(["voice", "common"]);
  const onLeaveRef = useRef(onLeave);
  onLeaveRef.current = onLeave;
  const {
    participants,
    isConnected,
    isMuted,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
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

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

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

  return createPortal(
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
  );
};

export default VoiceView;
