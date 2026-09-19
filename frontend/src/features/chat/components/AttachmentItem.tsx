import React from "react";
import { createPortal } from "react-dom";

import { attachmentAPI } from "../api/attachment";
import type { Attachment } from "../../../shared/types/channel";
import { formatFileSize } from "../lib/messageFormat";
import {
  acquireImageUrl,
  discardBrokenImageUrl,
  downloadImageUrl,
  releaseImageUrl,
  retainImageUrl,
} from "../lib/imageUrlCache";
import Icon from "../../../shared/ui/Icon";

export const IMAGE_PREVIEW_MAX_BYTES = 8 * 1024 * 1024;

export const AttachmentItem: React.FC<{ attachment: Attachment }> = ({
  attachment,
}) => {
  const isImage =
    attachment.contentType.startsWith("image/") &&
    attachment.sizeBytes <= IMAGE_PREVIEW_MAX_BYTES;
  const [url, setUrl] = React.useState<string | null>(() =>
    acquireImageUrl(attachment.id),
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
    return () => releaseImageUrl(attachment.id, urlRef.current);
  }, [attachment.id]);

  React.useEffect(() => {
    if (!isImage) return;
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
          if (!viewerOpenRef.current && urlRef.current) {
            releaseImageUrl(attachment.id, urlRef.current);
            setUrl(null);
          }
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [attachment.id, isImage]);

  React.useEffect(() => {
    if (!isImage || url || !visible) return;
    const cached = acquireImageUrl(attachment.id);
    if (cached) {
      setUrl(cached);
      return;
    }
    let cancelled = false;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;
    downloadImageUrl(attachment.id, attachment.sizeBytes)
      .then((objectUrl) => {
        if (!cancelled) {
          attemptsRef.current = 0;
          retainImageUrl(attachment.id, objectUrl);
          setUrl(objectUrl);
        }
      })
      .catch(() => {
        if (cancelled) return;
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
  }, [attachment.id, attachment.sizeBytes, isImage, url, visible, retryTick]);

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
                if (url) discardBrokenImageUrl(attachment.id, url);
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
        {viewerOpen &&
          url &&
          createPortal(
            <div
              className="image-viewer-overlay"
              onClick={() => {
                setViewerOpen(false);
                if (!visible && urlRef.current) {
                  releaseImageUrl(attachment.id, urlRef.current);
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
