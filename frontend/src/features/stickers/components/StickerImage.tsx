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

const useSeen = (ref: React.RefObject<HTMLElement | null>): boolean => {
  const [seen, setSeen] = React.useState(() => typeof IntersectionObserver === "undefined");
  React.useEffect(() => {
    const node = ref.current;
    if (seen || !node) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setSeen(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [ref, seen]);
  return seen;
};

const StickerImage: React.FC<StickerImageProps> = ({ id, emoji, size, width, height, className }) => {
  const ref = React.useRef<HTMLSpanElement>(null);
  const visible = useSeen(ref);
  const url = useStickerUrl(id, visible);
  const ratio = width && height ? width / height : 1;
  const w = ratio >= 1 ? size : Math.round(size * ratio);
  const h = ratio >= 1 ? Math.round(size / ratio) : size;
  return (
    <span ref={ref} className={"sticker-img" + (className ? ` ${className}` : "")} style={{ width: w, height: h }}>
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
