import React, { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { VideoTile } from "../../services/livekit";

const VideoTileView: React.FC<{ tile: VideoTile }> = ({ tile }) => {
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

  return (
    <div className="video-tile">
      <video
        ref={ref}
        autoPlay
        playsInline
        muted={tile.isLocal}
        className={tile.source === "camera" ? "mirror" : ""}
      />
      <div className="video-tile-label">
        <Icon name={tile.source === "screen" ? "monitor" : "camera"} size={12} />
        <span>
          {tile.name}
          {tile.isLocal ? ` (${t("you")})` : ""}
          {tile.source === "screen" ? ` · ${t("screen")}` : ""}
        </span>
      </div>
    </div>
  );
};

const VideoStage: React.FC<{ tiles: VideoTile[] }> = ({ tiles }) => {
  const visible = tiles.filter(
    (t) => !(t.isLocal && t.source === "screen"),
  );
  if (visible.length === 0) return null;

  const cols = visible.length === 1 ? 1 : visible.length <= 4 ? 2 : 3;

  return (
    <div
      className="video-stage"
      style={{ ["--vs-cols" as unknown as string]: cols } as React.CSSProperties}
    >
      {visible.map((tile) => (
        <VideoTileView key={tile.id} tile={tile} />
      ))}
    </div>
  );
};

export default VideoStage;
