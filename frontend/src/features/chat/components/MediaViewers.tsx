import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import { attachmentAPI } from "../api/attachment";
import type { Attachment } from "../../../shared/types/channel";
import { acquireImageUrl, downloadImageUrl, releaseImageUrl, retainImageUrl } from "../lib/imageUrlCache";

const useEscape = (onClose: () => void) => {
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);
};

export const OriginalImageViewer: React.FC<{ attachment: Attachment; placeholder: string | null; onClose: () => void }> = ({
  attachment,
  placeholder,
  onClose,
}) => {
  const [url, setUrl] = React.useState<string | null>(() => acquireImageUrl(attachment.id));
  const urlRef = React.useRef(url);
  urlRef.current = url;
  useEscape(onClose);

  React.useEffect(() => {
    if (url) return;
    let cancelled = false;
    downloadImageUrl(attachment.id, attachment.sizeBytes)
      .then((objectUrl) => {
        if (cancelled) return;
        retainImageUrl(attachment.id, objectUrl);
        setUrl(objectUrl);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [attachment.id, attachment.sizeBytes, url]);

  React.useEffect(() => () => releaseImageUrl(attachment.id, urlRef.current), [attachment.id]);

  const shown = url ?? placeholder;
  return createPortal(
    <div className="image-viewer-overlay" onClick={onClose}>
      {shown ? <img src={shown} alt={attachment.fileName} /> : <span className="loading-spinner" />}
    </div>,
    document.body,
  );
};

export const VideoViewer: React.FC<{ attachment: Attachment; onClose: () => void }> = ({ attachment, onClose }) => {
  const { t } = useTranslation("chat");
  const [url, setUrl] = React.useState<string | null>(null);
  const [failed, setFailed] = React.useState(false);
  useEscape(onClose);

  React.useEffect(() => {
    const controller = new AbortController();
    let objectUrl: string | null = null;
    attachmentAPI
      .download(attachment.id, attachment.sizeBytes, controller.signal)
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        setUrl(objectUrl);
      })
      .catch(() => {
        if (!controller.signal.aborted) setFailed(true);
      });
    return () => {
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [attachment.id, attachment.sizeBytes]);

  return createPortal(
    <div className="image-viewer-overlay video-viewer-overlay" onClick={onClose}>
      <div className="video-viewer" onClick={(e) => e.stopPropagation()}>
        {url ? (
          <video src={url} controls autoPlay playsInline />
        ) : failed ? (
          <span className="video-viewer-error">{t("videoLoadFailed")}</span>
        ) : (
          <span className="loading-spinner" />
        )}
        <div className="video-viewer-bar">
          <span className="video-viewer-name">{attachment.fileName}</span>
          {url && (
            <a className="video-viewer-download" href={url} download={attachment.fileName}>
              {t("download")}
            </a>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
};
