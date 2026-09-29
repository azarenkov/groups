import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "./Icon";
import { cropFromView } from "../lib/imageCrop";
import { MAX_CROP_ZOOM, useCropView } from "../hooks/useCropView";
import type { CropRect } from "../lib/imageCrop";

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
  const [failed, setFailed] = React.useState(false);
  const { view, zoom, setNatural, applyZoom, frameHandlers, mediaStyle } = useCropView(
    frameWidth,
    frameHeight,
  );

  React.useEffect(() => {
    const objectUrl = URL.createObjectURL(file);
    setUrl(objectUrl);
    return () => URL.revokeObjectURL(objectUrl);
  }, [file]);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      onCancel();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onCancel]);

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
            {...frameHandlers}
          >
            {url && !failed && (
              <img
                src={url}
                alt=""
                draggable={false}
                className="image-crop-picture"
                style={mediaStyle}
                onLoad={(e) =>
                  setNatural({
                    width: e.currentTarget.naturalWidth,
                    height: e.currentTarget.naturalHeight,
                  })
                }
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
              max={MAX_CROP_ZOOM}
              step={0.01}
              value={zoom}
              aria-label={t("zoom")}
              style={
                {
                  "--pct": `${((zoom - 1) / (MAX_CROP_ZOOM - 1)) * 100}%`,
                } as React.CSSProperties
              }
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
