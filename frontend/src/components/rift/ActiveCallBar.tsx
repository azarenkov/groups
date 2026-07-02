import React, { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import VoicePill from "./VoicePill";
import { useCallStore } from "../../store/callStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { getLiveKitService } from "../../services/livekit";

const ActiveCallBar: React.FC = () => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const leave = useCallStore((s) => s.leave);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const participants = useCallStore((s) => s.participants);
  const friends = useFriendshipStore((s) => s.friends);
  const [muted, setMuted] = useState(false);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const { t } = useTranslation(["call", "voice"]);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

  const inCallIds = useMemo(
    () => new Set(Object.keys(participants)),
    [participants],
  );
  const candidates = useMemo(
    () =>
      friends
        .filter((f) => f.status === "accepted" && !inCallIds.has(f.friendId))
        .map((f) => ({ id: f.friendId, name: f.friendName })),
    [friends, inCallIds],
  );

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => {
    const next = !muted;
    getLiveKitService().setMicrophoneMuted(next);
    setMuted(next);
  };

  const count = Math.max(inCallIds.size, 1);

  return createPortal(
    <VoicePill
      title={t("inCall")}
      meta={`${count} ${t("voice:connected").toLowerCase()}`}
      isMuted={muted}
      onToggleMute={toggleMute}
      onLeave={() => leave()}
      invite={{
        candidates,
        onAdd: (ids) =>
          addParticipants(ids).catch((e) =>
            console.warn("[call] add participants failed:", e),
          ),
        labels: {
          title: t("addParticipants"),
          add: t("add"),
          cancel: t("cancel"),
          empty: t("noFriendsToAdd"),
        },
      }}
    />,
    mountPoint,
  );
};

export default ActiveCallBar;
