import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { isTextEntryTarget } from "../../settings/lib/hotkeyMatching";
import VoiceTheater, { TheaterEntry } from "./VoiceTheater";
import { useCallStore } from "../store/callStore";
import { useAuthStore } from "../../auth/store/authStore";
import { useParticipants } from "../hooks/useParticipants";
import { useVideoTiles } from "../hooks/useVideoTiles";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { useSpeakingStore } from "../store/speakingStore";
import { useScreenShares } from "../hooks/useScreenShares";

const CallStage: React.FC<{ onMinimize: () => void }> = ({ onMinimize }) => {
  const participants = useCallStore((s) => s.participants);
  const muted = useCallStore((s) => s.micMuted);
  const deafened = useCallStore((s) => s.deafened);
  const user = useAuthStore((s) => s.user);
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

  const speakingIds = useSpeakingStore((state) => state.ids);
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
  const cameraByIdentity = useMemo(() => {
    const m = new Map<string, (typeof videoTiles)[number]>();
    for (const v of videoTiles) if (v.source === "camera") m.set(v.identity, v);
    return m;
  }, [videoTiles]);
  const { shares, watching, watched, watch, setWatched } = useScreenShares();
  const screenTiles = useMemo(
    () =>
      watched
        .map((id) => videoTiles.find((v) => v.source === "screen" && v.identity === id))
        .filter((v): v is (typeof videoTiles)[number] => !!v),
    [videoTiles, watched],
  );
  const screenTile = useMemo(
    () =>
      videoTiles.find((v) => v.source === "screen" && v.identity === watching) ??
      null,
    [videoTiles, watching],
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
      deafened: id === selfId ? deafened : deafenedIds.has(id),
      moderatorDeafened: false,
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
    deafenedIds,
    deafened,
    cameraByIdentity,
  ]);

  const sharerName = screenTile
    ? (roster.find((r) => r.id === screenTile.identity)?.name ?? selfName)
    : null;

  return (
    <VoiceTheater
      roster={roster}
      screenTile={screenTile}
      screenTiles={screenTiles}
      watched={watched}
      onSetWatched={setWatched}
      shares={shares}
      watching={watching}
      onWatch={watch}
      liveName={sharerName}
      onMinimize={onMinimize}
      minimizeTitle={t("minimizeCall")}
    />
  );
};

export default CallStage;
