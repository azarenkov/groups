import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "./Icon";
import { clampOffset, coverScale, cropFromView } from "../lib/imageCrop";
import type { CropRect } from "../lib/imageCrop";

const MAX_ZOOM = 4;

export interface ImageCropOptions {
  aspect: number;
  shape: "circle" | "rect";
  title: string;
}

interface ImageCropModalProps extends ImageCropOptions {
  file: File;
  onCancel: () => void;
  onConfirm: (rect: CropRect) => void;
}

const ImageCropModal: React.FC<ImageCropModalProps> = ({
  file,
  aspect,
  shape,
  title,
  onCancel,
  onConfirm,
}) => {
  const { t } = useTranslation("common");
  const frameWidth = aspect >= 2 ? 480 : 300;
  const frameHeight = Math.round(frameWidth / aspect);
  const [url, setUrl] = React.useState<string | null>(null);
  const [natural, setNatural] = React.useState<{ width: number; height: number } | null>(null);
  const [zoom, setZoom] = React.useState(1);
  const [offset, setOffset] = React.useState({ offsetX: 0, offsetY: 0 });
  const [failed, setFailed] = React.useState(false);
  const dragRef = React.useRef<{ x: number; y: number; offsetX: number; offsetY: number } | null>(null);

  React.useEffect(() => {
    const objectUrl = URL.createObjectURL(file);
    setUrl(objectUrl);
    return () => URL.revokeObjectURL(objectUrl);
  }, [file]);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  const view = natural
    ? {
        frameWidth,
        frameHeight,
        naturalWidth: natural.width,
        naturalHeight: natural.height,
        zoom,
        ...offset,
      }
    : null;

  const applyZoom = (next: number) => {
    if (!view) return;
    const clamped = Math.min(MAX_ZOOM, Math.max(1, next));
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

  const scale = view ? coverScale(view) * zoom : 1;

  return (
    <div className="modal-overlay image-crop-overlay" onMouseDown={onCancel}>
      <div
        className="modal image-crop-modal"
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="m-head">
          <h3>{title}</h3>
          <button type="button" tabIndex={-1} className="close" onClick={onCancel} title={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div
            className={"image-crop-frame " + shape}
            style={{ width: frameWidth, height: frameHeight }}
            onPointerDown={(e) => {
              if (!view) return;
              e.currentTarget.setPointerCapture(e.pointerId);
              dragRef.current = { x: e.clientX, y: e.clientY, ...offset };
            }}
            onPointerMove={(e) => {
              const drag = dragRef.current;
              if (!drag || !view) return;
              setOffset(
                clampOffset({
                  ...view,
                  offsetX: drag.offsetX + (e.clientX - drag.x),
                  offsetY: drag.offsetY + (e.clientY - drag.y),
                }),
              );
            }}
            onPointerUp={() => {
              dragRef.current = null;
            }}
            onPointerCancel={() => {
              dragRef.current = null;
            }}
            onWheel={(e) => applyZoom(zoom - e.deltaY * 0.0015)}
          >
            {url && !failed && (
              <img
                src={url}
                alt=""
                draggable={false}
                className="image-crop-picture"
                style={
                  natural
                    ? {
                        width: natural.width * scale,
                        height: natural.height * scale,
                        transform: `translate(${offset.offsetX}px, ${offset.offsetY}px)`,
                      }
                    : { visibility: "hidden" }
                }
                onLoad={(e) => {
                  const img = e.currentTarget;
                  const size = { width: img.naturalWidth, height: img.naturalHeight };
                  setNatural(size);
                  const base = Math.max(frameWidth / size.width, frameHeight / size.height);
                  setOffset({
                    offsetX: (frameWidth - size.width * base) / 2,
                    offsetY: (frameHeight - size.height * base) / 2,
                  });
                }}
                onError={() => setFailed(true)}
              />
            )}
            <div className="image-crop-mask" />
          </div>
          {failed ? (
            <div className="image-crop-error">{t("imageUnreadable")}</div>
          ) : (
            <input
              type="range"
              className="s-range image-crop-zoom"
              min={1}
              max={MAX_ZOOM}
              step={0.01}
              value={zoom}
              aria-label={t("zoom")}
              style={{ "--pct": `${((zoom - 1) / (MAX_ZOOM - 1)) * 100}%` } as React.CSSProperties}
              onChange={(e) => applyZoom(Number(e.target.value))}
            />
          )}
        </div>
        <div className="m-foot">
          <button type="button" onClick={onCancel}>
            {t("cancel")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={!view || failed}
            onClick={() => view && onConfirm(cropFromView(view))}
          >
            {t("apply")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ImageCropModal;
