import React, { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTileCell } from "./VideoStage";
import { useCallStore } from "../../store/callStore";
import { useAuthStore } from "../../store/authStore";
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
  const leave = useCallStore((s) => s.leave);
  const user = useAuthStore((s) => s.user);
  const voiceParticipants = useParticipants();
  const videoTiles = useVideoTiles();

  const selfId = user?.id || "you";
  const selfName = user?.name || "You";

  const [tileVolumes, setTileVolumes] = useState<Record<string, number>>({});
  const [openVolumeFor, setOpenVolumeFor] = useState<string | null>(null);
  const volumePopoverRef = useRef<HTMLDivElement>(null);

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

  const cellCount = videoTiles.length + avatarTiles.length;
  const cols =
    cellCount <= 1 ? 1 : cellCount <= 4 ? 2 : cellCount <= 9 ? 3 : 4;

  return (
    <div className={"voice-stage-wrap" + (embedded ? " embedded" : "")}>
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
      <div className="voice-room">
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
          {avatarTiles.map((tl) => {
            const isSelf = tl.id === selfId;
            const vol = tileVolumes[tl.id] ?? 100;
            return (
              <div key={tl.id} className="voice-tile">
                <div className="voice-tile-ava" style={{ background: tl.color }}>
                  {tl.initials}
                </div>
                <div className="voice-tile-name">{tl.name}</div>
                {!isSelf && (
                  <div className="voice-tile-volume-wrap">
                    <button
                      type="button"
                      className={
                        "voice-tile-volume-btn" +
                        (openVolumeFor === tl.id ? " on" : "")
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
                        size={14}
                      />
                    </button>
                    {openVolumeFor === tl.id && (
                      <div
                        className="voice-tile-volume-pop"
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
                )}
              </div>
            );
          })}
        </div>
        <div className="voice-room-actions call-actions">
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
