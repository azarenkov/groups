import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import type { PinnedMessage } from "../api/pins";

interface PinnedBarProps {
  pins: PinnedMessage[];
  currentUserId?: string;
  canUnpin: boolean;
  isHidden?: (pin: PinnedMessage) => boolean;
  onJump: (messageId: string) => void;
  onUnpin: (messageId: string) => void;
}

export function pinPreview(pin: PinnedMessage, t: (key: string, opts?: Record<string, unknown>) => string): string {
  const text = pin.content.split(/\s+/).filter(Boolean).join(" ");
  if (text) return text;
  if (pin.stickerEmoji) return t("pins.sticker", { emoji: pin.stickerEmoji });
  if (pin.hasAttachments) return t("pins.attachment");
  return t("pins.message");
}

const PinnedBar: React.FC<PinnedBarProps> = ({ pins, currentUserId, canUnpin, isHidden, onJump, onUnpin }) => {
  const { t } = useTranslation("chat");
  const [index, setIndex] = React.useState(0);
  const [listOpen, setListOpen] = React.useState(false);
  const wrapRef = React.useRef<HTMLDivElement>(null);
  const latestId = pins[0]?.messageId;

  React.useEffect(() => {
    setIndex(0);
  }, [latestId]);

  React.useEffect(() => {
    if (pins.length === 0) setListOpen(false);
  }, [pins.length]);

  React.useEffect(() => {
    if (!listOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setListOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setListOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [listOpen]);

  if (pins.length === 0) return null;
  const current = pins[Math.min(index, pins.length - 1)];
  const author = (pin: PinnedMessage) =>
    currentUserId && pin.authorId === currentUserId ? t("you") : pin.authorName || t("pins.unknownAuthor");
  const describe = (pin: PinnedMessage) =>
    isHidden?.(pin) ? (
      <span className="pinned-bar-blocked">{t("blockedMessage")}</span>
    ) : (
      <>
        <span className="pinned-bar-author">{author(pin)}: </span>
        {pinPreview(pin, t)}
      </>
    );

  const openCurrent = () => {
    onJump(current.messageId);
    if (pins.length > 1) setIndex((i) => (Math.min(i, pins.length - 1) + 1) % pins.length);
  };

  return (
    <div className="pinned-bar" ref={wrapRef}>
      <button type="button" className="pinned-bar-main" onClick={openCurrent} title={t("pins.showMessage")}>
        <span className="pinned-bar-icon" aria-hidden="true">
          <Icon name="pin" size={14} />
        </span>
        <span className="pinned-bar-text">
          <span className="pinned-bar-title">
            {pins.length > 1
              ? t("pins.titleOf", { index: Math.min(index, pins.length - 1) + 1, count: pins.length })
              : t("pins.title")}
          </span>
          <span className="pinned-bar-preview">{describe(current)}</span>
        </span>
      </button>
      <button
        type="button"
        className={"icon-btn" + (listOpen ? " on" : "")}
        title={t("pins.all")}
        aria-label={t("pins.all")}
        aria-expanded={listOpen}
        onClick={() => setListOpen((o) => !o)}
      >
        <Icon name="more" size={15} />
      </button>
      {listOpen && (
        <div className="popover pinned-list" role="list">
          {pins.map((pin) => (
            <div key={pin.messageId} className="pinned-list-row" role="listitem">
              <button
                type="button"
                className="pinned-list-open"
                onClick={() => {
                  setListOpen(false);
                  onJump(pin.messageId);
                }}
              >
                <span className="pinned-list-preview">{describe(pin)}</span>
              </button>
              {canUnpin && (
                <button
                  type="button"
                  className="icon-btn"
                  title={t("pins.unpin")}
                  aria-label={t("pins.unpin")}
                  onClick={() => onUnpin(pin.messageId)}
                >
                  <Icon name="x" size={12} />
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default PinnedBar;
