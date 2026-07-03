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

const CallStage: React.FC<{ onMinimize: () => void; embedded?: boolean }> = ({
  onMinimize,
  embedded,
}) => {
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

  const [tileVolumes, setTileVolumes] = useState<Record<string, number>>({});
  const [openVolumeFor, setOpenVolumeFor] = useState<string | null>(null);
  const volumePopoverRef = useRef<HTMLDivElement>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const inviteRef = useRef<HTMLDivElement>(null);

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
    if (!openVolumeFor) return;
    const onDoc = (e: MouseEvent) => {
      if (
        volumePopoverRef.current &&
        !volumePopoverRef.current.contains(e.target as Node)
      ) {
        setOpenVolumeFor(null);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [openVolumeFor]);

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

  const localCameraOn = videoTiles.some(
    (t) => t.isLocal && t.source === "camera",
  );
  const localScreenOn = videoTiles.some(
    (t) => t.isLocal && t.source === "screen",
  );

  const { nameById, avatarTiles } = useMemo(() => {
    const names = new Map<string, string>();
    names.set(selfId, selfName);
    for (const [id, name] of Object.entries(participants)) names.set(id, name);
    for (const p of voiceParticipants) {
      if (!names.has(p.identity)) names.set(p.identity, p.name || p.identity);
    }
    const withVideo = new Set(videoTiles.map((v) => v.identity));
    const tiles: { id: string; name: string; initials: string; color: string }[] =
      [];
    for (const [id, name] of names) {
      if (withVideo.has(id)) continue;
      tiles.push({ id, name, initials: initialFor(name), color: colorForId(id) });
    }
    return { nameById: names, avatarTiles: tiles };
  }, [participants, voiceParticipants, videoTiles, selfId, selfName]);

  const cols =
    videoTiles.length <= 1
      ? 1
      : videoTiles.length <= 4
        ? 2
        : videoTiles.length <= 9
          ? 3
          : 4;
  const speakingIds = useMemo(
    () =>
      new Set(
        voiceParticipants.filter((p) => p.isSpeaking).map((p) => p.identity),
      ),
    [voiceParticipants],
  );

  return (
    <div
      className={
        "voice-stage-wrap" +
        (embedded ? " embedded" : "") +
        (videoTiles.length > 0 ? " has-video" : "")
      }
    >
      {!embedded && (
        <header className="main-header">
          <span className="ch-ico">
            <Icon name="phone" size={16} />
          </span>
          <span className="ch-name">{t("inCall")}</span>
          <span className="sep" />
          <span className="topic voice-stage-status">
            <span className="voice-stage-dot" />
            {`${Math.max(Object.keys(participants).length, 1)} ${t(
              "voice:connected",
            ).toLowerCase()}`}
          </span>
          <div className="right">
            <button
              type="button"
              className="icon-btn"
              title={t("minimize")}
              onClick={onMinimize}
            >
              <Icon name="chevron" size={16} />
            </button>
          </div>
        </header>
      )}
      <div className="voice-room">
        {videoTiles.length > 0 && (
          <div
            className="voice-room-grid"
            style={{ "--vt-cols": cols } as React.CSSProperties}
          >
            {videoTiles.map((vt) => (
              <VideoTileCell
                key={vt.id}
                tile={vt}
                label={
                  vt.isLocal ? selfName : nameById.get(vt.identity) || vt.name
                }
              />
            ))}
          </div>
        )}
        {avatarTiles.length > 0 && (
          <div className="call-orbs">
            {avatarTiles.map((tl) => {
              const isSelf = tl.id === selfId;
              const vol = tileVolumes[tl.id] ?? 100;
              const speaking = speakingIds.has(tl.id);
              return (
                <div key={tl.id} className="call-orb-cell">
                  <div
                    className={"call-orb" + (speaking ? " speaking" : "")}
                    style={{ "--orb-color": tl.color } as React.CSSProperties}
                  >
                    <span className="call-orb-letter">{tl.initials}</span>
                    {!isSelf && (
                      <button
                        type="button"
                        className={
                          "orb-vol-btn" + (openVolumeFor === tl.id ? " on" : "")
                        }
                        title={`Volume ${vol}%`}
                        onClick={(e) => {
                          e.stopPropagation();
                          setOpenVolumeFor((prev) =>
                            prev === tl.id ? null : tl.id,
                          );
                        }}
                      >
                        <Icon
                          name={
                            vol === 0
                              ? "speaker-off"
                              : vol < 40
                                ? "speaker-lo"
                                : "speaker-hi"
                          }
                          size={13}
                        />
                      </button>
                    )}
                  </div>
                  <div className="call-orb-name">{tl.name}</div>
                  {!isSelf && openVolumeFor === tl.id && (
                    <div
                      className="voice-tile-volume-pop orb-vol-pop"
                      ref={volumePopoverRef}
                      onMouseDown={(e) => e.stopPropagation()}
                    >
                      <input
                        type="range"
                        min={0}
                        max={100}
                        step={1}
                        value={vol}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setTileVolumes((prev) => ({ ...prev, [tl.id]: v }));
                          getLiveKitService().setRemoteVolume(tl.id, v / 100);
                        }}
                        style={
                          {
                            ["--pct" as unknown as string]: `${vol}%`,
                          } as React.CSSProperties
                        }
                      />
                      <span className="voice-tile-volume-pct">{vol}%</span>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
        <div className="voice-room-actions call-actions">
          <div className="call-invite" ref={inviteRef}>
            <button
              type="button"
              className={"call-action" + (addOpen ? " on" : "")}
              title={t("addParticipants")}
              onClick={() => setAddOpen((v) => !v)}
            >
              <Icon name="user-plus" size={16} />
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
          <button
            type="button"
            className={"call-action" + (localCameraOn ? " on" : "")}
            title={localCameraOn ? t("voice:stopCamera") : t("voice:startCamera")}
            onClick={() => {
              getLiveKitService()
                .setCameraEnabled(!localCameraOn)
                .catch((e) => console.error("[call] camera toggle failed:", e));
            }}
          >
            <Icon name="camera" size={16} />
          </button>
          <button
            type="button"
            className={"call-action" + (localScreenOn ? " on" : "")}
            title={
              localScreenOn
                ? t("voice:stopScreenShare")
                : t("voice:startScreenShare")
            }
            onClick={() => {
              getLiveKitService()
                .setScreenShareEnabled(!localScreenOn)
                .catch((e) => console.error("[call] screen toggle failed:", e));
            }}
          >
            <Icon name="monitor" size={16} />
          </button>
          <button
            type="button"
            className="call-action leave"
            title={t("voice:leave")}
            onClick={() => leave()}
          >
            <Icon name="logout" size={16} />
          </button>
        </div>
      </div>
    </div>
  );
};

export default CallStage;
