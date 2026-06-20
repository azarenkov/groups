import React, { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTile } from "../../services/livekit";

export const VideoTileCell: React.FC<{ tile: VideoTile; label: string }> = ({
  tile,
  label,
}) => {
  const ref = useRef<HTMLVideoElement>(null);
  const { t } = useTranslation("voice");

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    tile.track.attach(el);
    return () => {
      tile.track.detach(el);
    };
  }, [tile.track]);

  const goFullscreen = () => {
    const el = ref.current;
    if (el?.requestFullscreen) {
      el.requestFullscreen().catch(() => {});
    }
  };

  return (
    <div className="voice-tile video">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={tile.isLocal}
        className={tile.source === "camera" ? "mirror" : ""}
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
      <div className="voice-tile-name">
        {label}
        {tile.source === "screen" ? ` · ${t("screen")}` : ""}
      </div>
    </div>
  );
};
