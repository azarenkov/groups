import React from "react";

import { useStickerUrl } from "../lib/stickerImages";

interface StickerImageProps {
  id: string;
  emoji: string;
  size: number;
  width?: number;
  height?: number;
  className?: string;
  animated?: boolean;
}

const VIEW_MARGIN = "200px";

const useOnScreen = (ref: React.RefObject<HTMLElement | null>, track: boolean) => {
  const supported = typeof IntersectionObserver !== "undefined";
  const [state, setState] = React.useState({ seen: !supported, onScreen: !supported });
  React.useEffect(() => {
    const node = ref.current;
    if (!supported || !node || (state.seen && !track)) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const onScreen = entries.some((entry) => entry.isIntersecting);
        setState((current) => {
          if (current.onScreen === onScreen && (current.seen || !onScreen)) return current;
          return { seen: current.seen || onScreen, onScreen };
        });
        if (onScreen && !track) observer.disconnect();
      },
      { rootMargin: VIEW_MARGIN },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [ref, supported, track, state.seen]);
  return state;
};

const StickerImage: React.FC<StickerImageProps> = ({ id, emoji, size, width, height, className, animated }) => {
  const ref = React.useRef<HTMLSpanElement>(null);
  const { seen, onScreen } = useOnScreen(ref, !!animated);
  const url = useStickerUrl(id, seen);
  const ratio = width && height ? width / height : 1;
  const w = ratio >= 1 ? size : Math.round(size * ratio);
  const h = ratio >= 1 ? Math.round(size / ratio) : size;
  const paused = !!animated && !onScreen;
  return (
    <span ref={ref} className={"sticker-img" + (className ? ` ${className}` : "")} style={{ width: w, height: h }}>
      {url && !paused ? (
        <img src={url} alt={emoji} draggable={false} width={w} height={h} />
      ) : url ? null : (
        <span className="sticker-img-placeholder" aria-hidden="true">
          {emoji}
        </span>
      )}
    </span>
  );
};

export default StickerImage;
