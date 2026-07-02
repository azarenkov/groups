import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTileCell } from "./VideoStage";
import { useCallStore } from "../../store/callStore";
import { useAuthStore } from "../../store/authStore";
import { useParticipants } from "../../hooks/useParticipants";
import { useVideoTiles } from "../../hooks/useVideoTiles";
import { getLiveKitService } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

const CallStage: React.FC<{ onMinimize: () => void }> = ({ onMinimize }) => {
  const { t } = useTranslation(["call", "voice"]);
  const participants = useCallStore((s) => s.participants);
  const leave = useCallStore((s) => s.leave);
  const user = useAuthStore((s) => s.user);
  const voiceParticipants = useParticipants();
  const videoTiles = useVideoTiles();

  const selfId = user?.id || "you";
  const selfName = user?.name || "You";

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
    <div className="voice-stage-wrap">
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
          {avatarTiles.map((tl) => (
            <div key={tl.id} className="voice-tile">
              <div className="voice-tile-ava" style={{ background: tl.color }}>
                {tl.initials}
              </div>
              <div className="voice-tile-name">{tl.name}</div>
            </div>
          ))}
        </div>
        <div className="voice-room-actions">
          <button
            type="button"
            className={"voice-room-action" + (localCameraOn ? " on" : "")}
            onClick={() => {
              getLiveKitService()
                .setCameraEnabled(!localCameraOn)
                .catch((e) => console.error("[call] camera toggle failed:", e));
            }}
          >
            <Icon name="camera" size={16} />
            <span>
              {localCameraOn ? t("voice:stopCamera") : t("voice:startCamera")}
            </span>
          </button>
          <button
            type="button"
            className={"voice-room-action" + (localScreenOn ? " on" : "")}
            onClick={() => {
              getLiveKitService()
                .setScreenShareEnabled(!localScreenOn)
                .catch((e) => console.error("[call] screen toggle failed:", e));
            }}
          >
            <Icon name="monitor" size={16} />
            <span>
              {localScreenOn
                ? t("voice:stopScreenShare")
                : t("voice:startScreenShare")}
            </span>
          </button>
          <button
            type="button"
            className="voice-room-action leave"
            onClick={() => leave()}
          >
            <Icon name="logout" size={16} />
            <span>{t("voice:leave")}</span>
          </button>
        </div>
      </div>
    </div>
  );
};

export default CallStage;
