import React from "react";

import { clampOffset, coverScale } from "../lib/imageCrop";
import type { CropView } from "../lib/imageCrop";

export const MAX_CROP_ZOOM = 4;

interface Natural {
  width: number;
  height: number;
}

export function useCropView(frameWidth: number, frameHeight: number) {
  const [natural, setNaturalState] = React.useState<Natural | null>(null);
  const [zoom, setZoom] = React.useState(1);
  const [offset, setOffset] = React.useState({ offsetX: 0, offsetY: 0 });
  const dragRef = React.useRef<{
    x: number;
    y: number;
    offsetX: number;
    offsetY: number;
  } | null>(null);

  const view: CropView | null = natural
    ? {
        frameWidth,
        frameHeight,
        naturalWidth: natural.width,
        naturalHeight: natural.height,
        zoom,
        ...offset,
      }
    : null;

  const setNatural = React.useCallback(
    (size: Natural) => {
      setNaturalState(size);
      setZoom(1);
      const base = Math.max(frameWidth / size.width, frameHeight / size.height);
      setOffset({
        offsetX: (frameWidth - size.width * base) / 2,
        offsetY: (frameHeight - size.height * base) / 2,
      });
    },
    [frameWidth, frameHeight],
  );

  const applyZoom = (next: number) => {
    if (!view) return;
    const clamped = Math.min(MAX_CROP_ZOOM, Math.max(1, next));
    const before = coverScale(view) * view.zoom;
    const after = coverScale(view) * clamped;
    const centerX = (frameWidth / 2 - view.offsetX) / before;
    const centerY = (frameHeight / 2 - view.offsetY) / before;
    setZoom(clamped);
    setOffset(
      clampOffset({
        ...view,
        zoom: clamped,
        offsetX: frameWidth / 2 - centerX * after,
        offsetY: frameHeight / 2 - centerY * after,
      }),
    );
  };

  const frameHandlers = {
    onPointerDown: (e: React.PointerEvent<HTMLElement>) => {
      if (!view) return;
      e.currentTarget.setPointerCapture(e.pointerId);
      dragRef.current = { x: e.clientX, y: e.clientY, ...offset };
    },
    onPointerMove: (e: React.PointerEvent<HTMLElement>) => {
      const drag = dragRef.current;
      if (!drag || !view) return;
      setOffset(
        clampOffset({
          ...view,
          offsetX: drag.offsetX + (e.clientX - drag.x),
          offsetY: drag.offsetY + (e.clientY - drag.y),
        }),
      );
    },
    onPointerUp: () => {
      dragRef.current = null;
    },
    onPointerCancel: () => {
      dragRef.current = null;
    },
    onWheel: (e: React.WheelEvent<HTMLElement>) => applyZoom(zoom - e.deltaY * 0.0015),
  };

  const scale = view ? coverScale(view) * zoom : 1;
  const mediaStyle: React.CSSProperties = natural
    ? {
        width: natural.width * scale,
        height: natural.height * scale,
        transform: `translate(${offset.offsetX}px, ${offset.offsetY}px)`,
      }
    : { visibility: "hidden" };

  return {
    view,
    zoom,
    natural,
    setNatural,
    applyZoom,
    frameHandlers,
    mediaStyle,
  };
}
