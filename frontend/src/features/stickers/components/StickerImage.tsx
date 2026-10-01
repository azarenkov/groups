import React from "react";

import { useStickerUrl } from "../lib/stickerImages";

interface StickerImageProps {
  id: string;
  emoji: string;
  size: number;
  width?: number;
  height?: number;
  className?: string;
}

const StickerImage: React.FC<StickerImageProps> = ({ id, emoji, size, width, height, className }) => {
  const url = useStickerUrl(id);
  const ratio = width && height ? width / height : 1;
  const w = ratio >= 1 ? size : Math.round(size * ratio);
  const h = ratio >= 1 ? Math.round(size / ratio) : size;
  return (
    <span className={"sticker-img" + (className ? ` ${className}` : "")} style={{ width: w, height: h }}>
      {url ? (
        <img src={url} alt={emoji} draggable={false} width={w} height={h} />
      ) : (
        <span className="sticker-img-placeholder" aria-hidden="true">
          {emoji}
        </span>
      )}
    </span>
  );
};

export default StickerImage;
