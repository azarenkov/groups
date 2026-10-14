import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import { attachmentAPI } from "../api/attachment";
import { takeRecentVideo } from "../lib/imageUrlCache";
import type { Attachment } from "../../../shared/types/channel";
import LoadingDots from "../../../shared/ui/LoadingDots";

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
  const [url, setUrl] = React.useState<string | null>(null);
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
      .catch(() => {});
    return () => {
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [attachment.id, attachment.sizeBytes]);

  const shown = url ?? placeholder;
  return createPortal(
    <div className="image-viewer-overlay" onClick={onClose}>
      {shown ? <img src={shown} alt={attachment.fileName} /> : <LoadingDots />}
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
    const recent = takeRecentVideo(attachment.id);
    (recent ? Promise.resolve(recent) : attachmentAPI.download(attachment.id, attachment.sizeBytes, controller.signal))
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
        {url && !failed ? (
          <video src={url} controls autoPlay playsInline onError={() => setFailed(true)} />
        ) : failed ? (
          <span className="video-viewer-error">{t("videoLoadFailed")}</span>
        ) : (
          <LoadingDots />
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
