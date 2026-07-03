import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTileCell } from "./VideoStage";
import { useCallStore } from "../../store/callStore";
import { useAuthStore } from "../../store/authStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { useParticipants } from "../../hooks/useParticipants";
import { useVideoTiles } from "../../hooks/useVideoTiles";
import { getLiveKitService } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

const STRIP_MAX = 5;

const CallStage: React.FC<{ onMinimize: () => void }> = ({ onMinimize }) => {
  const { t } = useTranslation(["call", "voice"]);
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
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const inviteRef = useRef<HTMLDivElement>(null);

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
  const cameraByIdentity = useMemo(() => {
    const m = new Map<string, (typeof videoTiles)[number]>();
    for (const v of videoTiles) if (v.source === "camera") m.set(v.identity, v);
    return m;
  }, [videoTiles]);
  const screenTile = useMemo(
    () => videoTiles.find((v) => v.source === "screen") ?? null,
    [videoTiles],
  );

  const roster = useMemo(() => {
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
      camera: cameraByIdentity.get(id) ?? null,
    }));
  }, [
    participants,
    voiceParticipants,
    selfId,
    selfName,
    speakingIds,
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

  useEffect(() => {
    if (!addOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (inviteRef.current && !inviteRef.current.contains(e.target as Node)) {
        setAddOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [addOpen]);

  const toggleMute = () => {
    const next = !muted;
    getLiveKitService().setMicrophoneMuted(next);
    setMuted(next);
  };
  const toggleCamera = () =>
    getLiveKitService()
      .setCameraEnabled(!localCameraOn)
      .catch((e) => console.error("[call] camera toggle failed:", e));
  const toggleScreen = () =>
    getLiveKitService()
      .setScreenShareEnabled(!localScreenOn)
      .catch((e) => console.error("[call] screen toggle failed:", e));

  const toggleSelected = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const confirmAdd = () => {
    const ids = Array.from(selected);
    setAddOpen(false);
    setSelected(new Set());
    if (ids.length > 0) {
      addParticipants(ids).catch((e) =>
        console.warn("[call] add participants failed:", e),
      );
    }
  };

  const mainVideo =
    screenTile ??
    roster.find((r) => !r.isSelf && r.camera)?.camera ??
    roster.find((r) => r.camera)?.camera ??
    null;
  const strip = roster.slice(0, STRIP_MAX);
  const overflow = roster.slice(STRIP_MAX);
  const sharerName = screenTile
    ? (roster.find((r) => r.id === screenTile.identity)?.name ?? selfName)
    : null;

  return (
    <div className="call-theater">
      <div className="call-stage">
        <button
          type="button"
          className="call-min"
          title={t("minimize")}
          onClick={onMinimize}
        >
          <Icon name="chevron" size={16} />
        </button>

        {mainVideo ? (
          <div className="call-screen">
            <VideoTileCell
              tile={mainVideo}
              label={roster.find((r) => r.id === mainVideo.identity)?.name ?? ""}
            />
            {sharerName && (
              <div className="call-live">
                <span className="call-live-badge">
                  <span className="call-live-dot" />
                  LIVE
                </span>
                <span className="call-live-name">
                  {t("sharingScreen", { name: sharerName })}
                </span>
              </div>
            )}
          </div>
        ) : (
          <div className="call-screen empty">
            <div className="call-empty-orbs">
              {roster.map((r) => (
                <div key={r.id} className="call-empty-cell">
                  <div
                    className={"call-empty-orb" + (r.speaking ? " speaking" : "")}
                    style={{ "--orb-color": r.color } as React.CSSProperties}
                  >
                    {r.initials}
                  </div>
                  <div className="call-empty-name">
                    {r.isSelf ? t("you") : r.name}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="call-pill">
          <button
            type="button"
            className={"cp-btn" + (muted ? " danger" : "")}
            title={muted ? t("voice:unmuteSelf") : t("voice:muteSelf")}
            onClick={toggleMute}
          >
            <Icon name={muted ? "mic-off" : "mic"} size={20} />
          </button>
          <button
            type="button"
            className={"cp-btn" + (localCameraOn ? " active" : "")}
            title={localCameraOn ? t("voice:stopCamera") : t("voice:startCamera")}
            onClick={toggleCamera}
          >
            <Icon name="camera" size={20} />
          </button>
          <button
            type="button"
            className={"cp-btn" + (localScreenOn ? " active" : "")}
            title={
              localScreenOn
                ? t("voice:stopScreenShare")
                : t("voice:startScreenShare")
            }
            onClick={toggleScreen}
          >
            <Icon name="monitor" size={20} />
          </button>
          <div className="call-invite" ref={inviteRef}>
            <button
              type="button"
              className={"cp-btn" + (addOpen ? " active" : "")}
              title={t("addParticipants")}
              onClick={() => setAddOpen((v) => !v)}
            >
              <Icon name="user-plus" size={20} />
            </button>
            {addOpen && (
              <div className="acb-add call-invite-pop">
                <div className="acb-add-title">{t("addParticipants")}</div>
                <div className="acb-add-list">
                  {candidates.length === 0 && (
                    <div className="acb-add-empty">{t("noFriendsToAdd")}</div>
                  )}
                  {candidates.map((c) => (
                    <button
                      key={c.id}
                      type="button"
                      className={
                        "acb-add-item" + (selected.has(c.id) ? " on" : "")
                      }
                      onClick={() => toggleSelected(c.id)}
                    >
                      <span
                        className="acb-add-ava"
                        style={{ background: colorForId(c.id) }}
                      >
                        {initialFor(c.name)}
                      </span>
                      <span className="acb-add-name">{c.name}</span>
                      {selected.has(c.id) && <Icon name="check" size={14} />}
                    </button>
                  ))}
                </div>
                <div className="acb-add-foot">
                  <button
                    type="button"
                    className="btn"
                    onClick={() => setAddOpen(false)}
                  >
                    {t("cancel")}
                  </button>
                  <button
                    type="button"
                    className="btn primary"
                    disabled={selected.size === 0}
                    onClick={confirmAdd}
                  >
                    {t("add")}
                  </button>
                </div>
              </div>
            )}
          </div>
          <span className="cp-sep" />
          <button
            type="button"
            className="cp-btn leave"
            title={t("voice:leave")}
            onClick={() => leave()}
          >
            <Icon name="logout" size={20} />
          </button>
        </div>
      </div>

      <div className="call-strip">
        {strip.map((r) => (
          <div
            key={r.id}
            className={"cs-tile" + (r.speaking ? " speaking" : "")}
          >
            {r.camera ? (
              <VideoTileCell tile={r.camera} label="" />
            ) : (
              <div className="cs-tile-ava" style={{ background: r.color }}>
                {r.initials}
              </div>
            )}
            <div className="cs-tile-name">
              {r.isSelf ? t("you") : r.name}
            </div>
          </div>
        ))}
        {overflow.length > 0 && (
          <div className="cs-more">
            <div className="cs-more-avas">
              {overflow.slice(0, 2).map((r) => (
                <span key={r.id} style={{ background: r.color }} />
              ))}
            </div>
            <span className="cs-more-count">+{overflow.length}</span>
          </div>
        )}
      </div>
    </div>
  );
};

export default CallStage;
