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
  const setFocused = useCallStore((s) => s.setFocused);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const participants = useCallStore((s) => s.participants);
  const ringing = useCallStore((s) => s.ringing);
  const friends = useFriendshipStore((s) => s.friends);
  const [muted, setMuted] = useState(false);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const { t } = useTranslation(["call", "voice"]);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

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

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => {
    const next = !muted;
    getLiveKitService().setMicrophoneMuted(next);
    setMuted(next);
  };

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
