import React from "react";
import { createPortal } from "react-dom";

import { attachmentAPI } from "../api/attachment";
import { ApiError } from "../../../shared/lib/apiError";
import type { Attachment } from "../../../shared/types/channel";
import { formatFileSize } from "../lib/messageFormat";
import {
  acquireImageUrl,
  discardBrokenImageUrl,
  downloadImageUrl,
  downloadThumbnailUrl,
  downloadVideoPosterUrl,
  posterKey,
  releaseImageUrl,
  retainImageUrl,
  thumbnailKey,
} from "../lib/imageUrlCache";
import Icon from "../../../shared/ui/Icon";
import { OriginalImageViewer, VideoViewer } from "./MediaViewers";

export const IMAGE_PREVIEW_MAX_BYTES = 8 * 1024 * 1024;
export const THUMBNAIL_SOURCE_MAX_BYTES = 25 * 1024 * 1024;
export const VIDEO_PLAY_MAX_BYTES = 200 * 1024 * 1024;

const playableTypes = new Map<string, boolean>();

export const canPlayVideo = (contentType: string): boolean => {
  const cached = playableTypes.get(contentType);
  if (cached !== undefined) return cached;
  let playable = false;
  try {
    playable = document.createElement("video").canPlayType(contentType) !== "";
  } catch {
    playable = false;
  }
  playableTypes.set(contentType, playable);
  return playable;
};

export const AttachmentItem: React.FC<{ attachment: Attachment; thumbnail?: boolean }> = ({
  attachment,
  thumbnail = false,
}) => {
  const isImage =
    attachment.contentType.startsWith("image/") &&
    attachment.sizeBytes <= (thumbnail ? THUMBNAIL_SOURCE_MAX_BYTES : IMAGE_PREVIEW_MAX_BYTES);
  const isVideo =
    attachment.contentType.startsWith("video/") &&
    attachment.sizeBytes <= VIDEO_PLAY_MAX_BYTES &&
    canPlayVideo(attachment.contentType);
  const hasPoster = isVideo && attachment.sizeBytes <= THUMBNAIL_SOURCE_MAX_BYTES;
  const hasPreview = isImage || hasPoster;
  const cacheKey = hasPoster
    ? posterKey(attachment.id)
    : thumbnail
      ? thumbnailKey(attachment.id)
      : attachment.id;
  const [url, setUrl] = React.useState<string | null>(() =>
    acquireImageUrl(cacheKey),
  );
  const [failed, setFailed] = React.useState(false);
  const [viewerOpen, setViewerOpen] = React.useState(false);
  const [visible, setVisible] = React.useState(false);
  const [retryTick, setRetryTick] = React.useState(0);
  const attemptsRef = React.useRef(0);
  const containerRef = React.useRef<HTMLButtonElement>(null);
  const urlRef = React.useRef(url);
  urlRef.current = url;
  const viewerOpenRef = React.useRef(viewerOpen);
  viewerOpenRef.current = viewerOpen;

  React.useEffect(() => {
    return () => releaseImageUrl(cacheKey, urlRef.current);
  }, [cacheKey]);

  React.useEffect(() => {
    if (!hasPreview) return;
    const node = containerRef.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        const intersecting = entries.some((e) => e.isIntersecting);
        if (intersecting) {
          setVisible(true);
        } else {
          setVisible(false);
          if (!hasPoster && !viewerOpenRef.current && urlRef.current) {
            releaseImageUrl(cacheKey, urlRef.current);
            setUrl(null);
          }
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [cacheKey, hasPreview, hasPoster]);

  React.useEffect(() => {
    if (!hasPreview || url || !visible) return;
    const cached = acquireImageUrl(cacheKey);
    if (cached) {
      setUrl(cached);
      return;
    }
    let cancelled = false;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;
    const load = hasPoster
      ? downloadVideoPosterUrl(attachment.id, attachment.sizeBytes)
      : thumbnail
      ? downloadThumbnailUrl(attachment.id, attachment.sizeBytes, IMAGE_PREVIEW_MAX_BYTES)
      : downloadImageUrl(attachment.id, attachment.sizeBytes);
    load
      .then((objectUrl) => {
        if (!cancelled) {
          attemptsRef.current = 0;
          retainImageUrl(cacheKey, objectUrl);
          setUrl(objectUrl);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        if (hasPoster || (thumbnail && err instanceof ApiError && err.status === 404)) {
          setFailed(true);
          return;
        }
        if (attemptsRef.current < 3) {
          const delay = 1500 * 2 ** attemptsRef.current;
          attemptsRef.current += 1;
          retryTimer = setTimeout(() => setRetryTick((t) => t + 1), delay);
        } else {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
      if (retryTimer) clearTimeout(retryTimer);
    };
  }, [attachment.id, attachment.sizeBytes, cacheKey, thumbnail, hasPoster, hasPreview, url, visible, retryTick]);

  const handleDownload = () => {
    void attachmentAPI
      .download(attachment.id, attachment.sizeBytes)
      .then((blob) => {
        const objectUrl = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = objectUrl;
        a.download = attachment.fileName;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        setTimeout(() => URL.revokeObjectURL(objectUrl), 10_000);
      })
      .catch(() => {});
  };

  if (isImage && !failed) {
    return (
      <>
        <button
          ref={containerRef}
          type="button"
          className="msg-attach-image"
          onClick={() => url && setViewerOpen(true)}
        >
          {url ? (
            <img
              src={url}
              alt={attachment.fileName}
              loading="lazy"
              onError={() => {
                if (url) discardBrokenImageUrl(cacheKey, url);
                setUrl(null);
                setFailed(true);
              }}
            />
          ) : (
            <span className="msg-attach-image-loading">
              <span className="loading-spinner" />
            </span>
          )}
        </button>
        {viewerOpen && url && thumbnail && (
          <OriginalImageViewer attachment={attachment} placeholder={url} onClose={() => setViewerOpen(false)} />
        )}
        {viewerOpen &&
          url &&
          !thumbnail &&
          createPortal(
            <div
              className="image-viewer-overlay"
              onClick={() => {
                setViewerOpen(false);
                if (!visible && urlRef.current) {
                  releaseImageUrl(cacheKey, urlRef.current);
                  setUrl(null);
                }
              }}
            >
              <img src={url} alt={attachment.fileName} />
            </div>,
            document.body,
          )}
      </>
    );
  }

  if (hasPoster && !failed) {
    return (
      <>
        <button
          ref={containerRef}
          type="button"
          className={"msg-attach-video msg-attach-video-poster" + (thumbnail ? " tile" : "")}
          aria-label={attachment.fileName}
          title={attachment.fileName}
          onClick={() => setViewerOpen(true)}
        >
          {url ? (
            <img
              src={url}
              alt=""
              onError={() => {
                if (url) discardBrokenImageUrl(cacheKey, url);
                setUrl(null);
                setFailed(true);
              }}
            />
          ) : (
            <span className="msg-attach-image-loading">
              <span className="loading-spinner" />
            </span>
          )}
          <span className="msg-attach-video-play" aria-hidden="true">
            <Icon name="play" size={18} />
          </span>
          <span className="msg-attach-video-size">{formatFileSize(attachment.sizeBytes)}</span>
        </button>
        {viewerOpen && <VideoViewer attachment={attachment} onClose={() => setViewerOpen(false)} />}
      </>
    );
  }

  if (isVideo) {
    return (
      <>
        <button
          type="button"
          className={"msg-attach-file msg-attach-video" + (thumbnail ? " tile" : "")}
          onClick={() => setViewerOpen(true)}
        >
          <span className="msg-attach-file-icon">
            <Icon name="play" size={16} />
          </span>
          <span className="msg-attach-file-info">
            <span className="msg-attach-file-name">{attachment.fileName}</span>
            <span className="msg-attach-file-size">{formatFileSize(attachment.sizeBytes)}</span>
          </span>
        </button>
        {viewerOpen && <VideoViewer attachment={attachment} onClose={() => setViewerOpen(false)} />}
      </>
    );
  }

  return (
    <button type="button" className="msg-attach-file" onClick={handleDownload}>
      <span className="msg-attach-file-icon">
        <Icon name="download" size={16} />
      </span>
      <span className="msg-attach-file-info">
        <span className="msg-attach-file-name">{attachment.fileName}</span>
        <span className="msg-attach-file-size">
          {formatFileSize(attachment.sizeBytes)}
        </span>
      </span>
    </button>
  );
};

export default AttachmentItem;
