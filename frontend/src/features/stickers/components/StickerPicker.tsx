import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { useAuthStore } from "../../auth/store/authStore";
import type { PackSticker } from "../api/stickers";
import { installedPacks, searchMyStickers, useStickerStore } from "../store/stickerStore";
import StickerImage from "./StickerImage";
import StickerPacksModal from "./StickerPacksModal";

const PACKS_MAX_AGE_MS = 60_000;

interface StickerPickerProps {
  disabled?: boolean;
  onPick: (sticker: PackSticker) => void;
}

const RECENT_TAB = "recent";

const StickerPicker: React.FC<StickerPickerProps> = ({ disabled, onPick }) => {
  const { t } = useTranslation("stickers");
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const allPacks = useStickerStore((s) => s.packs);
  const packs = React.useMemo(() => installedPacks(allPacks), [allPacks]);
  const recent = useStickerStore((s) => s.recent);
  const loaded = useStickerStore((s) => s.loaded);
  const failed = useStickerStore((s) => s.failed);
  const [open, setOpen] = React.useState(false);
  const [tab, setTab] = React.useState<string>(RECENT_TAB);
  const [query, setQuery] = React.useState("");
  const [managing, setManaging] = React.useState(false);
  const rootRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (!open || !userId) return;
    const store = useStickerStore.getState();
    if (!loaded || failed || store.userId !== userId) void store.load(userId);
    else void store.refreshIfStale(PACKS_MAX_AGE_MS);
  }, [open, userId, loaded, failed]);

  React.useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  React.useEffect(() => {
    if (tab !== RECENT_TAB && !packs.some((p) => p.id === tab)) setTab(RECENT_TAB);
  }, [packs, tab]);

  const searching = query.trim().length > 0;
  const shown: PackSticker[] = searching
    ? searchMyStickers(packs, query)
    : tab === RECENT_TAB
      ? recent
      : packs.find((p) => p.id === tab)?.stickers ?? [];

  const pick = (sticker: PackSticker) => {
    setOpen(false);
    setQuery("");
    onPick(sticker);
  };

  return (
    <div className="sticker-picker" ref={rootRef}>
      <button
        type="button"
        className={"attach-btn sticker-btn" + (open ? " active" : "")}
        title={t("open")}
        aria-label={t("open")}
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen((v) => !v)}
      >
        <Icon name="sticker" size={17} />
      </button>
      {open && (
        <div className="sticker-pop" role="dialog" aria-label={t("open")}>
          <div className="sticker-pop-search">
            <Icon name="search" size={13} />
            <input
              value={query}
              placeholder={t("searchByEmoji")}
              aria-label={t("searchByEmoji")}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="sticker-pop-grid">
            {shown.length === 0 ? (
              <div className="sticker-pop-empty">
                {failed && !loaded ? t("loadFailed") : !loaded ? t("loading") : searching ? t("nothingFound") : tab === RECENT_TAB ? t("noRecent") : t("emptyPack")}
              </div>
            ) : (
              shown.map((sticker) => (
                <button
                  type="button"
                  key={sticker.id}
                  className="sticker-pop-item"
                  title={sticker.emoji}
                  onClick={() => pick(sticker)}
                >
                  <StickerImage
                    id={sticker.id}
                    emoji={sticker.emoji}
                    size={72}
                    width={sticker.width}
                    height={sticker.height}
                  />
                </button>
              ))
            )}
          </div>
          <div className="sticker-pop-tabs" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={!searching && tab === RECENT_TAB}
              className={"sticker-pop-tab" + (!searching && tab === RECENT_TAB ? " on" : "")}
              title={t("recent")}
              onClick={() => {
                setQuery("");
                setTab(RECENT_TAB);
              }}
            >
              <Icon name="refresh" size={14} />
            </button>
            {packs.map((pack) => (
              <button
                type="button"
                role="tab"
                key={pack.id}
                aria-selected={!searching && tab === pack.id}
                className={"sticker-pop-tab" + (!searching && tab === pack.id ? " on" : "")}
                title={pack.name}
                onClick={() => {
                  setQuery("");
                  setTab(pack.id);
                }}
              >
                {pack.stickers[0] ? (
                  <StickerImage id={pack.stickers[0].id} emoji={pack.stickers[0].emoji} size={22} />
                ) : (
                  <span className="sticker-pop-tab-letter">{pack.name.slice(0, 1)}</span>
                )}
              </button>
            ))}
            <button
              type="button"
              className="sticker-pop-tab sticker-pop-manage"
              title={t("manage")}
              onClick={() => {
                setOpen(false);
                setManaging(true);
              }}
            >
              <Icon name="plus" size={14} />
            </button>
          </div>
        </div>
      )}
      {managing && <StickerPacksModal onClose={() => setManaging(false)} />}
    </div>
  );
};

export default StickerPicker;
