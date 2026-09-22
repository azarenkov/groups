import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import {
  REMOTE_VOLUME_MAX,
  REMOTE_VOLUME_PLAIN_MAX,
  VideoTile,
  getLiveKitService,
} from "../lib/livekit";

export const VideoTileCell: React.FC<{
  tile: VideoTile;
  label: string;
  viewers?: number;
  onWatchChange?: (streamerId: string, watching: boolean) => void;
}> = ({ tile, label, viewers, onWatchChange }) => {
  const ref = useRef<HTMLVideoElement>(null);
  const { t } = useTranslation("voice");
  const [volOpen, setVolOpen] = useState(false);
  const [volume, setVolume] = useState(() =>
    Math.round((getLiveKitService().getRemoteVolume(tile.identity) ?? 1) * 100),
  );
  const [volumeMax, setVolumeMax] = useState(REMOTE_VOLUME_PLAIN_MAX);
  const volRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    tile.track.attach(el);
    return () => {
      tile.track.detach(el);
    };
  }, [tile.track]);

  useEffect(() => {
    if (tile.isLocal || tile.source !== "screen" || !onWatchChange) return;
    onWatchChange(tile.identity, true);
    return () => onWatchChange(tile.identity, false);
  }, [tile.isLocal, tile.source, tile.identity, onWatchChange]);

  useEffect(() => {
    if (!volOpen) return;
    const service = getLiveKitService();
    setVolume(Math.round(service.getRemoteVolume(tile.identity) * 100));
    setVolumeMax(
      service.supportsRemoteBoost(tile.identity)
        ? REMOTE_VOLUME_MAX
        : REMOTE_VOLUME_PLAIN_MAX,
    );
    const onDoc = (e: MouseEvent) => {
      if (volRef.current && !volRef.current.contains(e.target as Node)) {
        setVolOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [volOpen, tile.identity]);

  const goFullscreen = () => {
    const el = ref.current;
    if (el?.requestFullscreen) {
      el.requestFullscreen().catch(() => {});
    }
  };

  const changeVolume = (v: number) => {
    setVolume(v);
    getLiveKitService().setRemoteVolume(tile.identity, v / 100);
  };

  return (
    <div className="voice-tile video">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={tile.isLocal}
        className={tile.isLocal && tile.source === "camera" ? "mirror" : ""}
      />
      <button
        type="button"
        className="voice-tile-fs"
        onClick={goFullscreen}
        title={t("fullscreen")}
      >
        <Icon name="expand" size={14} />
        <span>{t("fullscreen")}</span>
      </button>
      {!tile.isLocal && (
        <div className="voice-tile-vol" ref={volRef}>
          <button
            type="button"
            className={"voice-tile-vol-btn" + (volOpen ? " on" : "")}
            title={`${t("volume")} ${volume}%`}
            onClick={() => setVolOpen((v) => !v)}
          >
            <Icon
              name={
                volume === 0
                  ? "speaker-off"
                  : volume < 40
                    ? "speaker-lo"
                    : "speaker-hi"
              }
              size={14}
            />
          </button>
          {volOpen && (
            <div
              className="voice-tile-vol-pop"
              onMouseDown={(e) => e.stopPropagation()}
            >
              <input
                type="range"
                min={0}
                max={volumeMax}
                step={1}
                value={volume}
                onChange={(e) => changeVolume(Number(e.target.value))}
              />
              <span className="voice-tile-vol-pct">{volume}%</span>
            </div>
          )}
        </div>
      )}
      {tile.source === "screen" && viewers !== undefined && (
        <div className="voice-tile-viewers" title={t("viewers")}>
          <Icon name="eye" size={12} />
          <span>{viewers}</span>
        </div>
      )}
      <div className="voice-tile-name">
        {label}
        {tile.source === "screen" ? ` · ${t("screen")}` : ""}
      </div>
    </div>
  );
};
