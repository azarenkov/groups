import React from "react";
import i18n from "../../../i18n";

import { ParsedLink, domainOf } from "../../../utils/links";
import Icon from "../Icon";

export const LinkPreview: React.FC<{ link: ParsedLink }> = ({ link }) => {
  const [hidden, setHidden] = React.useState(false);
  const [revealed, setRevealed] = React.useState(false);
  if (hidden) return null;

  if (link.kind === "image") {
    if (!revealed) {
      return (
        <button
          type="button"
          className="link-preview-card link-preview-reveal"
          onClick={() => setRevealed(true)}
        >
          <span className="link-preview-favicon">
            <Icon name="eye" size={16} />
          </span>
          <span className="link-preview-info">
            <span className="link-preview-domain">
              {i18n.t("linkPreviewShow", { ns: "chat" })}
            </span>
            <span className="link-preview-url">{domainOf(link.url)}</span>
          </span>
        </button>
      );
    }
    return (
      <a
        href={link.url}
        target="_blank"
        rel="noopener noreferrer"
        className="link-preview-image"
      >
        <img
          src={link.url}
          alt={domainOf(link.url)}
          loading="lazy"
          onError={() => setHidden(true)}
        />
      </a>
    );
  }

  if (link.kind === "youtube" && link.youtubeId) {
    if (!revealed) {
      return (
        <button
          type="button"
          className="link-preview-card link-preview-reveal"
          onClick={() => setRevealed(true)}
        >
          <span className="link-preview-favicon">
            <Icon name="eye" size={16} />
          </span>
          <span className="link-preview-info">
            <span className="link-preview-domain">
              {i18n.t("linkPreviewShowVideo", { ns: "chat" })}
            </span>
            <span className="link-preview-url">{domainOf(link.url)}</span>
          </span>
        </button>
      );
    }
    return (
      <a
        href={link.url}
        target="_blank"
        rel="noopener noreferrer"
        className="link-preview-video"
      >
        <img
          src={`https://img.youtube.com/vi/${link.youtubeId}/hqdefault.jpg`}
          alt="YouTube"
          loading="lazy"
          onError={() => setHidden(true)}
        />
        <span className="link-preview-play">
          <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        </span>
        <span className="link-preview-video-domain">youtube.com</span>
      </a>
    );
  }

  const domain = domainOf(link.url);
  return (
    <a
      href={link.url}
      target="_blank"
      rel="noopener noreferrer"
      className="link-preview-card"
    >
      <span className="link-preview-favicon">
        <Icon name="globe" size={16} />
      </span>
      <span className="link-preview-info">
        <span className="link-preview-domain">{domain}</span>
        <span className="link-preview-url">{link.url}</span>
      </span>
      <Icon name="chev-left" size={14} className="link-preview-arrow" />
    </a>
  );
};

export default LinkPreview;
