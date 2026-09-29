import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "./Icon";
import { cropFromView } from "../lib/imageCrop";
import { MAX_CROP_ZOOM, useCropView } from "../hooks/useCropView";
import { clampWindow, GIF_MAX_LENGTH_MS, GIF_MIN_LENGTH_MS } from "../lib/videoGif";
import { loadVideo, videoDurationMs, VideoGifError, videoToGif } from "../lib/videoToGif";

const FRAME = 300;

interface VideoAvatarModalProps {
  file: File;
  onCancel: () => void;
  onConfirm: (gif: File) => void;
}

const formatSeconds = (ms: number) => `${(ms / 1000).toFixed(1)} s`;

const rangeStyle = (value: number, min: number, max: number) =>
  ({
    "--pct": `${max > min ? ((value - min) / (max - min)) * 100 : 0}%`,
  }) as React.CSSProperties;

const VideoAvatarModal: React.FC<VideoAvatarModalProps> = ({ file, onCancel, onConfirm }) => {
  const { t } = useTranslation("common");
  const [url, setUrl] = React.useState<string | null>(null);
  const [durationMs, setDurationMs] = React.useState<number | null>(null);
  const [startMs, setStartMs] = React.useState(0);
  const [lengthMs, setLengthMs] = React.useState(GIF_MAX_LENGTH_MS);
  const [error, setError] = React.useState<"unreadable" | "tooLarge" | null>(null);
  const [progress, setProgress] = React.useState<number | null>(null);
  const previewRef = React.useRef<HTMLVideoElement>(null);
  const abortRef = React.useRef<AbortController | null>(null);
  const { view, zoom, setNatural, applyZoom, frameHandlers, mediaStyle } = useCropView(FRAME, FRAME);

  React.useEffect(() => {
    const objectUrl = URL.createObjectURL(file);
    const controller = new AbortController();
    setUrl(objectUrl);
    loadVideo(objectUrl, controller.signal)
      .then((probe) => {
        const total = videoDurationMs(probe);
        probe.removeAttribute("src");
        probe.load();
        setDurationMs(total);
        setLengthMs(Math.min(GIF_MAX_LENGTH_MS, total));
      })
      .catch((err: unknown) => {
        if (!(err instanceof VideoGifError && err.code === "aborted")) setError("unreadable");
      });
    return () => {
      controller.abort();
      abortRef.current?.abort();
      URL.revokeObjectURL(objectUrl);
    };
  }, [file]);

  const cancel = React.useCallback(() => {
    abortRef.current?.abort();
    onCancel();
  }, [onCancel]);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      cancel();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [cancel]);

  React.useEffect(() => {
    const video = previewRef.current;
    if (!video || durationMs === null) return;
    const from = startMs / 1000;
    const to = (startMs + lengthMs) / 1000;
    video.currentTime = from;
    void video.play().catch(() => {});
    const onTime = () => {
      if (video.currentTime >= to || video.currentTime < from - 0.05) video.currentTime = from;
    };
    video.addEventListener("timeupdate", onTime);
    return () => video.removeEventListener("timeupdate", onTime);
  }, [durationMs, startMs, lengthMs]);

  const converting = progress !== null;
  const ready = !!view && durationMs !== null && error !== "unreadable";
  const maxLength = Math.min(GIF_MAX_LENGTH_MS, durationMs ?? GIF_MAX_LENGTH_MS);
  const minLength = Math.min(GIF_MIN_LENGTH_MS, maxLength);
  const maxStart = Math.max(0, (durationMs ?? 0) - lengthMs);

  const apply = async () => {
    if (!view || !url || durationMs === null) return;
    const controller = new AbortController();
    abortRef.current = controller;
    const range = clampWindow(durationMs, startMs, lengthMs);
    setError(null);
    setProgress(0);
    previewRef.current?.pause();
    try {
      const gif = await videoToGif(url, {
        crop: cropFromView(view),
        ...range,
        signal: controller.signal,
        onProgress: setProgress,
      });
      if (!controller.signal.aborted) onConfirm(gif);
    } catch (err) {
      if (controller.signal.aborted) return;
      setError(err instanceof VideoGifError && err.code === "tooLarge" ? "tooLarge" : "unreadable");
      setProgress(null);
      void previewRef.current?.play().catch(() => {});
    }
  };

  return (
    <div className="modal-overlay image-crop-overlay" onMouseDown={cancel}>
      <div
        className="modal image-crop-modal"
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="m-head">
          <h3>{t("videoAvatarTitle")}</h3>
          <button type="button" tabIndex={-1} className="close" onClick={cancel} title={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div
            className="image-crop-frame circle"
            style={{ width: FRAME, height: FRAME }}
            {...(converting ? {} : frameHandlers)}
          >
            {url && error !== "unreadable" && (
              <video
                ref={previewRef}
                src={url}
                muted
                playsInline
                autoPlay
                className="image-crop-picture"
                style={mediaStyle}
                onLoadedMetadata={(e) =>
                  setNatural({
                    width: e.currentTarget.videoWidth,
                    height: e.currentTarget.videoHeight,
                  })
                }
                onError={() => setError("unreadable")}
              />
            )}
            <div className="image-crop-mask" />
          </div>
          {error === "unreadable" ? (
            <div className="image-crop-error">{t("videoUnreadable")}</div>
          ) : (
            <div className="video-avatar-controls">
              <label className="video-avatar-field">
                <span>{t("zoom")}</span>
                <input
                  type="range"
                  className="s-range"
                  min={1}
                  max={MAX_CROP_ZOOM}
                  step={0.01}
                  value={zoom}
                  disabled={!ready || converting}
                  style={rangeStyle(zoom, 1, MAX_CROP_ZOOM)}
                  onChange={(e) => applyZoom(Number(e.target.value))}
                />
              </label>
              <label className="video-avatar-field">
                <span>{t("videoAvatarStart")}</span>
                <input
                  type="range"
                  className="s-range"
                  min={0}
                  max={maxStart}
                  step={100}
                  value={Math.min(startMs, maxStart)}
                  disabled={!ready || converting || maxStart === 0}
                  style={rangeStyle(startMs, 0, maxStart)}
                  onChange={(e) => setStartMs(Number(e.target.value))}
                />
                <span className="video-avatar-value">{formatSeconds(startMs)}</span>
              </label>
              <label className="video-avatar-field">
                <span>{t("videoAvatarLength")}</span>
                <input
                  type="range"
                  className="s-range"
                  min={minLength}
                  max={maxLength}
                  step={100}
                  value={lengthMs}
                  disabled={!ready || converting || maxLength <= minLength}
                  style={rangeStyle(lengthMs, minLength, maxLength)}
                  onChange={(e) => {
                    const next = clampWindow(durationMs ?? 0, startMs, Number(e.target.value));
                    setLengthMs(next.lengthMs);
                    setStartMs(next.startMs);
                  }}
                />
                <span className="video-avatar-value">{formatSeconds(lengthMs)}</span>
              </label>
              {converting && (
                <div
                  className="video-avatar-progress"
                  role="progressbar"
                  aria-valuenow={Math.round(progress * 100)}
                >
                  <span style={{ width: `${Math.round(progress * 100)}%` }} />
                </div>
              )}
              {error === "tooLarge" && <div className="image-crop-error">{t("videoAvatarTooLarge")}</div>}
            </div>
          )}
        </div>
        <div className="m-foot">
          <button type="button" onClick={cancel}>
            {t("cancel")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={!ready || converting}
            onClick={() => void apply()}
          >
            {converting
              ? t("videoAvatarConverting", {
                  pct: Math.round((progress ?? 0) * 100),
                })
              : t("apply")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default VideoAvatarModal;
