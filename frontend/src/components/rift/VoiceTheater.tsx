import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTileCell } from "./VideoStage";
import { getLiveKitService, VideoTile } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

export interface TheaterEntry {
  id: string;
  name: string;
  isSelf: boolean;
  color: string;
  initials: string;
  speaking: boolean;
  muted: boolean;
  camera: VideoTile | null;
}

interface VoiceTheaterProps {
  roster: TheaterEntry[];
  screenTile: VideoTile | null;
  liveName?: string | null;
  isMuted: boolean;
  cameraOn: boolean;
  screenOn: boolean;
  onToggleMute: () => void;
  onToggleCamera: () => void;
  onToggleScreen: () => void;
  onLeave: () => void;
  onMinimize?: () => void;
  invite?: {
    candidates: { id: string; name: string }[];
    onAdd: (ids: string[]) => void;
  };
  viewersFor?: (id: string) => number | undefined;
  onWatchChange?: (id: string, watching: boolean) => void;
}

const STRIP_MAX = 6;

const VoiceTheater: React.FC<VoiceTheaterProps> = ({
  roster,
  screenTile,
  liveName,
  isMuted,
  cameraOn,
  screenOn,
  onToggleMute,
  onToggleCamera,
  onToggleScreen,
  onLeave,
  onMinimize,
  invite,
  viewersFor,
  onWatchChange,
}) => {
  const { t } = useTranslation(["call", "voice"]);
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const inviteRef = useRef<HTMLDivElement>(null);
  const [tileVolumes, setTileVolumes] = useState<Record<string, number>>({});
  const [openVolumeFor, setOpenVolumeFor] = useState<string | null>(null);
  const volRef = useRef<HTMLDivElement>(null);

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

  useEffect(() => {
    if (!openVolumeFor) return;
    const onDoc = (e: MouseEvent) => {
      if (volRef.current && !volRef.current.contains(e.target as Node)) {
        setOpenVolumeFor(null);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [openVolumeFor]);

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
    if (ids.length > 0) invite?.onAdd(ids);
  };

  const mainVideo =
    screenTile ??
    roster.find((r) => !r.isSelf && r.camera)?.camera ??
    roster.find((r) => r.camera)?.camera ??
    null;
  const strip = roster.slice(0, STRIP_MAX);
  const overflow = roster.slice(STRIP_MAX);
  const mainName = mainVideo
    ? (roster.find((r) => r.id === mainVideo.identity)?.name ?? "")
    : "";

  return (
    <div className="call-theater">
      <div className="call-stage">
        {onMinimize && (
          <button
            type="button"
            className="call-min"
            title={t("minimize")}
            onClick={onMinimize}
          >
            <Icon name="chevron" size={16} />
          </button>
        )}

        {mainVideo ? (
          <div className="call-screen">
            <VideoTileCell
              tile={mainVideo}
              label={mainName}
              viewers={
                mainVideo.source === "screen" && viewersFor
                  ? viewersFor(mainVideo.identity)
                  : undefined
              }
              onWatchChange={onWatchChange}
            />
            {liveName && (
              <div className="call-live">
                <span className="call-live-badge">
                  <span className="call-live-dot" />
                  LIVE
                </span>
                <span className="call-live-name">
                  {t("sharingScreen", { name: liveName })}
                </span>
              </div>
            )}
          </div>
        ) : (
          <div className="call-screen empty">
            <div className="call-empty-orbs">
              {roster.map((r) => {
                const vol = tileVolumes[r.id] ?? 100;
                return (
                  <div key={r.id} className="call-empty-cell">
                    <div
                      className={
                        "call-empty-orb" + (r.speaking ? " speaking" : "")
                      }
                      style={{ "--orb-color": r.color } as React.CSSProperties}
                    >
                      {r.initials}
                      {r.muted && (
                        <span className="call-empty-mute">
                          <Icon name="mic-off" size={16} />
                        </span>
                      )}
                      {!r.isSelf && (
                      <div className="call-empty-vol">
                        <button
                          type="button"
                          className={
                            "cs-vol-btn" + (openVolumeFor === r.id ? " on" : "")
                          }
                          title={`Volume ${vol}%`}
                          onClick={(e) => {
                            e.stopPropagation();
                            setOpenVolumeFor((p) => (p === r.id ? null : r.id));
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
                            size={14}
                          />
                        </button>
                        {openVolumeFor === r.id && (
                          <div
                            className="voice-tile-volume-pop call-empty-vol-pop"
                            ref={volRef}
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
                                setTileVolumes((prev) => ({
                                  ...prev,
                                  [r.id]: v,
                                }));
                                getLiveKitService().setRemoteVolume(r.id, v / 100);
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
                      )}
                    </div>
                    <div className="call-empty-name">
                      {r.isSelf ? t("you") : r.name}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <div className="call-pill">
          <button
            type="button"
            className={"cp-btn" + (isMuted ? " danger" : "")}
            title={isMuted ? t("voice:unmuteSelf") : t("voice:muteSelf")}
            onClick={onToggleMute}
          >
            <Icon name={isMuted ? "mic-off" : "mic"} size={18} />
          </button>
          <button
            type="button"
            className={"cp-btn" + (cameraOn ? " active" : "")}
            title={cameraOn ? t("voice:stopCamera") : t("voice:startCamera")}
            onClick={onToggleCamera}
          >
            <Icon name="camera" size={18} />
          </button>
          <button
            type="button"
            className={"cp-btn" + (screenOn ? " active" : "")}
            title={
              screenOn
                ? t("voice:stopScreenShare")
                : t("voice:startScreenShare")
            }
            onClick={onToggleScreen}
          >
            <Icon name="monitor" size={18} />
          </button>
          {invite && (
            <div className="call-invite" ref={inviteRef}>
              <button
                type="button"
                className={"cp-btn" + (addOpen ? " active" : "")}
                title={t("addParticipants")}
                onClick={() => setAddOpen((v) => !v)}
              >
                <Icon name="user-plus" size={18} />
              </button>
              {addOpen && (
                <div className="acb-add call-invite-pop">
                  <div className="acb-add-title">{t("addParticipants")}</div>
                  <div className="acb-add-list">
                    {invite.candidates.length === 0 && (
                      <div className="acb-add-empty">{t("noFriendsToAdd")}</div>
                    )}
                    {invite.candidates.map((c) => (
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
          )}
          <span className="cp-sep" />
          <button
            type="button"
            className="cp-btn leave"
            title={t("voice:leave")}
            onClick={onLeave}
          >
            <Icon name="logout" size={18} />
          </button>
        </div>
      </div>

      {mainVideo && (
      <div className="call-strip">
        {strip.map((r) => {
          const vol = tileVolumes[r.id] ?? 100;
          return (
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
              <div className={"cs-tile-name" + (r.muted ? " muted" : "")}>
                <Icon name={r.muted ? "mic-off" : "mic"} size={12} />
                <span>{r.isSelf ? t("you") : r.name}</span>
              </div>
              {!r.isSelf && (
                <div className="cs-vol">
                  <button
                    type="button"
                    className={
                      "cs-vol-btn" + (openVolumeFor === r.id ? " on" : "")
                    }
                    title={`Volume ${vol}%`}
                    onClick={(e) => {
                      e.stopPropagation();
                      setOpenVolumeFor((p) => (p === r.id ? null : r.id));
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
                  {openVolumeFor === r.id && (
                    <div
                      className="voice-tile-volume-pop cs-vol-pop"
                      ref={volRef}
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
                          setTileVolumes((prev) => ({ ...prev, [r.id]: v }));
                          getLiveKitService().setRemoteVolume(r.id, v / 100);
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
              )}
            </div>
          );
        })}
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
      )}
    </div>
  );
};

export default VoiceTheater;
